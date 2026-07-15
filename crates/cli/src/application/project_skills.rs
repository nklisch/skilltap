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
        AbsolutePath, CapabilityId, CompatibilityClass, ComponentGraph, DesiredOrigin,
        DesiredResource, Fingerprint, FingerprintAlgorithm, GitCommit, HarnessId, HarnessSet,
        CapabilitySupport, NativeId, OperationAction, OperationDependency, OperationId, Ownership, Provenance,
        ResourceId, ResourceKey, ResourceKind, Scope, Source, UpdateIntent,
    },
    project_skill::{
        ProjectSkillLinkHealth, TargetProjectSkillProjection, project_skill_projection,
    },
    runtime::{
        ConfigurationLockGuard, ConfinedEntryObservation, ConfinedFileSystem, DirectoryIdentity,
        DirectoryTreeFileSystem, ExternalTreeLimits, JsonLimits, ProcessLimits, RuntimeError,
        SystemFileSystem,
    },
    skill::{SkillTreeError, ValidatedSkillTree},
    skill_compatibility::{
        AgentSkillName, AgentSkillValidation, SkillCompatibility, SkillLoadability,
        validate_agent_skill,
    },
    storage::{
        ArtifactTree, ConfigDocument, DocumentState, InventoryDocument, ResourceState,
        StateDocument, StateRepository, TargetResourceState, Timestamp,
    },
};

fn project_skill_operation(
    operation_id: OperationId,
    target: HarnessId,
    resource: ResourceKey,
    path: AbsolutePath,
    dependencies: Vec<OperationDependency>,
    partial: bool,
) -> Result<skilltap_core::domain::Operation, skilltap_core::domain::OperationContractError> {
    if !partial {
        return skilltap_core::lifecycle_operation::faithful_file_operation_with_dependencies(
            operation_id,
            target,
            resource,
            OperationAction::SkillInstall,
            path,
            dependencies,
        );
    }
    let component = skilltap_core::domain::ComponentId::new(resource.id().as_str().to_owned())
        .expect("project skill component id is valid");
    skilltap_core::lifecycle_operation::partial_file_operation_with_dependencies(
        operation_id,
        target.clone(),
        resource,
        OperationAction::SkillInstall,
        path,
        [skilltap_core::domain::CompatibilityEvidence::new(
            skilltap_core::domain::EvidenceCode::new("skill.frontmatter_unverified")
                .expect("static evidence code is valid"),
            target,
            [component.clone()],
            skilltap_core::domain::EvidenceDetail::new(
                "The project skill is loadable, but its frontmatter is not fully strict for this target.",
            )
            .expect("static evidence detail is valid"),
        )],
        [skilltap_core::domain::MaterialConsequence::new(
            skilltap_core::domain::ConsequenceCode::new("skill.frontmatter_unverified")
                .expect("static consequence code is valid"),
            [component],
            skilltap_core::domain::ConsequenceSummary::new(
                "The project skill will be installed while strict frontmatter compatibility remains unverified.",
            )
            .expect("static consequence summary is valid"),
        )],
        dependencies,
    )
}

use super::{
    ErrorDetail, NextAction, Outcome, OutputEntry, ResultClass, StatusApplication, Warning,
    execution::{
        ManagedSkillAction, ManagedSkillEntry, ManagedSkillPort, ProjectSkillLifecyclePort,
        ProjectSkillLinkAction, ProjectSkillLinkEntry, ProjectSkillLinkPort, StateExecutionJournal,
    },
    scope_label, skill_install_can_complete, stable_hash,
    status::{StatusScope, StatusTargets},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ProjectSkillObservation {
    pub(super) resource: ResourceKey,
    pub(super) canonical: CanonicalProjectSkillObservation,
    pub(super) targets: BTreeMap<HarnessId, TargetProjectSkillObservation>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum CanonicalProjectSkillObservation {
    Missing,
    Invalid {
        tree_error: Option<SkillTreeError>,
        format: Option<AgentSkillValidation>,
    },
    Present {
        fingerprint: Fingerprint,
        format: AgentSkillValidation,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct TargetProjectSkillObservation {
    pub(super) compatibility: SkillCompatibility,
    pub(super) projection: ProjectSkillLinkHealth,
    pub(super) ownership: Ownership,
}

impl CanonicalProjectSkillObservation {
    pub(super) fn fingerprint(&self) -> Option<&Fingerprint> {
        match self {
            Self::Present { fingerprint, .. } => Some(fingerprint),
            Self::Missing | Self::Invalid { .. } => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ProjectSkillObservationError {
    InvalidResource,
    UnsupportedFilesystem,
}

fn project_skill_paths(
    project: &AbsolutePath,
    name: &AgentSkillName,
) -> (AbsolutePath, skilltap_core::domain::RelativeArtifactPath) {
    (
        AbsolutePath::new(format!("{}/.agents", project.as_str()))
            .expect("project canonical root is valid"),
        skilltap_core::domain::RelativeArtifactPath::new(format!("skills/{name}"))
            .expect("validated Agent Skill name is a safe path component"),
    )
}

fn canonical_project_skill(
    filesystem: &dyn ConfinedFileSystem,
    root: &AbsolutePath,
    destination: &skilltap_core::domain::RelativeArtifactPath,
    limits: ExternalTreeLimits,
) -> Result<
    (CanonicalProjectSkillObservation, Option<ValidatedSkillTree>),
    ProjectSkillObservationError,
> {
    let (_, files) = match filesystem.load_tree_bounded_no_follow(root, destination, limits) {
        Ok(value) => value,
        Err(RuntimeError::FileSystem { source, .. })
            if source.kind() == std::io::ErrorKind::NotFound =>
        {
            return Ok((CanonicalProjectSkillObservation::Missing, None));
        }
        Err(_) => {
            return Ok((
                CanonicalProjectSkillObservation::Invalid {
                    tree_error: None,
                    format: None,
                },
                None,
            ));
        }
    };
    let tree = match ArtifactTree::new(
        files
            .into_iter()
            .map(|(path, file)| (path.as_str().to_owned(), file)),
    ) {
        Ok(tree) => tree,
        Err(error) => {
            return Ok((
                CanonicalProjectSkillObservation::Invalid {
                    tree_error: Some(SkillTreeError::Artifact(error)),
                    format: None,
                },
                None,
            ));
        }
    };
    let skill = match ValidatedSkillTree::from_artifact_tree(tree) {
        Ok(skill) => skill,
        Err(error) => {
            return Ok((
                CanonicalProjectSkillObservation::Invalid {
                    tree_error: Some(error),
                    format: None,
                },
                None,
            ));
        }
    };
    let name = destination
        .as_str()
        .strip_prefix("skills/")
        .ok_or(ProjectSkillObservationError::InvalidResource)?;
    let directory_name =
        AgentSkillName::new(name).map_err(|_| ProjectSkillObservationError::InvalidResource)?;
    let format = validate_agent_skill(&skill, &directory_name);
    if !format.loadable_shape() {
        return Ok((
            CanonicalProjectSkillObservation::Invalid {
                tree_error: None,
                format: Some(format),
            },
            None,
        ));
    }
    Ok((
        CanonicalProjectSkillObservation::Present {
            fingerprint: skill.fingerprint().clone(),
            format,
        },
        Some(skill),
    ))
}

pub(super) fn observe_project_skill(
    registry: &skilltap_harnesses::TargetRegistry,
    filesystem: &dyn ConfinedFileSystem,
    paths: &skilltap_core::runtime::PlatformPaths,
    resource: &DesiredResource,
    state: Option<&ResourceState>,
    selected_targets: &HarnessSet,
    limits: ExternalTreeLimits,
) -> Result<ProjectSkillObservation, ProjectSkillObservationError> {
    let Scope::Project(project) = resource.scope() else {
        return Err(ProjectSkillObservationError::InvalidResource);
    };
    let name = resource
        .id()
        .as_str()
        .strip_prefix("skill:")
        .ok_or(ProjectSkillObservationError::InvalidResource)
        .and_then(|value| {
            AgentSkillName::new(value).map_err(|_| ProjectSkillObservationError::InvalidResource)
        })?;
    let (canonical_root, canonical_destination) = project_skill_paths(project, &name);
    let (canonical, skill) =
        canonical_project_skill(filesystem, &canonical_root, &canonical_destination, limits)?;
    let validation = match &canonical {
        CanonicalProjectSkillObservation::Present { format, .. } => Some(format),
        _ => None,
    };
    let canonical_healthy = skill.is_some();
    let mut observed_targets = BTreeMap::new();
    for target in selected_targets.iter() {
        let ownership = state
            .and_then(|state| state.target(target))
            .map_or(Ownership::Unmanaged, |binding| binding.ownership());
        let Some(adapter) = registry.adapter(target) else {
            return Err(ProjectSkillObservationError::InvalidResource);
        };
        let Some(projection_port) = adapter.skill_projection() else {
            observed_targets.insert(
                target.clone(),
                TargetProjectSkillObservation {
                    compatibility: SkillCompatibility::blocked(target.clone()),
                    projection: ProjectSkillLinkHealth::UnmanagedConflict,
                    ownership,
                },
            );
            continue;
        };
        let compatibility = match (&skill, validation) {
            (Some(skill), Some(validation)) => {
                projection_port.compatibility(target, skill, validation)
            }
            _ => SkillCompatibility::blocked(target.clone()),
        };
        let projection = match projection_port.destination(paths, resource.scope()) {
            None => ProjectSkillLinkHealth::UnmanagedConflict,
            Some(native_root) => match project_skill_projection(project, &native_root, &name) {
                Ok(TargetProjectSkillProjection::Canonical { .. }) => {
                    ProjectSkillLinkHealth::NotRequired
                }
                Ok(TargetProjectSkillProjection::RelativeLink(spec)) => match filesystem
                    .inspect_entry_beneath_no_follow(&spec.project_root, &spec.destination)
                {
                    Ok(ConfinedEntryObservation::Missing) => ProjectSkillLinkHealth::Missing,
                    Ok(ConfinedEntryObservation::RelativeSymlink { target, .. })
                        if target == spec.target && canonical_healthy =>
                    {
                        ProjectSkillLinkHealth::Healthy
                    }
                    Ok(ConfinedEntryObservation::RelativeSymlink { target, .. })
                        if target == spec.target =>
                    {
                        ProjectSkillLinkHealth::Broken
                    }
                    Ok(ConfinedEntryObservation::RelativeSymlink { .. })
                        if ownership == Ownership::Skilltap =>
                    {
                        ProjectSkillLinkHealth::Divergent
                    }
                    Ok(_) => ProjectSkillLinkHealth::UnmanagedConflict,
                    Err(_) => ProjectSkillLinkHealth::Broken,
                },
                Err(_) => ProjectSkillLinkHealth::UnmanagedConflict,
            },
        };
        observed_targets.insert(
            target.clone(),
            TargetProjectSkillObservation {
                compatibility,
                projection,
                ownership,
            },
        );
    }
    Ok(ProjectSkillObservation {
        resource: resource.key().clone(),
        canonical,
        targets: observed_targets,
    })
}

pub(super) fn enumerate_canonical_project_skills(
    filesystem: &dyn ConfinedFileSystem,
    project: &AbsolutePath,
    limits: ExternalTreeLimits,
) -> Result<Vec<AgentSkillName>, ProjectSkillObservationError> {
    let root = AbsolutePath::new(format!("{}/.agents/skills", project.as_str()))
        .map_err(|_| ProjectSkillObservationError::InvalidResource)?;
    let names = match filesystem.list_direct_entries_beneath_no_follow(&root, limits.entries()) {
        Ok(names) => names,
        Err(RuntimeError::FileSystem { source, .. })
            if source.kind() == std::io::ErrorKind::NotFound =>
        {
            return Ok(Vec::new());
        }
        Err(_) => return Err(ProjectSkillObservationError::UnsupportedFilesystem),
    };
    let mut result = Vec::new();
    for value in names {
        let Ok(name) = AgentSkillName::new(value.clone()) else {
            continue;
        };
        let destination = skilltap_core::domain::RelativeArtifactPath::new(value)
            .map_err(|_| ProjectSkillObservationError::InvalidResource)?;
        if matches!(
            filesystem.inspect_entry_beneath_no_follow(&root, &destination),
            Ok(ConfinedEntryObservation::Directory)
        ) {
            result.push(name);
        }
    }
    Ok(result)
}

/// Resolve CLI scope/target arguments before entering the source-less project
/// skill reconciliation path used by `plan` and `sync`.
pub(super) fn execute_project_skill_source_less_command(
    application: &StatusApplication<'_>,
    command: &'static str,
    requested_scope: &crate::command::ScopeArgs,
    requested_target: &crate::command::TargetArgs,
    resource: &DesiredResource,
    acknowledged: bool,
) -> Outcome {
    let (documents, mut outcome) = match application.load_documents(command) {
        Ok(value) => value,
        Err(outcome) => return *outcome,
    };
    let status_args = crate::command::StatusArgs {
        target: requested_target.clone(),
        scope: requested_scope.clone(),
        output: crate::command::OutputArgs::default(),
    };
    let scope = match StatusScope::resolve(application, &status_args, &documents) {
        Ok(scope) => scope,
        Err(error) => {
            outcome.result = ResultClass::Invalid;
            return outcome.with_error(error);
        }
    };
    let targets = match StatusTargets::resolve(&status_args, &documents) {
        Ok(targets) => targets,
        Err(_) => {
            outcome.result = ResultClass::Invalid;
            return outcome.with_error(ErrorDetail::new(
                "target_not_enabled",
                "The requested project skill target is not enabled.",
            ));
        }
    };
    let paths = match skilltap_core::runtime::PlatformPaths::resolve(
        &skilltap_core::runtime::ProcessEnvironment,
    ) {
        Ok(paths) => paths,
        Err(_) => {
            outcome.result = ResultClass::Invalid;
            return outcome.with_error(ErrorDetail::new(
                "platform_paths_unavailable",
                "The skilltap configuration paths could not be resolved.",
            ));
        }
    };
    outcome.scope = Some(scope.output.clone());
    execute_project_skill_source_less(
        application,
        command,
        &scope,
        &targets,
        resource,
        acknowledged,
        paths,
        outcome,
    )
}

#[derive(Clone)]
struct ProjectSkillAdoptionCandidate {
    resource: DesiredResource,
    fingerprint: Fingerprint,
}

fn collect_project_skill_adoption_candidates(
    filesystem: &dyn ConfinedFileSystem,
    project: &AbsolutePath,
    targets: &HarnessSet,
    limits: ExternalTreeLimits,
) -> Result<(Vec<ProjectSkillAdoptionCandidate>, usize), ProjectSkillObservationError> {
    let names = enumerate_canonical_project_skills(filesystem, project, limits)?;
    let mut candidates = Vec::new();
    let mut invalid = 0;
    for name in names {
        let (root, destination) = project_skill_paths(project, &name);
        let (canonical, skill) = canonical_project_skill(filesystem, &root, &destination, limits)?;
        let Some(skill) = skill else {
            if !matches!(canonical, CanonicalProjectSkillObservation::Missing) {
                invalid += 1;
            }
            continue;
        };
        let key = ResourceKey::new(
            ResourceId::new(format!("skill:{name}"))
                .map_err(|_| ProjectSkillObservationError::InvalidResource)?,
            Scope::Project(project.clone()),
        );
        let origin_target = targets
            .iter()
            .next()
            .cloned()
            .ok_or(ProjectSkillObservationError::InvalidResource)?;
        let resource = DesiredResource::new(
            key,
            ResourceKind::StandaloneSkill,
            targets.clone(),
            DesiredOrigin::Adopted(origin_target),
            None,
            UpdateIntent::Track,
            ComponentGraph::new([]).expect("empty skill graph is valid"),
            BTreeMap::new(),
            BTreeMap::new(),
            BTreeSet::new(),
        )
        .map_err(|_| ProjectSkillObservationError::InvalidResource)?;
        candidates.push(ProjectSkillAdoptionCandidate {
            resource,
            fingerprint: skill.fingerprint().clone(),
        });
    }
    Ok((candidates, invalid))
}

/// Adopt validated direct-child canonical project skills without touching native
/// links, state, or canonical content. A later source-less sync owns links.
pub(super) fn adopt_project_skills(
    application: &StatusApplication<'_>,
    scope: &StatusScope,
    targets: &StatusTargets,
    paths: &skilltap_core::runtime::PlatformPaths,
    mut outcome: Outcome,
) -> Outcome {
    let filesystem = SystemFileSystem;
    let limits = ExternalTreeLimits::new(16, 10_000, 4 * 1024 * 1024, 64 * 1024 * 1024, 64 * 1024)
        .expect("bounded project skill adoption limits are valid");
    let selected_targets = targets.resolved.clone();
    let mut candidates = Vec::new();
    for concrete_scope in &scope.resolved {
        let Scope::Project(project) = concrete_scope else {
            continue;
        };
        match collect_project_skill_adoption_candidates(
            &filesystem,
            project,
            &selected_targets,
            limits,
        ) {
            Ok((mut values, invalid)) => {
                candidates.append(&mut values);
                if invalid > 0 {
                    outcome = outcome.with_warning(Warning::new(
                        "skill.format.invalid",
                        "One or more direct-child canonical project skills are malformed and were not adopted.",
                    ));
                }
            }
            Err(_) => {
                outcome = outcome.with_warning(Warning::new(
                    "skill_observation_unavailable",
                    "Canonical project skills could not be enumerated safely; adoption preserved inventory.",
                ));
            }
        }
    }
    if candidates.is_empty() {
        return outcome.with_summary("project_adopted", 0_u64);
    }
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
    let lock = skilltap_core::runtime::SystemConfigurationLock;
    let guard = match skilltap_core::runtime::ConfigurationLock::try_acquire(&lock, &lock_path) {
        Ok(guard) => guard,
        Err(_) => {
            outcome.result = ResultClass::AttentionRequired;
            return outcome.with_warning(Warning::new(
                "configuration_locked",
                "Another skilltap mutation holds the configuration lock; project adoption did not write inventory.",
            ));
        }
    };
    let result = (|| {
        let mut fresh = Vec::new();
        for concrete_scope in &scope.resolved {
            let Scope::Project(project) = concrete_scope else {
                continue;
            };
            let (values, _) = collect_project_skill_adoption_candidates(
                &filesystem,
                project,
                &selected_targets,
                limits,
            )
            .map_err(|_| ())?;
            fresh.extend(
                values
                    .into_iter()
                    .map(|candidate| (candidate.resource.key().clone(), candidate.fingerprint)),
            );
        }
        let mut expected = candidates
            .iter()
            .map(|candidate| {
                (
                    candidate.resource.key().clone(),
                    candidate.fingerprint.clone(),
                )
            })
            .collect::<Vec<_>>();
        fresh.sort();
        expected.sort();
        if fresh != expected {
            return Err(());
        }
        let mut inventory = match application.inventory.load().map_err(|_| ())? {
            DocumentState::Present(document) => document,
            DocumentState::Missing => {
                InventoryDocument::new(skilltap_core::storage::INVENTORY_SCHEMA_VERSION, [], [])
                    .map_err(|_| ())?
            }
        };
        let mut adopted = 0_u64;
        for candidate in &candidates {
            if inventory.resources().contains_key(candidate.resource.key()) {
                continue;
            }
            inventory = inventory
                .with_resource(candidate.resource.clone())
                .map_err(|_| ())?;
            adopted += 1;
        }
        let changed = application.inventory.load().ok().is_some_and(|current| {
            matches!(current, DocumentState::Present(ref document) if document != &inventory)
                || matches!(current, DocumentState::Missing) && adopted > 0
        });
        if changed {
            application.inventory.replace(&inventory).map_err(|_| ())?;
        }
        Ok::<_, ()>((adopted, changed))
    })();
    let released = guard.release();
    let (adopted, changed) = match (result, released) {
        (Ok(result), Ok(())) => result,
        _ => {
            outcome.result = ResultClass::AttentionRequired;
            return outcome.with_warning(Warning::new(
                "inventory_publish_failed",
                "Project skill adoption could not publish its desired inventory safely.",
            ));
        }
    };
    for candidate in candidates {
        if !outcome
            .resources
            .iter()
            .any(|resource| resource.id == candidate.resource.key().to_string())
        {
            outcome = outcome.with_resource(OutputEntry::new(
                candidate.resource.key().to_string(),
                if adopted > 0 {
                    "adopted"
                } else {
                    "already_managed"
                },
            ));
        }
    }
    outcome
        .with_summary("project_adopted", adopted)
        .with_summary("changed", changed)
}

/// Reconcile a desired project skill whose canonical tree is already present.
/// This is the source-less adoption path: it validates and reads the canonical
/// tree, then repairs only selected target links without replacing content.
#[allow(clippy::too_many_arguments)]
fn execute_project_skill_source_less(
    application: &StatusApplication<'_>,
    command: &'static str,
    scope: &StatusScope,
    targets: &StatusTargets,
    resource: &DesiredResource,
    _acknowledged: bool,
    paths: skilltap_core::runtime::PlatformPaths,
    mut outcome: Outcome,
) -> Outcome {
    let filesystem = SystemFileSystem;
    let limits =
        ExternalTreeLimits::new(64, 100_000, 64 * 1024 * 1024, 1024 * 1024 * 1024, 64 * 1024)
            .expect("bounded project skill limits are valid");
    let process_limits = ProcessLimits::new(5_000, 256 * 1024, 256 * 1024, 512 * 1024)
        .expect("bounded conditional profile process limits are valid");
    let json_limits =
        JsonLimits::new(256 * 1024, 64).expect("bounded conditional profile JSON limits are valid");
    let config = application
        .config
        .load()
        .ok()
        .and_then(|document| match document {
            DocumentState::Present(document) => Some(document),
            DocumentState::Missing => None,
        })
        .unwrap_or_else(ConfigDocument::defaults);
    let state_document = application
        .state
        .load()
        .ok()
        .and_then(|document| match document {
            DocumentState::Present(document) => Some(document),
            DocumentState::Missing => None,
        });
    let mut operations = Vec::new();
    let mut link_entries = BTreeMap::new();
    let mut blocked = false;
    let mut seeds = BTreeMap::new();

    for concrete_scope in &scope.resolved {
        let Scope::Project(project) = concrete_scope else {
            continue;
        };
        let selected_requested = HarnessSet::new(
            targets
                .iter()
                .filter(|target| resource.targets().contains(target))
                .cloned(),
        )
        .expect("selected project skill targets are unique");
        let (selected, next_outcome) = super::conditional_profile::filter_targets_for_capability(
            application.registry,
            &config,
            &selected_requested,
            concrete_scope,
            &paths,
            process_limits,
            json_limits,
            &filesystem,
            &CapabilityId::new("skill.install").expect("static skill capability is valid"),
            outcome,
        );
        outcome = next_outcome;
        let Some(selected) = selected else {
            continue;
        };
        let observed = match observe_project_skill(
            application.registry,
            &filesystem,
            &paths,
            resource,
            state_document
                .as_ref()
                .and_then(|state| state.resources().get(resource.key())),
            &selected,
            limits,
        ) {
            Ok(observed) => observed,
            Err(_) => {
                blocked = true;
                outcome = outcome.with_warning(Warning::new(
                    "skill_observation_unavailable",
                    "The canonical project skill could not be observed safely; no target link was changed.",
                ));
                continue;
            }
        };
        let canonical_healthy = matches!(
            observed.canonical,
            CanonicalProjectSkillObservation::Present { .. }
        );
        if !canonical_healthy {
            blocked = true;
            outcome = outcome.with_warning(Warning::new(
                "skill_canonical_unavailable",
                "The source-less project skill has no valid canonical tree; no target link was changed.",
            ));
            continue;
        }

        let existing_state = state_document
            .as_ref()
            .and_then(|state| state.resources().get(resource.key()));
        let mut bindings = Vec::new();
        for target in resource.targets().iter() {
            if !selected.contains(target) {
                if let Some(existing) = existing_state.and_then(|state| state.target(target)) {
                    bindings.push(existing.clone());
                }
                continue;
            }
            let existing = existing_state.and_then(|state| state.target(target));
            let (provenance, ownership) = if matches!(resource.origin(), DesiredOrigin::Adopted(_))
                && observed
                    .targets
                    .get(target)
                    .is_some_and(|target| target.projection == ProjectSkillLinkHealth::NotRequired)
            {
                (Provenance::Adopted, Ownership::Harness)
            } else {
                (Provenance::Direct, Ownership::Skilltap)
            };
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
                    Some(
                        NativeId::new(
                            resource
                                .id()
                                .as_str()
                                .strip_prefix("skill:")
                                .expect("project skill resource id has a name"),
                        )
                        .expect("project skill state native id is valid"),
                    ),
                    provenance,
                    ownership,
                    None,
                    None,
                    observed.canonical.fingerprint().cloned(),
                    None,
                    None,
                    observed_at,
                    existing.and_then(|target| target.last_apply()).cloned(),
                )
                .expect("source-less project skill target state is valid"),
            );
        }
        seeds.insert(
            resource.key().clone(),
            ResourceState::new(resource.key().clone(), bindings)
                .expect("source-less project skill state is valid"),
        );

        let mut seen_destinations = BTreeSet::new();
        for target in selected.iter() {
            let Some(adapter) = application.registry.adapter(target) else {
                blocked = true;
                continue;
            };
            let Some(projection_port) = adapter.skill_projection() else {
                blocked = true;
                continue;
            };
            let Some(native_root) = projection_port.destination(&paths, concrete_scope) else {
                blocked = true;
                continue;
            };
            let name = AgentSkillName::new(
                resource
                    .id()
                    .as_str()
                    .strip_prefix("skill:")
                    .expect("project skill resource id has a name"),
            )
            .expect("project skill resource id has a valid name");
            let projection = match project_skill_projection(project, &native_root, &name) {
                Ok(projection) => projection,
                Err(_) => {
                    blocked = true;
                    outcome = outcome.with_warning(Warning::new(
                        "skill_destination_invalid",
                        "The selected project skill destination cannot be represented safely.",
                    ));
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
            let state_target = state_document
                .as_ref()
                .and_then(|state| state.resources().get(resource.key()))
                .and_then(|state| state.target(target));
            let observation = match filesystem
                .inspect_entry_beneath_no_follow(&spec.project_root, &spec.destination)
            {
                Ok(observation) => observation,
                Err(_) => {
                    blocked = true;
                    continue;
                }
            };
            let action = match observation {
                ConfinedEntryObservation::Missing => Some(ProjectSkillLinkAction::Create),
                ConfinedEntryObservation::RelativeSymlink { target, .. }
                    if target == spec.target =>
                {
                    None
                }
                ConfinedEntryObservation::RelativeSymlink {
                    identity,
                    target: previous_target,
                } if state_target.is_some_and(|state| state.ownership() == Ownership::Skilltap) => {
                    Some(ProjectSkillLinkAction::Replace {
                        expected_identity: identity,
                        previous_target,
                    })
                }
                _ => {
                    blocked = true;
                    outcome = outcome.with_warning(Warning::new(
                        "skill_destination_unmanaged",
                        "The selected project skill destination is unmanaged or divergent; it was preserved.",
                    ));
                    None
                }
            };
            let Some(action) = action else {
                continue;
            };
            let operation_id = project_link_operation_id(target, resource.key(), &destination_path);
            let operation = match skilltap_core::lifecycle_operation::faithful_file_operation(
                operation_id.clone(),
                target.clone(),
                resource.key().clone(),
                OperationAction::SkillInstall,
                destination_path,
            ) {
                Ok(operation) => operation,
                Err(_) => {
                    outcome.result = ResultClass::Invalid;
                    return outcome.with_error(ErrorDetail::new(
                        "operation_contract_invalid",
                        "The source-less project skill link operation was invalid.",
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
    }

    if blocked {
        outcome.result = ResultClass::AttentionRequired;
        return outcome
            .with_summary("operations", 0_u64)
            .with_summary("changed", false);
    }
    if command == "plan" {
        for operation in &operations {
            outcome = outcome.with_operation(
                crate::OperationOutcome::new(operation.id().to_string(), "planned")
                    .with_field("target", operation.target().as_str())
                    .with_field("scope", scope_label(operation.scope())),
            );
        }
        return outcome
            .with_summary("operations", operations.len() as u64)
            .with_summary("changed", false);
    }
    if operations.is_empty() {
        if refresh_state_seeds(application.state, &seeds).is_err() {
            outcome.result = ResultClass::Invalid;
            return outcome.with_error(ErrorDetail::new(
                "state_seed_publish_failed",
                "The source-less project skill state could not be recorded safely.",
            ));
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
                "The source-less project skill link plan was invalid.",
            ));
        }
    };
    let port = ProjectSkillLifecyclePort {
        canonical: ManagedSkillPort {
            filesystem: &filesystem,
            entries: BTreeMap::new(),
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
    if report.changed && outcome.errors.is_empty() && outcome.warnings.is_empty() {
        outcome.result = ResultClass::Completed;
    }
    outcome
        .with_summary("operations", report.result.operations().len() as u64)
        .with_summary("changed", report.changed)
}

/// Execute a project-only skill install, update, or reconciliation. Global
/// behavior remains in the established lifecycle path.
#[allow(clippy::too_many_arguments)]
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

    let Some(project) = scope.resolved.iter().find_map(|scope| match scope {
        Scope::Project(project) => Some(project),
        Scope::Global => None,
    }) else {
        return outcome;
    };
    let process_limits = ProcessLimits::new(5_000, 256 * 1024, 256 * 1024, 512 * 1024)
        .expect("bounded conditional profile process limits are valid");
    let json_limits =
        JsonLimits::new(256 * 1024, 64).expect("bounded conditional profile JSON limits are valid");
    let search_path = std::env::var_os("PATH");
    let environment = match paths.native_process_environment(search_path.clone()) {
        Ok(environment) => environment,
        Err(_) => {
            outcome.result = ResultClass::AttentionRequired;
            return outcome.with_error(ErrorDetail::new(
                "native_environment_unavailable",
                "The bounded native process environment could not be resolved.",
            ));
        }
    };
    let capability_name = if command == "skill update" || command == "sync" {
        "skill.update"
    } else {
        "skill.install"
    };
    let config = application
        .config
        .load()
        .ok()
        .and_then(|document| match document {
            DocumentState::Present(document) => Some(document),
            DocumentState::Missing => None,
        })
        .unwrap_or_else(ConfigDocument::defaults);
    let mut profile_target_ids = Vec::new();
    for target in targets.iter() {
        match super::configured_adapter_profile(
            application.registry,
            &config,
            target,
            super::NativeProfileRequest {
                scope: &Scope::Project(project.clone()),
                environment: &environment,
                process_limits,
                json_limits,
                search_path: search_path.clone(),
                capability_name,
            },
        ) {
            Ok(Some(profile)) if profile.capability == CapabilitySupport::Supported => {
                profile_target_ids.push(target.clone());
            }
            Ok(Some(_)) | Ok(None) | Err(_) => {
                outcome = outcome.with_warning(
                    Warning::new(
                        "skill_mutation_unavailable",
                        "The selected harness profile is not verified for project skill mutation; no files were written for it.",
                    )
                    .with_context("target", target.as_str()),
                );
            }
        }
    }
    let profile_targets = HarnessSet::new(profile_target_ids).ok();
    let mut partial_targets = BTreeSet::new();
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
                partial_targets.insert(target.clone());
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
    let original_inventory = inventory.clone();
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
        let (mutating_targets, next_outcome) =
            super::conditional_profile::filter_targets_for_capability(
                application.registry,
                &config,
                &targets.resolved,
                concrete_scope,
                &paths,
                process_limits,
                json_limits,
                &filesystem,
                &CapabilityId::new(capability_name).expect("static skill capability is valid"),
                outcome,
            );
        outcome = next_outcome;
        let Some(mutating_targets) = mutating_targets else {
            continue;
        };
        let Some(profile_targets) = profile_targets.as_ref() else {
            outcome = outcome.with_warning(Warning::new(
                "skill_mutation_unavailable",
                "No verified project skill mutation profile is available; no files were written.",
            ));
            continue;
        };
        let authorized_target_ids = mutating_targets
            .iter()
            .filter(|target| profile_targets.contains(target))
            .cloned()
            .collect::<Vec<_>>();
        let Some(mutating_targets) = HarnessSet::new(authorized_target_ids).ok() else {
            outcome = outcome.with_warning(Warning::new(
                "skill_mutation_unavailable",
                "No selected harness has a verified project skill mutation profile; no files were written.",
            ));
            continue;
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
        desired_targets.extend(mutating_targets.iter().cloned());
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
                        ))
                        .with_summary("operations", 0_u64)
                        .with_summary("changed", false);
                }
            }
            let operation_id = project_canonical_operation_id(&key);
            let path = AbsolutePath::new(format!(
                "{}/{}",
                canonical_root.as_str(),
                canonical_destination.as_str()
            ))
            .expect("canonical skill path is valid");
            let target = mutating_targets
                .iter()
                .next()
                .cloned()
                .expect("non-empty mutation targets");
            let operation = match project_skill_operation(
                operation_id.clone(),
                target,
                key.clone(),
                path,
                Vec::new(),
                mutating_targets
                    .iter()
                    .any(|target| partial_targets.contains(target)),
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
        for target in mutating_targets.iter() {
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
            let operation = match project_skill_operation(
                operation_id.clone(),
                target.clone(),
                key.clone(),
                destination_path,
                dependencies,
                partial_targets.contains(target),
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
                && !mutating_targets.contains(target)
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
    if !acknowledged && operations.iter().any(|operation| {
        operation.class() == skilltap_core::domain::OperationClass::Partial
    }) {
        outcome.result = ResultClass::AttentionRequired;
        return outcome
            .with_warning(Warning::new(
                "partial_operation_requires_acknowledgment",
                "The project skill plan contains an exact compatibility consequence; rerun with `--yes` to accept it.",
            ))
            .with_next_action(NextAction::new(
                "accept_partial",
                "Review the project skill plan, then retry with `--yes` if acceptable.",
            ))
            .with_summary("operations", operations.len() as u64)
            .with_summary("changed", false);
    }
    if inventory != original_inventory && application.inventory.replace(&inventory).is_err() {
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
                "The lock path is invalid.",
            ));
        }
    };
    let acknowledgments = if acknowledged {
        skilltap_core::executor::ExecutionAcknowledgments::foreground_all(&plan)
    } else {
        skilltap_core::executor::ExecutionAcknowledgments::default()
    };
    let report = match skilltap_core::executor::execute_plan_with_acknowledgments(
        &skilltap_core::runtime::SystemConfigurationLock,
        &lock_path,
        &port,
        &journal,
        &plan,
        &acknowledgments,
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
    let process_limits = ProcessLimits::new(5_000, 256 * 1024, 256 * 1024, 512 * 1024)
        .expect("bounded conditional profile process limits are valid");
    let json_limits =
        JsonLimits::new(256 * 1024, 64).expect("bounded conditional profile JSON limits are valid");
    let config = application
        .config
        .load()
        .ok()
        .and_then(|document| match document {
            DocumentState::Present(document) => Some(document),
            DocumentState::Missing => None,
        })
        .unwrap_or_else(ConfigDocument::defaults);
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
    let mut authorized_targets = Vec::new();
    let mut blocked = false;
    let Some(project) = scope.resolved.iter().find_map(|scope| match scope {
        Scope::Project(project) => Some(project),
        Scope::Global => None,
    }) else {
        return outcome;
    };
    let search_path = std::env::var_os("PATH");
    let environment = match paths.native_process_environment(search_path.clone()) {
        Ok(environment) => environment,
        Err(_) => {
            outcome.result = ResultClass::AttentionRequired;
            return outcome.with_error(ErrorDetail::new(
                "native_environment_unavailable",
                "The bounded native process environment could not be resolved.",
            ));
        }
    };
    let mut profile_target_ids = Vec::new();
    for target in targets.iter() {
        match super::configured_adapter_profile(
            application.registry,
            &config,
            target,
            super::NativeProfileRequest {
                scope: &Scope::Project(project.clone()),
                environment: &environment,
                process_limits,
                json_limits,
                search_path: search_path.clone(),
                capability_name: "skill.remove",
            },
        ) {
            Ok(Some(profile)) if profile.capability == CapabilitySupport::Supported => {
                profile_target_ids.push(target.clone());
            }
            Ok(Some(_)) | Ok(None) | Err(_) => {
                outcome = outcome.with_warning(
                    Warning::new(
                        "skill_mutation_unavailable",
                        "The selected harness profile is not verified for project skill removal; no files were removed for it.",
                    )
                    .with_context("target", target.as_str()),
                );
            }
        }
    }
    if profile_target_ids.is_empty() {
        outcome.result = ResultClass::AttentionRequired;
        return outcome.with_error(ErrorDetail::new(
            "skill_mutation_unavailable",
            "No selected harness has a verified project skill removal profile.",
        ));
    }
    let profile_targets = HarnessSet::new(profile_target_ids)
        .expect("verified project removal profile targets are non-empty");

    for concrete_scope in &scope.resolved {
        let Scope::Project(project) = concrete_scope else {
            continue;
        };
        let (mutating_targets, next_outcome) =
            super::conditional_profile::filter_targets_for_capability(
                application.registry,
                &config,
                &profile_targets,
                concrete_scope,
                &paths,
                process_limits,
                json_limits,
                &filesystem,
                &CapabilityId::new("skill.remove").expect("static skill capability is valid"),
                outcome,
            );
        outcome = next_outcome;
        let Some(mutating_targets) = mutating_targets else {
            continue;
        };
        authorized_targets.extend(mutating_targets.iter().cloned());
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
        for target in mutating_targets.iter() {
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
        let remaining = resource.targets().iter().any(|target| {
            !authorized_targets
                .iter()
                .any(|authorized| authorized == target)
        });
        let canonical_owned = matches!(resource.origin(), DesiredOrigin::Direct)
            && state.targets().values().any(|target| {
                target.provenance() == Provenance::Direct
                    && target.ownership() == Ownership::Skilltap
            });
        if !remaining
            && canonical_owned
            && let Some(canonical) = canonical
        {
            let operation_id = project_canonical_remove_operation_id(&key);
            let Some(target) = authorized_targets
                .iter()
                .find(|target| resource.targets().contains(target))
                .cloned()
            else {
                continue;
            };
            let dependencies = link_ids.iter().cloned().map(OperationDependency::new);
            let path = AbsolutePath::new(format!(
                "{}/{}",
                canonical_root.as_str(),
                canonical_destination.as_str()
            ))
            .expect("canonical skill path is valid");
            let operation =
                match skilltap_core::lifecycle_operation::faithful_file_operation_with_dependencies(
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
        target_projection_keys.insert(key);
    }
    let authorized_targets = HarnessSet::new(authorized_targets)
        .expect("authorized project skill targets remain unique");
    if authorized_targets.iter().next().is_none() {
        outcome.result = ResultClass::AttentionRequired;
        return outcome
            .with_summary("operations", 0_u64)
            .with_summary("changed", false);
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
            &authorized_targets,
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
                &authorized_targets,
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
            &authorized_targets,
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
                &authorized_targets,
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
