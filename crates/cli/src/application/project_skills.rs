//! Project-scoped standalone-skill lifecycle composition.
//!
//! A project skill has one complete canonical tree and zero or more target
//! links. This module owns the cross-target planning rules; the execution
//! ports remain in `execution.rs` so all mutations share the normal lock and
//! revalidation protocol.

use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest, Sha256};
use skilltap_core::{
    domain::{
        AbsolutePath, CompatibilityClass, ComponentGraph, DesiredOrigin, DesiredResource,
        Fingerprint, FingerprintAlgorithm, GitCommit, HarnessId, HarnessSet, NativeId,
        OperationAction, OperationDependency, OperationId, Ownership, Provenance, ResourceId,
        ResourceKey, ResourceKind, Scope, Source, UpdateIntent,
    },
    project_skill::{TargetProjectSkillProjection, project_skill_projection},
    runtime::{
        ConfinedFileSystem, DirectoryIdentity, DirectoryTreeFileSystem, ExternalTreeLimits,
        RuntimeError, SystemFileSystem,
    },
    skill::ValidatedSkillTree,
    skill_compatibility::{AgentSkillName, SkillLoadability, validate_agent_skill},
    storage::{
        ArtifactTree, DocumentState, InventoryDocument, ResourceState, StateDocument,
        StateRepository, TargetResourceState, Timestamp,
    },
};

use super::{
    ErrorDetail, NextAction, Outcome, ResultClass, StatusApplication, Warning,
    execution::{
        ManagedSkillAction, ManagedSkillEntry, ManagedSkillPort, ProjectSkillLifecyclePort,
        ProjectSkillLinkAction, ProjectSkillLinkEntry, ProjectSkillLinkPort, StateExecutionJournal,
    },
    scope_label, skill_install_can_complete, stable_hash,
    status::{StatusScope, StatusTargets},
};

/// Execute a project-only skill install, update, or reconciliation. Global
/// behavior remains in the established lifecycle path.
pub(super) fn execute_project_skill_install(
    application: &StatusApplication<'_>,
    command: &'static str,
    scope: &StatusScope,
    targets: &StatusTargets,
    acknowledged: bool,
    name: NativeId,
    skill: ValidatedSkillTree,
    source: Source,
    update_intent: UpdateIntent,
    git_commit: Option<GitCommit>,
    paths: skilltap_core::runtime::PlatformPaths,
    mut outcome: Outcome,
) -> Outcome {
    let skill_name = match AgentSkillName::new(name.as_str()) {
        Ok(name) => name,
        Err(_) => {
            outcome.result = ResultClass::Invalid;
            return outcome.with_error(ErrorDetail::new(
                "skill_name_invalid",
                "Project skill names must use the lowercase Agent Skills name grammar.",
            ));
        }
    };
    let validation = validate_agent_skill(&skill, &skill_name);
    if !validation.loadable_shape() {
        outcome.result = ResultClass::Invalid;
        return outcome.with_error(ErrorDetail::new(
            "skill_format_invalid",
            "The project skill has malformed or incomplete Agent Skills metadata; no canonical tree or link was changed.",
        ));
    }

    let mut partial = false;
    for target in targets.iter() {
        let Some(adapter) = application.registry.adapter(target) else {
            outcome.result = ResultClass::Invalid;
            return outcome.with_error(ErrorDetail::new(
                "target_not_registered",
                "The selected project skill target is not registered in this build.",
            ));
        };
        let Some(projection) = adapter.skill_projection() else {
            outcome.result = ResultClass::AttentionRequired;
            return outcome
                .with_warning(
                    Warning::new(
                        "skill_target_incompatible",
                        "The selected harness does not expose a documented project skill root.",
                    )
                    .with_context("target", target.as_str()),
                )
                .with_next_action(NextAction::new(
                    "inspect_target_contract",
                    "Use a target with a verified project skill load path, then retry.",
                ));
        };
        let Some(project) = scope.resolved.iter().find_map(|scope| match scope {
            Scope::Project(project) => Some(project),
            Scope::Global => None,
        }) else {
            return outcome;
        };
        let Some(native_root) = projection.destination(&paths, &Scope::Project(project.clone()))
        else {
            outcome.result = ResultClass::AttentionRequired;
            return outcome.with_warning(
                Warning::new(
                    "skill_target_incompatible",
                    "The selected harness does not expose a project skill destination.",
                )
                .with_context("target", target.as_str()),
            );
        };
        let compatibility = projection.compatibility(target, &skill, &validation);
        match (compatibility.class(), compatibility.loadability()) {
            (CompatibilityClass::Compatible, SkillLoadability::Loadable) => {}
            (CompatibilityClass::Unknown, SkillLoadability::Unknown) => {
                partial = true;
                outcome = outcome.with_warning(
                    Warning::new(
                        "skill_frontmatter_warning",
                        "The project skill is loadable evidence but is not strictly conforming for the selected target.",
                    )
                    .with_context("target", target.as_str()),
                );
            }
            _ => {
                outcome.result = ResultClass::AttentionRequired;
                return outcome
                    .with_warning(
                        Warning::new(
                            "skill_target_incompatible",
                            "The project skill metadata is not loadable by the selected target.",
                        )
                        .with_context("target", target.as_str()),
                    )
                    .with_next_action(NextAction::new(
                        "repair_project_skill",
                        "Repair the skill metadata or choose a target with verified loadability evidence.",
                    ));
            }
        }
        let _ = native_root;
    }
    if partial && !acknowledged {
        outcome.result = ResultClass::AttentionRequired;
        return outcome
            .with_warning(Warning::new(
                "partial_operation_requires_acknowledgment",
                "The project skill is not strictly conforming for every selected target; rerun with `--yes` to acknowledge the disclosed compatibility consequence.",
            ))
            .with_next_action(NextAction::new(
                "accept_partial",
                "Review the project skill compatibility warning, then retry with `--yes` if acceptable.",
            ))
            .with_summary("operations", 0_u64)
            .with_summary("changed", false);
    }

    let filesystem = SystemFileSystem;
    let limits =
        ExternalTreeLimits::new(64, 100_000, 64 * 1024 * 1024, 1024 * 1024 * 1024, 64 * 1024)
            .expect("bounded project skill limits are valid");
    let mut inventory = application
        .inventory
        .load()
        .ok()
        .and_then(|document| match document {
            DocumentState::Present(document) => Some(document),
            DocumentState::Missing => None,
        })
        .unwrap_or_else(|| {
            InventoryDocument::new(skilltap_core::storage::INVENTORY_SCHEMA_VERSION, [], [])
                .expect("empty inventory is valid")
        });
    let state_document = application
        .state
        .load()
        .ok()
        .and_then(|document| match document {
            DocumentState::Present(document) => Some(document),
            DocumentState::Missing => None,
        });
    let mut operations = Vec::new();
    let mut canonical_entries = BTreeMap::new();
    let mut link_entries = BTreeMap::new();
    let mut seeds = BTreeMap::new();
    let mut any_blocked = false;

    for concrete_scope in &scope.resolved {
        let Scope::Project(project) = concrete_scope else {
            continue;
        };
        let key = match ResourceId::new(format!("skill:{}", name.as_str())) {
            Ok(id) => ResourceKey::new(id, concrete_scope.clone()),
            Err(_) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "skill_resource_invalid",
                    "The project skill resource could not be represented safely.",
                ));
            }
        };
        let existing_resource = inventory.resources().get(&key);
        if let Some(existing) = existing_resource
            && existing
                .source()
                .is_some_and(|existing| existing != &source)
            && command != "skill update"
        {
            outcome.result = ResultClass::AttentionRequired;
            return outcome.with_warning(
                Warning::new(
                    "skill_update_required",
                    "The project canonical tree already belongs to another source; use `skill update` to replace it explicitly.",
                )
                .with_context("scope", scope_label(concrete_scope)),
            );
        }
        let mut desired_targets = existing_resource
            .map(|resource| resource.targets().iter().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        desired_targets.extend(targets.iter().cloned());
        let desired_targets = match HarnessSet::new(desired_targets) {
            Ok(targets) => targets,
            Err(_) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "skill_targets_invalid",
                    "The project skill target set could not be represented safely.",
                ));
            }
        };
        let desired = match DesiredResource::new(
            key.clone(),
            ResourceKind::StandaloneSkill,
            desired_targets.clone(),
            DesiredOrigin::Direct,
            Some(source.clone()),
            update_intent,
            ComponentGraph::new([]).expect("empty skill component graph is valid"),
            BTreeMap::new(),
            BTreeMap::new(),
            BTreeSet::new(),
        ) {
            Ok(resource) => resource,
            Err(_) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "skill_resource_invalid",
                    "The desired project skill could not be represented safely.",
                ));
            }
        };

        let canonical_root = AbsolutePath::new(format!("{}/.agents", project.as_str()))
            .expect("project canonical root is valid");
        let canonical_destination =
            skilltap_core::domain::RelativeArtifactPath::new(format!("skills/{}", name.as_str()))
                .expect("Agent Skill names are safe path components");
        let current = match load_tree(&filesystem, &canonical_root, &canonical_destination, limits)
        {
            Ok(current) => current,
            Err(_) => {
                outcome.result = ResultClass::AttentionRequired;
                return outcome.with_warning(
                    Warning::new(
                        "skill_canonical_unavailable",
                        "The canonical project skill could not be observed safely; no target link was planned.",
                    )
                    .with_context("scope", scope_label(concrete_scope)),
                );
            }
        };
        let current_fingerprint = current
            .as_ref()
            .map(|current| fingerprint_tree(&current.tree));
        let mut canonical_operation = None;
        if current_fingerprint.as_ref() != Some(skill.fingerprint()) {
            if current.is_some() {
                let recorded = state_document
                    .as_ref()
                    .and_then(|state| state.resources().get(&key))
                    .and_then(|state| {
                        state
                            .targets()
                            .values()
                            .find_map(|target| target.fingerprint())
                    });
                if recorded != current_fingerprint.as_ref() {
                    outcome.result = ResultClass::AttentionRequired;
                    return outcome.with_warning(
                        Warning::new(
                            "skill_canonical_drifted",
                            "The canonical project skill differs from skilltap's recorded fingerprint; no replacement was made.",
                        )
                        .with_context("scope", scope_label(concrete_scope)),
                    );
                }
                if command != "skill update" {
                    outcome.result = ResultClass::AttentionRequired;
                    return outcome.with_warning(
                        Warning::new(
                            "skill_update_required",
                            "The project source changed while the canonical tree is intact; use `skill update` to replace it explicitly.",
                        )
                        .with_context("scope", scope_label(concrete_scope)),
                    );
                }
                if !desired_targets
                    .iter()
                    .all(|target| targets.resolved.contains(target))
                {
                    outcome.result = ResultClass::AttentionRequired;
                    return outcome
                        .with_warning(
                            Warning::new(
                                "project_skill_shared_content_requires_all_targets",
                                "Updating one canonical project skill would change an unselected target; select every desired target before retrying.",
                            )
                            .with_context("scope", scope_label(concrete_scope)),
                        )
                        .with_next_action(NextAction::new(
                            "select_all_project_skill_targets",
                            "Retry with `--target all` or explicitly select every desired target.",
                        ));
                }
            }
            let operation_id = project_canonical_operation_id(&key);
            let path = AbsolutePath::new(format!(
                "{}/{}",
                canonical_root.as_str(),
                canonical_destination.as_str()
            ))
            .expect("canonical skill path is valid");
            let target = desired_targets
                .iter()
                .next()
                .cloned()
                .expect("non-empty targets");
            let operation = match skilltap_core::lifecycle_operation::faithful_file_operation(
                operation_id.clone(),
                target,
                key.clone(),
                OperationAction::SkillInstall,
                path,
            ) {
                Ok(operation) => operation,
                Err(_) => {
                    outcome.result = ResultClass::Invalid;
                    return outcome.with_error(ErrorDetail::new(
                        "operation_contract_invalid",
                        "The canonical project skill operation could not be represented safely.",
                    ));
                }
            };
            operations.push(operation);
            canonical_entries.insert(
                operation_id.clone(),
                ManagedSkillEntry {
                    root: canonical_root.clone(),
                    destination: canonical_destination.clone(),
                    tree: skill.tree().clone(),
                    backup_tree: current.as_ref().map(|current| current.tree.clone()),
                    action: if current.is_some() {
                        ManagedSkillAction::Replace
                    } else {
                        ManagedSkillAction::Install
                    },
                    expected_identity: current.as_ref().map(|current| current.identity),
                    owner: Some(key.clone()),
                    config_root: Some(paths.skilltap_config().clone()),
                },
            );
            canonical_operation = Some(operation_id);
        }

        let mut seen_destinations = BTreeSet::new();
        for target in targets.iter() {
            let Some(adapter) = application.registry.adapter(target) else {
                continue;
            };
            let Some(projection_port) = adapter.skill_projection() else {
                any_blocked = true;
                continue;
            };
            let Some(native_root) = projection_port.destination(&paths, concrete_scope) else {
                any_blocked = true;
                continue;
            };
            let projection = match project_skill_projection(project, &native_root, &skill_name) {
                Ok(projection) => projection,
                Err(_) => {
                    any_blocked = true;
                    outcome = outcome.with_warning(
                        Warning::new(
                            "skill_destination_invalid",
                            "The target project skill root is outside the project or cannot be represented safely.",
                        )
                        .with_context("target", target.as_str()),
                    );
                    continue;
                }
            };
            let state_target = state_document
                .as_ref()
                .and_then(|state| state.resources().get(&key))
                .and_then(|state| state.target(target));
            let Some(spec) = (match projection {
                TargetProjectSkillProjection::Canonical { .. } => None,
                TargetProjectSkillProjection::RelativeLink(spec) => Some(spec),
            }) else {
                continue;
            };
            let destination_path = spec.destination_path();
            if !seen_destinations.insert(destination_path.clone()) {
                continue;
            }
            let observation = match filesystem
                .inspect_entry_beneath_no_follow(&spec.project_root, &spec.destination)
            {
                Ok(observation) => observation,
                Err(_) => {
                    any_blocked = true;
                    outcome = outcome.with_warning(
                        Warning::new(
                            "skill_destination_unavailable",
                            "The target project skill destination could not be observed safely.",
                        )
                        .with_context("target", target.as_str()),
                    );
                    continue;
                }
            };
            let action = match observation {
                skilltap_core::runtime::ConfinedEntryObservation::Missing => {
                    Some(ProjectSkillLinkAction::Create)
                }
                skilltap_core::runtime::ConfinedEntryObservation::RelativeSymlink {
                    identity: _,
                    target: observed_target,
                } if observed_target == spec.target => None,
                skilltap_core::runtime::ConfinedEntryObservation::RelativeSymlink {
                    identity,
                    target: observed_target,
                } if state_target.is_some_and(|state| state.ownership() == Ownership::Skilltap) => {
                    Some(ProjectSkillLinkAction::Replace {
                        expected_identity: identity,
                        previous_target: observed_target,
                    })
                }
                _ => {
                    any_blocked = true;
                    outcome = outcome.with_warning(
                        Warning::new(
                            "skill_destination_unmanaged",
                            "The target destination is unmanaged or divergent; it was preserved and not overwritten.",
                        )
                        .with_context("target", target.as_str()),
                    );
                    None
                }
            };
            let Some(action) = action else {
                continue;
            };
            let operation_id = project_link_operation_id(target, &key, &destination_path);
            let dependencies = canonical_operation
                .iter()
                .cloned()
                .map(OperationDependency::new)
                .collect::<Vec<_>>();
            let operation =
                match skilltap_core::lifecycle_operation::faithful_file_operation_with_dependencies(
                    operation_id.clone(),
                    target.clone(),
                    key.clone(),
                    OperationAction::SkillInstall,
                    destination_path,
                    dependencies,
                ) {
                    Ok(operation) => operation,
                    Err(_) => {
                        outcome.result = ResultClass::Invalid;
                        return outcome.with_error(ErrorDetail::new(
                            "operation_contract_invalid",
                            "The project skill link operation could not be represented safely.",
                        ));
                    }
                };
            operations.push(operation);
            link_entries.insert(
                operation_id,
                ProjectSkillLinkEntry {
                    root: spec.project_root,
                    destination: spec.destination,
                    target: spec.target,
                    action,
                },
            );
        }
        if any_blocked {
            continue;
        }

        let mut bindings = Vec::new();
        for target in desired_targets.iter() {
            if let Some(existing) = state_document
                .as_ref()
                .and_then(|state| state.resources().get(&key))
                .and_then(|state| state.target(target))
                && !targets.resolved.contains(target)
            {
                bindings.push(existing.clone());
                continue;
            }
            let observed_at = match Timestamp::from_system_time(std::time::SystemTime::now()) {
                Ok(value) => value,
                Err(_) => {
                    outcome.result = ResultClass::Invalid;
                    return outcome.with_error(ErrorDetail::new(
                        "clock_unavailable",
                        "The project skill state timestamp could not be recorded safely.",
                    ));
                }
            };
            bindings.push(
                TargetResourceState::new(
                    target.clone(),
                    Some(name.clone()),
                    Provenance::Direct,
                    Ownership::Skilltap,
                    Some(source.clone()),
                    None,
                    Some(skill.fingerprint().clone()),
                    git_commit
                        .clone()
                        .map(skilltap_core::domain::ResolvedRevision::GitCommit),
                    None,
                    observed_at,
                    None,
                )
                .expect("project skill target seed is valid"),
            );
        }
        seeds.insert(
            key.clone(),
            ResourceState::new(key, bindings).expect("project skill state seed is valid"),
        );
        inventory = match inventory.with_resource(desired) {
            Ok(inventory) => inventory,
            Err(_) => {
                outcome.result = ResultClass::AttentionRequired;
                return outcome.with_error(ErrorDetail::new(
                    "inventory_resource_conflict",
                    "The requested project skill conflicts with an existing desired definition.",
                ));
            }
        };
    }

    if any_blocked {
        outcome.result = ResultClass::AttentionRequired;
        return outcome
            .with_summary("operations", 0_u64)
            .with_summary("changed", false);
    }
    if application.inventory.replace(&inventory).is_err() {
        outcome.result = ResultClass::Invalid;
        return outcome.with_error(ErrorDetail::new(
            "inventory_publish_failed",
            "The desired project skill inventory could not be published safely.",
        ));
    }
    if operations.is_empty() {
        if refresh_state_seeds(application.state, &seeds).is_err() {
            outcome.result = ResultClass::Invalid;
            return outcome.with_error(ErrorDetail::new(
                "state_seed_publish_failed",
                "The project skill state could not be recorded safely.",
            ));
        }
        if skill_install_can_complete(&outcome, acknowledged) {
            outcome.result = ResultClass::Completed;
        }
        return outcome
            .with_summary("operations", 0_u64)
            .with_summary("changed", false);
    }
    let plan = match skilltap_core::domain::Plan::new(operations) {
        Ok(plan) => plan,
        Err(_) => {
            outcome.result = ResultClass::Invalid;
            return outcome.with_error(ErrorDetail::new(
                "operation_plan_invalid",
                "The project skill operation plan was invalid.",
            ));
        }
    };
    let port = ProjectSkillLifecyclePort {
        canonical: ManagedSkillPort {
            filesystem: &filesystem,
            entries: canonical_entries,
        },
        links: ProjectSkillLinkPort {
            filesystem: &filesystem,
            entries: link_entries,
            foreign_operations: BTreeSet::new(),
        },
    };
    let journal = StateExecutionJournal {
        plan: &plan,
        state: application.state,
        seeds,
    };
    let lock_path = match AbsolutePath::new(format!(
        "{}/skilltap.lock",
        paths.skilltap_config().as_str()
    )) {
        Ok(path) => path,
        Err(_) => {
            outcome.result = ResultClass::Invalid;
            return outcome.with_error(ErrorDetail::new(
                "lock_path_invalid",
                "The skilltap configuration lock path is invalid.",
            ));
        }
    };
    let report = match skilltap_core::executor::execute_plan(
        &skilltap_core::runtime::SystemConfigurationLock,
        &lock_path,
        &port,
        &journal,
        &plan,
    ) {
        Ok(report) => report,
        Err(error) => {
            outcome.result = ResultClass::AttentionRequired;
            return outcome.with_error(super::native_execution_error(&error));
        }
    };
    for result in report.result.operations().values() {
        outcome = outcome.with_operation(crate::OperationOutcome::new(
            result.operation_id().to_string(),
            super::operation_result_status(result.outcome()),
        ));
        if !matches!(
            result.outcome(),
            skilltap_core::domain::OperationOutcome::Applied
                | skilltap_core::domain::OperationOutcome::NoChange
        ) {
            outcome.result = ResultClass::AttentionRequired;
        }
    }
    if report.changed && skill_install_can_complete(&outcome, acknowledged) {
        outcome.result = ResultClass::Completed;
    }
    outcome
        .with_summary("operations", report.result.operations().len() as u64)
        .with_summary("changed", report.changed)
}

#[derive(Clone)]
struct LoadedTree {
    identity: DirectoryIdentity,
    tree: ArtifactTree,
}

fn load_tree(
    filesystem: &SystemFileSystem,
    root: &AbsolutePath,
    destination: &skilltap_core::domain::RelativeArtifactPath,
    _limits: ExternalTreeLimits,
) -> Result<Option<LoadedTree>, ()> {
    match filesystem.load_tree_no_follow(root, destination) {
        Ok((identity, files)) => Ok(Some(LoadedTree {
            identity,
            tree: ArtifactTree::new(
                files
                    .into_iter()
                    .map(|(path, file)| (path.as_str().to_owned(), file)),
            )
            .map_err(|_| ())?,
        })),
        Err(RuntimeError::FileSystem { source, .. })
            if source.kind() == std::io::ErrorKind::NotFound =>
        {
            Ok(None)
        }
        Err(_) => Err(()),
    }
}

fn fingerprint_tree(tree: &ArtifactTree) -> Fingerprint {
    let mut digest = Sha256::new();
    for (path, file) in tree.files() {
        let path = path.as_str().as_bytes();
        digest.update((path.len() as u64).to_be_bytes());
        digest.update(path);
        digest.update([u8::from(file.is_executable())]);
        digest.update((file.contents().len() as u64).to_be_bytes());
        digest.update(file.contents());
    }
    let hex = digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Fingerprint::new(FingerprintAlgorithm::Sha256, hex).expect("SHA-256 fingerprint is valid")
}

fn project_canonical_operation_id(resource: &ResourceKey) -> OperationId {
    let hash = stable_hash(&format!("project-skill-canonical:{resource}"));
    OperationId::new(format!("project-skill-canonical:{hash:016x}"))
        .expect("project canonical operation id is valid")
}

fn project_link_operation_id(
    target: &HarnessId,
    resource: &ResourceKey,
    path: &AbsolutePath,
) -> OperationId {
    let hash = stable_hash(&format!("project-skill-link:{target}:{resource}:{path}"));
    OperationId::new(format!("project-skill-link:{target}:{hash:016x}"))
        .expect("project link operation id is valid")
}

fn refresh_state_seeds(
    repository: &dyn StateRepository,
    seeds: &BTreeMap<ResourceKey, ResourceState>,
) -> Result<(), ()> {
    if seeds.is_empty() {
        return Ok(());
    }
    let mut document = match repository.load().map_err(|_| ())? {
        DocumentState::Present(document) => document,
        DocumentState::Missing => StateDocument::new(
            skilltap_core::storage::STATE_SCHEMA_VERSION,
            [],
            [],
            None,
            None,
            None,
        )
        .map_err(|_| ())?,
    };
    for seed in seeds.values() {
        document = document
            .refresh_resource_state(seed.clone())
            .map_err(|_| ())?;
    }
    repository.replace(&document).map_err(|_| ())
}

/// Remove selected project projections first, then remove the canonical tree
/// only when the final desired target is gone and direct ownership proves that
/// skilltap may delete it.
pub(super) fn execute_project_skill_remove(
    application: &StatusApplication<'_>,
    scope: &StatusScope,
    targets: &StatusTargets,
    name: NativeId,
    acknowledged: bool,
    paths: skilltap_core::runtime::PlatformPaths,
    mut outcome: Outcome,
) -> Outcome {
    let skill_name = match AgentSkillName::new(name.as_str()) {
        Ok(name) => name,
        Err(_) => {
            outcome.result = ResultClass::Invalid;
            return outcome.with_error(ErrorDetail::new(
                "skill_name_invalid",
                "Project skill names must use the lowercase Agent Skills name grammar.",
            ));
        }
    };
    let filesystem = SystemFileSystem;
    let limits =
        ExternalTreeLimits::new(64, 100_000, 64 * 1024 * 1024, 1024 * 1024 * 1024, 64 * 1024)
            .expect("bounded project skill limits are valid");
    let inventory = application
        .inventory
        .load()
        .ok()
        .and_then(|document| match document {
            DocumentState::Present(document) => Some(document),
            DocumentState::Missing => None,
        });
    let Some(inventory) = inventory else {
        outcome.result = ResultClass::AttentionRequired;
        return outcome.with_warning(Warning::new(
            "skill_not_managed",
            "The project skill is not in desired inventory; no files were removed.",
        ));
    };
    let state_document = application
        .state
        .load()
        .ok()
        .and_then(|document| match document {
            DocumentState::Present(document) => Some(document),
            DocumentState::Missing => None,
        });
    let mut operations = Vec::new();
    let mut canonical_entries = BTreeMap::new();
    let mut link_entries = BTreeMap::new();
    let mut target_projection_keys = BTreeSet::new();
    let mut blocked = false;

    for concrete_scope in &scope.resolved {
        let Scope::Project(project) = concrete_scope else {
            continue;
        };
        let Some(key) = ResourceId::new(format!("skill:{}", name.as_str()))
            .ok()
            .map(|id| ResourceKey::new(id, concrete_scope.clone()))
        else {
            outcome.result = ResultClass::Invalid;
            return outcome.with_error(ErrorDetail::new(
                "skill_resource_invalid",
                "The project skill resource could not be represented safely.",
            ));
        };
        let Some(resource) = inventory.resources().get(&key) else {
            outcome = outcome.with_warning(
                Warning::new(
                    "skill_not_managed",
                    "The project skill has no desired inventory entry; no files were removed.",
                )
                .with_context("scope", scope_label(concrete_scope)),
            );
            blocked = true;
            continue;
        };
        let Some(state) = state_document
            .as_ref()
            .and_then(|state| state.resources().get(&key))
        else {
            outcome = outcome.with_warning(
                Warning::new(
                    "skill_not_owned",
                    "The project skill has no ownership record; no files were removed.",
                )
                .with_context("scope", scope_label(concrete_scope)),
            );
            blocked = true;
            continue;
        };
        let canonical_root = AbsolutePath::new(format!("{}/.agents", project.as_str()))
            .expect("project canonical root is valid");
        let canonical_destination =
            skilltap_core::domain::RelativeArtifactPath::new(format!("skills/{}", name.as_str()))
                .expect("Agent Skill names are safe path components");
        let canonical = match load_tree(
            &filesystem,
            &canonical_root,
            &canonical_destination,
            limits,
        ) {
            Ok(canonical) => canonical,
            Err(_) => {
                outcome = outcome.with_warning(
                    Warning::new(
                        "skill_canonical_unavailable",
                        "The canonical project skill could not be observed safely; removal is blocked.",
                    )
                    .with_context("scope", scope_label(concrete_scope)),
                );
                blocked = true;
                continue;
            }
        };
        let mut link_ids = Vec::new();
        let mut seen_destinations = BTreeSet::new();
        let mut selected_safe = true;
        for target in targets.iter() {
            let Some(target_state) = state.target(target) else {
                selected_safe = false;
                outcome = outcome.with_warning(
                    Warning::new(
                        "skill_not_managed_for_target",
                        "The selected project skill target has no ownership record; no files were removed.",
                    )
                    .with_context("target", target.as_str()),
                );
                continue;
            };
            if target_state.ownership() != Ownership::Skilltap {
                selected_safe = false;
                outcome = outcome.with_warning(
                    Warning::new(
                        "skill_not_owned",
                        "The selected project skill target is not owned by skilltap; no files were removed.",
                    )
                    .with_context("target", target.as_str()),
                );
                continue;
            }
            let Some(adapter) = application.registry.adapter(target) else {
                selected_safe = false;
                continue;
            };
            let Some(projection_port) = adapter.skill_projection() else {
                selected_safe = false;
                continue;
            };
            let Some(native_root) = projection_port.destination(&paths, concrete_scope) else {
                selected_safe = false;
                continue;
            };
            let projection = match project_skill_projection(project, &native_root, &skill_name) {
                Ok(projection) => projection,
                Err(_) => {
                    selected_safe = false;
                    outcome = outcome.with_warning(
                        Warning::new(
                            "skill_destination_invalid",
                            "The selected project skill destination cannot be represented safely.",
                        )
                        .with_context("target", target.as_str()),
                    );
                    continue;
                }
            };
            let TargetProjectSkillProjection::RelativeLink(spec) = projection else {
                continue;
            };
            let destination_path = spec.destination_path();
            if !seen_destinations.insert(destination_path.clone()) {
                continue;
            }
            let observation = match filesystem
                .inspect_entry_beneath_no_follow(&spec.project_root, &spec.destination)
            {
                Ok(observation) => observation,
                Err(_) => {
                    selected_safe = false;
                    continue;
                }
            };
            let skilltap_core::runtime::ConfinedEntryObservation::RelativeSymlink {
                identity,
                target: observed_target,
            } = observation
            else {
                if !matches!(
                    observation,
                    skilltap_core::runtime::ConfinedEntryObservation::Missing
                ) {
                    selected_safe = false;
                    outcome = outcome.with_warning(
                        Warning::new(
                            "skill_destination_unmanaged",
                            "The selected project skill destination is not the owned relative link; it was preserved.",
                        )
                        .with_context("target", target.as_str()),
                    );
                }
                continue;
            };
            if observed_target != spec.target {
                selected_safe = false;
                outcome = outcome.with_warning(
                    Warning::new(
                        "skill_destination_divergent",
                        "The selected project skill link target diverged; it was preserved.",
                    )
                    .with_context("target", target.as_str()),
                );
                continue;
            }
            let operation_id = project_link_operation_id(target, &key, &destination_path);
            let operation = match skilltap_core::lifecycle_operation::faithful_file_operation(
                operation_id.clone(),
                target.clone(),
                key.clone(),
                OperationAction::SkillRemove,
                destination_path,
            ) {
                Ok(operation) => operation,
                Err(_) => {
                    outcome.result = ResultClass::Invalid;
                    return outcome.with_error(ErrorDetail::new(
                        "operation_contract_invalid",
                        "The project skill link removal could not be represented safely.",
                    ));
                }
            };
            operations.push(operation);
            link_ids.push(operation_id.clone());
            link_entries.insert(
                operation_id,
                ProjectSkillLinkEntry {
                    root: spec.project_root,
                    destination: spec.destination,
                    target: spec.target,
                    action: ProjectSkillLinkAction::Remove {
                        expected_identity: identity,
                    },
                },
            );
        }
        if !selected_safe {
            blocked = true;
            continue;
        }
        let remaining = resource
            .targets()
            .iter()
            .any(|target| !targets.resolved.contains(target));
        let canonical_owned = state.targets().values().any(|target| {
            target.provenance() == Provenance::Direct && target.ownership() == Ownership::Skilltap
        });
        if !remaining && canonical_owned {
            if let Some(canonical) = canonical {
                let operation_id = project_canonical_remove_operation_id(&key);
                let target = resource
                    .targets()
                    .iter()
                    .next()
                    .cloned()
                    .expect("non-empty targets");
                let dependencies = link_ids.iter().cloned().map(OperationDependency::new);
                let path = AbsolutePath::new(format!(
                    "{}/{}",
                    canonical_root.as_str(),
                    canonical_destination.as_str()
                ))
                .expect("canonical skill path is valid");
                let operation = match skilltap_core::lifecycle_operation::faithful_file_operation_with_dependencies(
                    operation_id.clone(),
                    target,
                    key.clone(),
                    OperationAction::SkillRemove,
                    path,
                    dependencies,
                ) {
                    Ok(operation) => operation,
                    Err(_) => {
                        outcome.result = ResultClass::Invalid;
                        return outcome.with_error(ErrorDetail::new(
                            "operation_contract_invalid",
                            "The canonical project skill removal could not be represented safely.",
                        ));
                    }
                };
                operations.push(operation);
                canonical_entries.insert(
                    operation_id,
                    ManagedSkillEntry {
                        root: canonical_root,
                        destination: canonical_destination,
                        tree: canonical.tree,
                        backup_tree: None,
                        action: ManagedSkillAction::Remove,
                        expected_identity: Some(canonical.identity),
                        owner: None,
                        config_root: None,
                    },
                );
            }
        }
        target_projection_keys.insert(key);
    }

    if blocked || (!acknowledged && !outcome.warnings.is_empty()) {
        outcome.result = ResultClass::AttentionRequired;
        return outcome
            .with_summary("operations", 0_u64)
            .with_summary("changed", false);
    }
    if operations.is_empty() {
        let next = match super::project_inventory_targets(
            &inventory,
            &target_projection_keys,
            &targets.resolved,
        ) {
            Ok(next) => next,
            Err(_) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "inventory_publish_failed",
                    "The project skill inventory could not be updated safely.",
                ));
            }
        };
        if application.inventory.replace(&next).is_err()
            || super::project_state_targets_after_remove(
                application.state,
                &target_projection_keys,
                &targets.resolved,
            )
            .is_err()
        {
            outcome.result = ResultClass::Invalid;
            return outcome.with_error(ErrorDetail::new(
                "state_publish_failed",
                "The project skill state could not be updated safely.",
            ));
        }
        outcome.result = ResultClass::Completed;
        return outcome
            .with_summary("operations", 0_u64)
            .with_summary("changed", false);
    }
    let plan = match skilltap_core::domain::Plan::new(operations) {
        Ok(plan) => plan,
        Err(_) => {
            outcome.result = ResultClass::Invalid;
            return outcome.with_error(ErrorDetail::new(
                "operation_plan_invalid",
                "The project skill removal plan was invalid.",
            ));
        }
    };
    let port = ProjectSkillLifecyclePort {
        canonical: ManagedSkillPort {
            filesystem: &filesystem,
            entries: canonical_entries,
        },
        links: ProjectSkillLinkPort {
            filesystem: &filesystem,
            entries: link_entries,
            foreign_operations: BTreeSet::new(),
        },
    };
    let journal = StateExecutionJournal {
        plan: &plan,
        state: application.state,
        seeds: BTreeMap::new(),
    };
    let lock_path = match AbsolutePath::new(format!(
        "{}/skilltap.lock",
        paths.skilltap_config().as_str()
    )) {
        Ok(path) => path,
        Err(_) => {
            outcome.result = ResultClass::Invalid;
            return outcome.with_error(ErrorDetail::new(
                "lock_path_invalid",
                "The skilltap configuration lock path is invalid.",
            ));
        }
    };
    let report = match skilltap_core::executor::execute_plan(
        &skilltap_core::runtime::SystemConfigurationLock,
        &lock_path,
        &port,
        &journal,
        &plan,
    ) {
        Ok(report) => report,
        Err(error) => {
            outcome.result = ResultClass::AttentionRequired;
            return outcome.with_error(super::native_execution_error(&error));
        }
    };
    let successful = report.result.operations().values().all(|result| {
        matches!(
            result.outcome(),
            skilltap_core::domain::OperationOutcome::Applied
                | skilltap_core::domain::OperationOutcome::NoChange
        )
    });
    for result in report.result.operations().values() {
        outcome = outcome.with_operation(crate::OperationOutcome::new(
            result.operation_id().to_string(),
            super::operation_result_status(result.outcome()),
        ));
        if !matches!(
            result.outcome(),
            skilltap_core::domain::OperationOutcome::Applied
                | skilltap_core::domain::OperationOutcome::NoChange
        ) {
            outcome.result = ResultClass::AttentionRequired;
        }
    }
    if successful {
        let next = match super::project_inventory_targets(
            &inventory,
            &target_projection_keys,
            &targets.resolved,
        ) {
            Ok(next) => next,
            Err(_) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "inventory_publish_failed",
                    "The project skill inventory could not be updated safely.",
                ));
            }
        };
        if application.inventory.replace(&next).is_err()
            || super::project_state_targets_after_remove(
                application.state,
                &target_projection_keys,
                &targets.resolved,
            )
            .is_err()
        {
            outcome.result = ResultClass::Invalid;
            return outcome.with_error(ErrorDetail::new(
                "state_publish_failed",
                "The project skill state could not be updated safely.",
            ));
        }
    }
    if report.changed && outcome.errors.is_empty() && outcome.warnings.is_empty() {
        outcome.result = ResultClass::Completed;
    }
    outcome
        .with_summary("operations", report.result.operations().len() as u64)
        .with_summary("changed", report.changed)
}

fn project_canonical_remove_operation_id(resource: &ResourceKey) -> OperationId {
    let hash = stable_hash(&format!("project-skill-remove-canonical:{resource}"));
    OperationId::new(format!("project-skill-remove-canonical:{hash:016x}"))
        .expect("project canonical removal operation id is valid")
}
