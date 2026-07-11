use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    path::{Component, Path, PathBuf},
};

use skilltap_core::{
    adoption::{
        AdoptionApplyError, AdoptionDecision, AdoptionObservationError, AdoptionSelection,
        apply_adoption, plan_adoption,
    },
    domain::{
        AbsolutePath, CapabilityId, CapabilitySupport, CommandArgument, ComponentGraph,
        ConfiguredBinary, DesiredOrigin, DesiredResource, GitCommit, HarnessId, HarnessObservation,
        HarnessObservationOutcome, HarnessReachability, HarnessSet, NativeId,
        ObservationAdapterError, ObservationBatch, ObservationEvidence, ObservationFields,
        ObservationFinding, ObservationFindingCode, ObservationKey, ObservationLayer,
        ObservationRequest, ObservationSeverity, ObservationSubject, ObservationSummary,
        ObservationTarget, ObservedResource, OperationAction, OperationId, OperationOutcome,
        OperationResult, Ownership, Plan, ProfileAuthority, Provenance, ResourceHealth, ResourceId,
        ResourceKey, ResourceKind, Scope, Source, SourceKind, SourceLocator, UpdateIntent,
    },
    executor::{ExecutionError, ExecutionJournal, ExecutionPort, execute_plan},
    instructions::fingerprint_contents,
    lifecycle_operation::native_operation,
    reconciliation::{ReconciliationRequest, plan_reconciliation},
    runtime::{
        DirectoryTreeFileSystem, ExecutableResolutionRequest, ExecutableResolver,
        ExternalTreeLimits, ExternalTreeObserver, ExternalTreeRequest, FileKind, FileSystem,
        JsonLimits, NativeProcessRequest, NativeProcessRunner, PlatformPaths, ProcessEnvironment,
        ProcessLimits, RelativeSymlinkTarget, ScopeRequest, ScopeResolver, SystemConfigurationLock,
        SystemExecutableResolver, SystemExternalTreeObserver, SystemFileSystem,
        SystemNativeProcessRunner, WorkingDirectory, resolve_targets,
    },
    skill::ValidatedSkillTree,
    skill_compatibility::{SkillCompatibility, SkillCompatibilityClass},
    storage::{
        ArtifactTree, ClaudeInstructionMode, ConfigDocument, ConfigRepository, DocumentState,
        InventoryDocument, InventoryRepository, ManagedArtifactRepository, ResourceState,
        StateDocument, StateRepository, StorageError, StorageFailure, Timestamp,
    },
    updates::{
        ResolutionError, SourceRevisionResolver, UpdateCandidate, UpdateDecision,
        UpdateDecisionReason, UpdateResolutionRequest, UpdateSafety, candidate_for,
        classify_update_with_mode, resolve_candidate,
    },
};
use skilltap_harnesses::{
    CanonicalObservation, GitSourceRevisionResolver, HarnessKind, NativeLifecycleAction,
    NativeLifecyclePort, NativeLifecycleRequest, ObservedNativeRevisionResolver,
    detect_configured_installation, native_arguments, normalize_observations,
    observe_claude_canonical_resources, observe_codex_canonical_resources, select_profile,
};

use crate::{
    ErrorDetail, NextAction, Outcome, OutputEntry, OutputScope, OutputValue, ResultClass, Warning,
    command::{
        AdoptArgs, OutputArgs, PlanArgs, ScopeArgs, ScopeArgument, ScopedOutputArgs,
        ScopedTargetArgs, StatusArgs, SyncArgs, TargetArgs,
    },
};

pub(crate) struct StatusApplication<'a> {
    pub(crate) config: &'a dyn ConfigRepository,
    pub(crate) inventory: &'a dyn InventoryRepository,
    pub(crate) state: &'a dyn StateRepository,
    pub(crate) scopes: &'a ScopeResolver<'a>,
    pub(crate) working_directory: &'a dyn WorkingDirectory,
    pub(crate) native_observation: NativeObservationMode,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NativeObservationMode {
    Disabled,
    System,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NativeLifecycleKind {
    MarketplaceAdd,
    MarketplaceRemove,
    MarketplaceUpdate,
    PluginInstall,
    PluginRemove,
    PluginUpdate,
}

pub(crate) struct SkillInstallRequest<'a> {
    pub(crate) source: &'a str,
    pub(crate) name: Option<&'a str>,
    pub(crate) requested_revision: Option<&'a str>,
    pub(crate) subdirectory: Option<&'a str>,
}

/// State-backed journal for mutating lifecycle composition. It resolves each
/// result through the validated plan, seeds only explicitly planned resources,
/// updates exact resource records, and publishes atomically through the
/// repository port.
pub(crate) struct StateExecutionJournal<'a> {
    pub(crate) plan: &'a Plan,
    pub(crate) state: &'a dyn StateRepository,
    pub(crate) seeds: BTreeMap<ResourceKey, ResourceState>,
}

impl ExecutionJournal for StateExecutionJournal<'_> {
    fn record(&self, result: &OperationResult) -> Result<(), ExecutionError> {
        let operation = self.plan.get(result.operation_id()).ok_or_else(|| {
            ExecutionError::journal_failure(
                skilltap_core::domain::EvidenceCode::new("state.operation_unknown")
                    .expect("static evidence code is valid"),
                skilltap_core::domain::EvidenceDetail::new(
                    "The state journal received an operation outside the validated plan.",
                )
                .expect("static evidence detail is valid"),
            )
        })?;
        let resource = operation.selector().resource();
        let current = self.state.load().map_err(|_| {
            ExecutionError::journal_failure(
                skilltap_core::domain::EvidenceCode::new("state.load_failed")
                    .expect("static evidence code is valid"),
                skilltap_core::domain::EvidenceDetail::new(
                    "The state document could not be loaded for journaling.",
                )
                .expect("static evidence detail is valid"),
            )
        })?;
        let current = match current {
            DocumentState::Present(current) => current,
            DocumentState::Missing => skilltap_core::storage::StateDocument::new(
                skilltap_core::storage::STATE_SCHEMA_VERSION,
                [],
                [],
                None,
                None,
                None,
            )
            .map_err(|_| {
                ExecutionError::journal_failure(
                    skilltap_core::domain::EvidenceCode::new("state.seed_invalid")
                        .expect("static evidence code is valid"),
                    skilltap_core::domain::EvidenceDetail::new(
                        "The seed state for the operation was invalid.",
                    )
                    .expect("static evidence detail is valid"),
                )
            })?,
        };
        let current = if current.resources().contains_key(resource) {
            if let Some(seed) = self.seeds.get(resource) {
                current.refresh_resource_state(seed.clone()).map_err(|_| {
                    ExecutionError::journal_failure(
                        skilltap_core::domain::EvidenceCode::new("state.seed_refresh_failed")
                            .expect("static evidence code is valid"),
                        skilltap_core::domain::EvidenceDetail::new(
                            "The existing resource metadata could not be refreshed safely.",
                        )
                        .expect("static evidence detail is valid"),
                    )
                })?
            } else {
                current
            }
        } else if let Some(seed) = self.seeds.get(resource) {
            current.with_resource_state(seed.clone()).map_err(|_| {
                ExecutionError::journal_failure(
                    skilltap_core::domain::EvidenceCode::new("state.seed_conflict")
                        .expect("static evidence code is valid"),
                    skilltap_core::domain::EvidenceDetail::new(
                        "The operation resource could not be seeded in state.",
                    )
                    .expect("static evidence detail is valid"),
                )
            })?
        } else {
            return Err(ExecutionError::journal_failure(
                skilltap_core::domain::EvidenceCode::new("state.resource_missing")
                    .expect("static evidence code is valid"),
                skilltap_core::domain::EvidenceDetail::new(
                    "The operation resource is not present in state.",
                )
                .expect("static evidence detail is valid"),
            ));
        };
        let at = Timestamp::from_system_time(std::time::SystemTime::now()).map_err(|_| {
            ExecutionError::journal_failure(
                skilltap_core::domain::EvidenceCode::new("state.clock_invalid")
                    .expect("static evidence code is valid"),
                skilltap_core::domain::EvidenceDetail::new(
                    "The operation timestamp could not be recorded.",
                )
                .expect("static evidence detail is valid"),
            )
        })?;
        let next = current
            .with_operation_result(resource, at, result.clone())
            .map_err(|_| {
                ExecutionError::journal_failure(
                    skilltap_core::domain::EvidenceCode::new("state.resource_unavailable")
                        .expect("static evidence code is valid"),
                    skilltap_core::domain::EvidenceDetail::new(
                        "The operation resource could not be journaled in state.",
                    )
                    .expect("static evidence detail is valid"),
                )
            })?;
        self.state.replace(&next).map_err(|_| {
            ExecutionError::journal_failure(
                skilltap_core::domain::EvidenceCode::new("state.publish_failed")
                    .expect("static evidence code is valid"),
                skilltap_core::domain::EvidenceDetail::new(
                    "The state journal could not be published atomically.",
                )
                .expect("static evidence detail is valid"),
            )
        })
    }
}

struct ManagedSkillPort<'a> {
    filesystem: &'a dyn DirectoryTreeFileSystem,
    entries: BTreeMap<OperationId, ManagedSkillEntry>,
}

struct ManagedSkillEntry {
    root: AbsolutePath,
    destination: skilltap_core::domain::RelativeArtifactPath,
    tree: ArtifactTree,
    backup_tree: Option<ArtifactTree>,
    action: ManagedSkillAction,
    expected_identity: Option<skilltap_core::runtime::DirectoryIdentity>,
    owner: Option<ResourceKey>,
    config_root: Option<AbsolutePath>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ManagedSkillAction {
    Install,
    Replace,
    Remove,
}

impl ExecutionPort for ManagedSkillPort<'_> {
    fn revalidate(&self, plan: &Plan) -> Result<(), ExecutionError> {
        for (_, operation) in plan.iter() {
            if !matches!(
                operation.action(),
                OperationAction::SkillInstall | OperationAction::SkillRemove
            ) {
                continue;
            }
            let Some(entry) = self.entries.get(operation.id()) else {
                return Err(ExecutionError::revalidation(
                    skilltap_core::domain::EvidenceCode::new("managed.skill_request_missing")
                        .expect("static evidence code is valid"),
                    skilltap_core::domain::EvidenceDetail::new(
                        "The managed skill adapter did not receive a request for a planned operation.",
                    )
                    .expect("static evidence detail is valid"),
                ));
            };
            let expected = AbsolutePath::new(format!(
                "{}/{}",
                entry.root.as_str(),
                entry.destination.as_str()
            ))
            .map_err(|_| {
                ExecutionError::revalidation(
                    skilltap_core::domain::EvidenceCode::new("managed.skill_path_invalid")
                        .expect("static evidence code is valid"),
                    skilltap_core::domain::EvidenceDetail::new(
                        "The managed skill destination could not be represented safely.",
                    )
                    .expect("static evidence detail is valid"),
                )
            })?;
            if !operation
                .affected_surfaces()
                .iter()
                .any(|surface| surface.path() == Some(&expected))
            {
                return Err(ExecutionError::revalidation(
                    skilltap_core::domain::EvidenceCode::new("managed.skill_surface_mismatch")
                        .expect("static evidence code is valid"),
                    skilltap_core::domain::EvidenceDetail::new(
                        "The managed skill destination no longer matches the validated operation.",
                    )
                    .expect("static evidence detail is valid"),
                ));
            }
        }
        Ok(())
    }

    fn apply(
        &self,
        operation: &skilltap_core::domain::Operation,
    ) -> Result<OperationOutcome, ExecutionError> {
        let Some(entry) = self.entries.get(operation.id()) else {
            return Err(ExecutionError::revalidation(
                skilltap_core::domain::EvidenceCode::new("managed.skill_request_missing")
                    .expect("static evidence code is valid"),
                skilltap_core::domain::EvidenceDetail::new(
                    "The managed skill adapter did not receive a request for a planned operation.",
                )
                .expect("static evidence detail is valid"),
            ));
        };
        if entry.action == ManagedSkillAction::Remove {
            let Some(expected) = entry.expected_identity else {
                return Err(managed_skill_apply_failure(
                    "The managed skill removal did not include an owned directory identity.",
                ));
            };
            self.filesystem
                .remove_tree_no_follow(&entry.root, &entry.destination, expected)
                .map(|_| OperationOutcome::Applied)
                .map_err(|_| {
                    managed_skill_apply_failure(
                        "The managed skill tree could not be removed safely.",
                    )
                })
        } else if entry.action == ManagedSkillAction::Replace {
            let Some(expected) = entry.expected_identity else {
                return Err(managed_skill_apply_failure(
                    "The managed skill replacement did not include an owned directory identity.",
                ));
            };
            let Some(owner) = &entry.owner else {
                return Err(managed_skill_apply_failure(
                    "The managed skill replacement did not include an ownership record.",
                ));
            };
            let Some(config_root) = &entry.config_root else {
                return Err(managed_skill_apply_failure(
                    "The managed skill replacement did not include a backup root.",
                ));
            };
            let repository = skilltap_core::storage::FileManagedArtifactRepository::new(
                self.filesystem,
                config_root.clone(),
            )
            .map_err(|_| {
                managed_skill_apply_failure(
                    "The managed skill backup repository could not be opened.",
                )
            })?;
            let Some(backup_tree) = &entry.backup_tree else {
                return Err(managed_skill_apply_failure(
                    "The managed skill replacement did not include the previous tree.",
                ));
            };
            let backup = repository.backup(owner, backup_tree).map_err(|_| {
                managed_skill_apply_failure(
                    "The existing skill tree could not be backed up safely.",
                )
            })?;
            self.filesystem
                .remove_tree_no_follow(&entry.root, &entry.destination, expected)
                .map_err(|_| {
                    managed_skill_apply_failure(
                        "The existing skill tree could not be removed safely.",
                    )
                })?;
            match self.filesystem.publish_tree_no_follow(
                &entry.root,
                &entry.destination,
                entry.tree.files(),
            ) {
                Ok(skilltap_core::runtime::DirectoryPublishOutcome::Published(_)) => {
                    Ok(OperationOutcome::Applied)
                }
                Ok(skilltap_core::runtime::DirectoryPublishOutcome::AlreadyExists) => {
                    Ok(OperationOutcome::NoChange)
                }
                Err(_) => {
                    let _ = self.filesystem.publish_tree_no_follow(
                        &entry.root,
                        &entry.destination,
                        backup_tree.files(),
                    );
                    let _ = backup;
                    Err(managed_skill_apply_failure(
                        "The replacement skill tree could not be published after backup.",
                    ))
                }
            }
        } else {
            match self
                .filesystem
                .publish_tree_no_follow(&entry.root, &entry.destination, entry.tree.files())
                .map_err(|_| {
                    managed_skill_apply_failure("The managed skill tree could not be published.")
                })? {
                skilltap_core::runtime::DirectoryPublishOutcome::Published(_) => {
                    Ok(OperationOutcome::Applied)
                }
                skilltap_core::runtime::DirectoryPublishOutcome::AlreadyExists => {
                    let (_, files) = self
                        .filesystem
                        .load_tree_no_follow(&entry.root, &entry.destination)
                        .map_err(|_| {
                            managed_skill_apply_failure(
                                "The existing managed skill tree could not be re-read safely.",
                            )
                        })?;
                    let current = ArtifactTree::new(
                        files
                            .into_iter()
                            .map(|(path, bytes)| (path.as_str().to_owned(), bytes)),
                    )
                    .map_err(|_| {
                        managed_skill_apply_failure(
                            "The existing managed skill tree had an invalid shape.",
                        )
                    })?;
                    if current == entry.tree {
                        Ok(OperationOutcome::NoChange)
                    } else {
                        Err(managed_skill_apply_failure(
                            "The managed skill destination changed before publication.",
                        ))
                    }
                }
            }
        }
    }
}

fn managed_skill_apply_failure(detail: &'static str) -> ExecutionError {
    ExecutionError::apply_failure(skilltap_core::domain::AttentionReason::operation_failed(
        skilltap_core::domain::EvidenceCode::new("managed.skill_publish_failed")
            .expect("static evidence code is valid"),
        skilltap_core::domain::EvidenceDetail::new(detail)
            .expect("static evidence detail is valid"),
    ))
}

enum InstructionWrite {
    Canonical,
    Symlink { target: RelativeSymlinkTarget },
    Import { contents: Vec<u8> },
    Remove,
}

struct InstructionPort<'a> {
    filesystem: &'a dyn FileSystem,
    entries: BTreeMap<OperationId, InstructionEntry>,
}

struct InstructionEntry {
    path: AbsolutePath,
    write: InstructionWrite,
    action: OperationAction,
    backup: Option<AbsolutePath>,
}

impl ExecutionPort for InstructionPort<'_> {
    fn revalidate(&self, plan: &Plan) -> Result<(), ExecutionError> {
        for (_, operation) in plan.iter() {
            if !matches!(
                operation.action(),
                OperationAction::InstructionSetup | OperationAction::InstructionRepair
            ) {
                continue;
            }
            let Some(entry) = self.entries.get(operation.id()) else {
                return Err(ExecutionError::revalidation(
                    skilltap_core::domain::EvidenceCode::new("instructions.request_missing")
                        .expect("static evidence code is valid"),
                    skilltap_core::domain::EvidenceDetail::new(
                        "The instruction adapter did not receive a request for a planned operation.",
                    )
                    .expect("static evidence detail is valid"),
                ));
            };
            if entry.action != operation.action() {
                return Err(ExecutionError::revalidation(
                    skilltap_core::domain::EvidenceCode::new("instructions.action_mismatch")
                        .expect("static evidence code is valid"),
                    skilltap_core::domain::EvidenceDetail::new(
                        "The instruction operation action no longer matches the validated adapter entry.",
                    )
                    .expect("static evidence detail is valid"),
                ));
            }
            if !operation
                .affected_surfaces()
                .iter()
                .any(|surface| surface.path() == Some(&entry.path))
            {
                return Err(ExecutionError::revalidation(
                    skilltap_core::domain::EvidenceCode::new("instructions.surface_mismatch")
                        .expect("static evidence code is valid"),
                    skilltap_core::domain::EvidenceDetail::new(
                        "The instruction destination no longer matches the validated operation.",
                    )
                    .expect("static evidence detail is valid"),
                ));
            }
        }
        Ok(())
    }

    fn apply(
        &self,
        operation: &skilltap_core::domain::Operation,
    ) -> Result<OperationOutcome, ExecutionError> {
        let Some(entry) = self.entries.get(operation.id()) else {
            return Err(ExecutionError::revalidation(
                skilltap_core::domain::EvidenceCode::new("instructions.request_missing")
                    .expect("static evidence code is valid"),
                skilltap_core::domain::EvidenceDetail::new(
                    "The instruction adapter did not receive a request for a planned operation.",
                )
                .expect("static evidence detail is valid"),
            ));
        };
        if matches!(&entry.write, InstructionWrite::Remove) {
            if let Some(backup) = &entry.backup {
                let backup_parent = backup
                    .as_str()
                    .rsplit_once('/')
                    .map(|(parent, _)| parent)
                    .and_then(|parent| AbsolutePath::new(parent).ok())
                    .ok_or_else(|| {
                        instruction_apply_failure("The instruction backup path is invalid.")
                    })?;
                self.filesystem
                    .create_directory_all(&backup_parent)
                    .map_err(|_| {
                        instruction_apply_failure(
                            "The existing instruction bridge could not be backed up safely.",
                        )
                    })?;
                self.filesystem
                    .copy_recoverable(&entry.path, backup)
                    .map_err(|_| {
                        instruction_apply_failure(
                            "The existing instruction bridge could not be backed up safely.",
                        )
                    })?;
            }
            self.filesystem.remove(&entry.path).map_err(|_| {
                instruction_apply_failure(
                    "The duplicate instruction bridge could not be removed safely.",
                )
            })?;
            return Ok(OperationOutcome::Applied);
        }
        let parent = entry
            .path
            .as_str()
            .rsplit_once('/')
            .map(|(parent, _)| parent)
            .and_then(|parent| AbsolutePath::new(parent).ok())
            .ok_or_else(|| instruction_apply_failure("The instruction parent path is invalid."))?;
        self.filesystem.create_directory_all(&parent).map_err(|_| {
            instruction_apply_failure("The instruction parent directory could not be created.")
        })?;
        if let Some(backup) = &entry.backup {
            let backup_parent = backup
                .as_str()
                .rsplit_once('/')
                .map(|(parent, _)| parent)
                .and_then(|parent| AbsolutePath::new(parent).ok())
                .ok_or_else(|| {
                    instruction_apply_failure("The instruction backup path is invalid.")
                })?;
            self.filesystem
                .create_directory_all(&backup_parent)
                .map_err(|_| {
                    instruction_apply_failure(
                        "The instruction backup directory could not be created.",
                    )
                })?;
            self.filesystem
                .copy_recoverable(&entry.path, backup)
                .map_err(|_| {
                    instruction_apply_failure(
                        "The existing instruction bridge could not be backed up safely.",
                    )
                })?;
            self.filesystem.remove(&entry.path).map_err(|_| {
                instruction_apply_failure(
                    "The existing instruction bridge could not be replaced safely.",
                )
            })?;
        }
        match &entry.write {
            InstructionWrite::Canonical => {
                self.filesystem
                    .atomic_write(&entry.path, &[])
                    .map_err(|_| {
                        instruction_apply_failure(
                            "The canonical instruction file could not be created.",
                        )
                    })?
            }
            InstructionWrite::Symlink { target } => self
                .filesystem
                .create_relative_symlink(target, &entry.path)
                .map_err(|_| {
                    instruction_apply_failure("The instruction bridge could not be created.")
                })?,
            InstructionWrite::Import { contents } => self
                .filesystem
                .atomic_write(&entry.path, contents)
                .map_err(|_| {
                    instruction_apply_failure("The instruction import bridge could not be created.")
                })?,
            InstructionWrite::Remove => unreachable!("remove entries return before publication"),
        }
        Ok(OperationOutcome::Applied)
    }
}

fn instruction_apply_failure(detail: &'static str) -> ExecutionError {
    ExecutionError::apply_failure(skilltap_core::domain::AttentionReason::operation_failed(
        skilltap_core::domain::EvidenceCode::new("instructions.publish_failed")
            .expect("static evidence code is valid"),
        skilltap_core::domain::EvidenceDetail::new(detail)
            .expect("static evidence detail is valid"),
    ))
}

impl StatusApplication<'_> {
    /// Build a fresh, adapter-neutral reconciliation plan from the current
    /// documents and bounded native observation. Lifecycle adapters add
    /// concrete candidates in their respective feature slices; until then an
    /// empty inventory is a valid no-op plan and populated inventory is
    /// reported as attention rather than guessed into a mutation.
    pub(crate) fn execute_plan(&self, args: &PlanArgs) -> Outcome {
        self.execute_reconciliation("plan", &args.target, &args.scope, &[], &[], false)
    }

    pub(crate) fn execute_sync(&self, args: &SyncArgs) -> Outcome {
        self.execute_reconciliation(
            "sync",
            &args.target,
            &args.scope,
            &args.selection.include,
            &args.selection.exclude,
            args.acknowledgment.yes,
        )
    }

    /// Run one bounded safe-update cycle. The cycle delegates each selected
    /// resource to the existing native/skill lifecycle executor without any
    /// acknowledgment selectors; pinned, disabled, drifted, or incompatible
    /// resources remain pending in their child outcome.
    pub(crate) fn execute_daemon_cycle(&self) -> Outcome {
        let command = "daemon run";
        let (documents, mut aggregate) = match self.load_documents(command) {
            Ok(value) => value,
            Err(outcome) => return *outcome,
        };
        if documents.config.updates().mode != skilltap_core::storage::UpdateMode::ApplySafe {
            aggregate = aggregate
                .with_warning(Warning::new(
                    "daemon_policy_not_apply_safe",
                    "The configured update policy does not permit automatic application.",
                ))
                .with_summary("changed", false)
                .with_summary("safe_operations", 0_u64)
                .with_summary("pending_operations", 0_u64);
            self.persist_daemon_run(&mut aggregate, 0, 0);
            return aggregate;
        }
        let mut tasks = Vec::new();
        if let Some(inventory) = documents.inventory.as_ref() {
            for resource in inventory.resources().values() {
                if resource.update() != UpdateIntent::Track {
                    continue;
                }
                let name = if resource.kind() == ResourceKind::Plugin {
                    resource.id().as_str().strip_prefix("plugin:")
                } else if resource.kind() == ResourceKind::StandaloneSkill
                    && resource
                        .source()
                        .is_some_and(|source| source.kind() == SourceKind::Git)
                {
                    resource.id().as_str().strip_prefix("skill:")
                } else {
                    None
                };
                let Some(name) = name else { continue };
                tasks.push((resource.kind(), name.to_owned(), resource.scope().clone()));
            }
        }
        let mut changed = false;
        let mut safe_operations = 0_u64;
        let mut pending_operations = 0_u64;
        for (kind, name, scope) in tasks {
            let child_scope = scope_args_for_scope(&scope);
            let child = match kind {
                ResourceKind::Plugin => self.execute_native_lifecycle(
                    command,
                    NativeLifecycleKind::PluginUpdate,
                    &child_scope,
                    &TargetArgs::default(),
                    None,
                    Some(&name),
                ),
                ResourceKind::StandaloneSkill => self.execute_skill_update(
                    command,
                    &child_scope,
                    &TargetArgs::default(),
                    Some(&name),
                ),
                _ => continue,
            };
            changed |= child.summary.get("changed") == Some(&OutputValue::Boolean(true));
            if child.result == ResultClass::Completed {
                safe_operations += child.operations.len() as u64;
            } else {
                pending_operations += 1;
            }
            aggregate.result = merge_result(aggregate.result, child.result);
            aggregate.resources.extend(child.resources);
            aggregate.operations.extend(child.operations);
            aggregate.warnings.extend(child.warnings);
            aggregate.errors.extend(child.errors);
            aggregate.next_actions.extend(child.next_actions);
        }
        aggregate = aggregate
            .with_summary("changed", changed)
            .with_summary("safe_operations", safe_operations)
            .with_summary("pending_operations", pending_operations);
        self.persist_daemon_run(&mut aggregate, safe_operations, pending_operations);
        aggregate
    }

    fn persist_daemon_run(
        &self,
        outcome: &mut Outcome,
        safe_operations: u64,
        pending_operations: u64,
    ) {
        let at = match Timestamp::from_system_time(std::time::SystemTime::now()) {
            Ok(at) => at,
            Err(_) => {
                outcome.result = merge_result(outcome.result, ResultClass::AttentionRequired);
                *outcome = outcome.clone().with_warning(Warning::new(
                    "daemon_record_failed",
                    "The daemon result timestamp could not be recorded safely.",
                ));
                return;
            }
        };
        let result = if pending_operations > 0 {
            skilltap_core::storage::DaemonRunResult::Pending
        } else if outcome.result == ResultClass::Completed {
            skilltap_core::storage::DaemonRunResult::Completed
        } else {
            skilltap_core::storage::DaemonRunResult::Failed
        };
        let failure_code = (!matches!(result, skilltap_core::storage::DaemonRunResult::Completed))
            .then(|| skilltap_core::domain::EvidenceCode::new("daemon.update_failed").unwrap());
        let record = match skilltap_core::storage::DaemonRunRecord::new(
            at,
            result,
            safe_operations,
            pending_operations,
            failure_code,
        ) {
            Ok(record) => record,
            Err(_) => {
                outcome.result = merge_result(outcome.result, ResultClass::AttentionRequired);
                *outcome = outcome.clone().with_warning(Warning::new(
                    "daemon_record_failed",
                    "The daemon result could not be represented safely.",
                ));
                return;
            }
        };
        let current = match self.state.load() {
            Ok(DocumentState::Present(state)) => state,
            Ok(DocumentState::Missing) => match StateDocument::new(
                skilltap_core::storage::STATE_SCHEMA_VERSION,
                [],
                [],
                None,
                None,
                None,
            ) {
                Ok(state) => state,
                Err(_) => {
                    outcome.result = merge_result(outcome.result, ResultClass::AttentionRequired);
                    *outcome = outcome.clone().with_warning(Warning::new(
                        "daemon_record_failed",
                        "The daemon state document could not be initialized safely.",
                    ));
                    return;
                }
            },
            Err(_) => {
                outcome.result = merge_result(outcome.result, ResultClass::AttentionRequired);
                *outcome = outcome.clone().with_warning(Warning::new(
                    "daemon_record_failed",
                    "The daemon state document could not be loaded safely.",
                ));
                return;
            }
        };
        let next = match current.with_daemon_run(record) {
            Ok(next) => next,
            Err(_) => {
                outcome.result = merge_result(outcome.result, ResultClass::AttentionRequired);
                *outcome = outcome.clone().with_warning(Warning::new(
                    "daemon_record_failed",
                    "The daemon result could not be attached to state safely.",
                ));
                return;
            }
        };
        if self.state.replace(&next).is_err() {
            outcome.result = merge_result(outcome.result, ResultClass::AttentionRequired);
            *outcome = outcome.clone().with_warning(Warning::new(
                "daemon_record_failed",
                "The daemon result could not be published atomically.",
            ));
        }
    }

    /// List desired standalone skills only. This is deliberately inventory
    /// backed and never scans source directories or marketplace contents.
    pub(crate) fn execute_skill_list(&self, args: &ScopedTargetArgs) -> Outcome {
        self.execute_resource_list("skill list", args, ResourceKind::StandaloneSkill)
    }

    pub(crate) fn execute_resource_list(
        &self,
        command: &'static str,
        args: &ScopedTargetArgs,
        kind: ResourceKind,
    ) -> Outcome {
        let (documents, mut outcome) = match self.load_documents(command) {
            Ok(value) => value,
            Err(outcome) => return *outcome,
        };
        let status_args = StatusArgs {
            target: args.target.clone(),
            scope: args.scope.clone(),
            output: OutputArgs::default(),
        };
        let scope = match StatusScope::resolve(self, &status_args, &documents) {
            Ok(scope) => scope,
            Err(error) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(error);
            }
        };
        outcome.scope = Some(scope.output.clone());
        let targets = match StatusTargets::resolve(&status_args, &documents) {
            Ok(targets) => targets,
            Err(StatusTargetError::NoneEnabled) => {
                return outcome
                    .with_error(ErrorDetail::new(
                        "no_enabled_harnesses",
                        "No harness is enabled in skilltap configuration.",
                    ))
                    .with_next_action(
                        NextAction::new("enable_harness", "Enable Codex or Claude management.")
                            .with_command("skilltap harness enable <codex|claude>"),
                    );
            }
            Err(StatusTargetError::NotEnabled) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "target_not_enabled",
                    "The requested harness target is not enabled.",
                ));
            }
        };
        let mut count = 0_u64;
        if let Some(inventory) = &documents.inventory {
            for resource in inventory.resources().values() {
                if resource.kind() != kind
                    || !scope.resolved.iter().any(|value| value == resource.scope())
                    || !resource
                        .targets()
                        .iter()
                        .any(|target| targets.resolved.contains(target))
                {
                    continue;
                }
                count += 1;
                outcome = outcome.with_resource(
                    OutputEntry::new(resource.key().to_string(), "desired")
                        .with_field("kind", format!("{:?}", kind).to_lowercase())
                        .with_field("scope", scope_label(resource.scope()))
                        .with_field("targets", resource.targets().iter().count() as u64),
                );
            }
        }
        outcome.result = ResultClass::Completed;
        outcome
            .with_summary("resources", count)
            .with_summary("scopes", scope.count)
            .with_summary("targets", targets.iter().len() as u64)
    }

    pub(crate) fn execute_instruction_status(&self, args: &ScopedOutputArgs) -> Outcome {
        let (documents, mut outcome) = match self.load_documents("instructions status") {
            Ok(value) => value,
            Err(outcome) => return *outcome,
        };
        let status_args = StatusArgs {
            target: TargetArgs::default(),
            scope: args.scope.clone(),
            output: OutputArgs::default(),
        };
        let scope = match StatusScope::resolve(self, &status_args, &documents) {
            Ok(scope) => scope,
            Err(error) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(error);
            }
        };
        outcome.scope = Some(scope.output.clone());
        let enabled = enabled_harnesses(&documents.config);
        if enabled.is_empty() {
            return outcome.with_error(ErrorDetail::new(
                "no_enabled_harnesses",
                "No harness is enabled in skilltap configuration.",
            ));
        }
        let paths = match PlatformPaths::resolve(&ProcessEnvironment) {
            Ok(paths) => paths,
            Err(_) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "platform_paths_unavailable",
                    "The skilltap configuration paths could not be resolved.",
                ));
            }
        };
        let filesystem = SystemFileSystem;
        let mode = documents.config.instructions().claude_mode;
        let mut path_count = 0_u64;
        let mut healthy = true;
        for concrete_scope in &scope.resolved {
            let (canonical, bridges) = instruction_locations(&paths, concrete_scope, &enabled);
            let canonical_status = match filesystem.inspect(&canonical) {
                Ok(metadata) => match metadata.kind() {
                    FileKind::Missing => "missing",
                    FileKind::RegularFile => "present",
                    _ => "conflict",
                },
                Err(_) => "unreadable",
            };
            path_count += 1;
            outcome = outcome.with_resource(
                OutputEntry::new(
                    instruction_resource_key(concrete_scope, "canonical", "root")
                        .map(|key| key.to_string())
                        .unwrap_or_else(|| "instructions:canonical".to_owned()),
                    canonical_status,
                )
                .with_field("path", canonical.as_str())
                .with_field("scope", scope_label(concrete_scope)),
            );
            if canonical_status != "present" {
                healthy = false;
                outcome = outcome.with_warning(
                    Warning::new(
                        "instruction_canonical_unhealthy",
                        "The canonical AGENTS.md file is missing or not a regular file.",
                    )
                    .with_context("scope", scope_label(concrete_scope)),
                );
            }
            for (target, bridge) in bridges {
                path_count += 1;
                let status = instruction_bridge_status(&filesystem, &bridge, concrete_scope, mode);
                outcome = outcome.with_resource(
                    OutputEntry::new(
                        instruction_resource_key(concrete_scope, "bridge", target.as_str())
                            .map(|key| key.to_string())
                            .unwrap_or_else(|| format!("instructions:bridge:{}", target)),
                        status,
                    )
                    .with_field("path", bridge.as_str())
                    .with_field("target", target.as_str())
                    .with_field("scope", scope_label(concrete_scope)),
                );
                if status != "managed" {
                    healthy = false;
                    outcome = outcome.with_warning(
                        Warning::new(
                            "instruction_bridge_unhealthy",
                            "The harness instruction bridge is missing or divergent.",
                        )
                        .with_context("target", target.as_str())
                        .with_context("scope", scope_label(concrete_scope)),
                    );
                }
            }
            if let Scope::Project(project) = concrete_scope
                && enabled.iter().any(|target| target.as_str() == "claude")
            {
                let nested = AbsolutePath::new(format!("{}/.claude/CLAUDE.md", project.as_str()))
                    .expect("nested project Claude bridge path is valid");
                let nested_exists = filesystem
                    .inspect(&nested)
                    .map(|metadata| metadata.kind() != FileKind::Missing)
                    .unwrap_or(false);
                if nested_exists {
                    let root = AbsolutePath::new(format!("{}/CLAUDE.md", project.as_str()))
                        .expect("project Claude bridge path is valid");
                    let root_exists = filesystem
                        .inspect(&root)
                        .map(|metadata| metadata.kind() != FileKind::Missing)
                        .unwrap_or(false);
                    path_count += 1;
                    let nested_status = instruction_bridge_status_with_target(
                        &filesystem,
                        &nested,
                        mode,
                        "../AGENTS.md",
                        b"@../AGENTS.md\n",
                    );
                    outcome = outcome.with_resource(
                        OutputEntry::new(
                            instruction_resource_key(concrete_scope, "bridge-nested", "claude")
                                .map(|key| key.to_string())
                                .unwrap_or_else(|| "instructions:bridge-nested:claude".to_owned()),
                            if root_exists {
                                "duplicate"
                            } else {
                                nested_status
                            },
                        )
                        .with_field("path", nested.as_str())
                        .with_field("target", "claude")
                        .with_field("scope", scope_label(concrete_scope)),
                    );
                    healthy = false;
                    outcome = outcome.with_warning(Warning::new(
                        "instruction_duplicate_claude_bridge",
                        if root_exists {
                            "Both project Claude instruction locations exist; consolidate to one managed bridge."
                        } else if nested_status == "managed" {
                            "The project uses the nested Claude instruction bridge; setup should preserve that location."
                        } else {
                            "The nested project Claude instruction bridge is missing or divergent."
                        },
                    )
                    .with_context("scope", scope_label(concrete_scope)));
                }
            }
        }
        if healthy {
            outcome.result = ResultClass::Completed;
        } else {
            outcome.result = ResultClass::AttentionRequired;
            outcome = outcome.with_next_action(NextAction::new(
                "repair_instruction_bridges",
                "Run instructions setup or repair after reviewing the reported paths.",
            ));
        }
        outcome
            .with_summary("scopes", scope.count)
            .with_summary("instruction_paths", path_count)
    }

    /// Render a deterministic lifecycle preview while the resource-specific
    /// mutation adapter is unavailable. This keeps command output useful and
    /// safe: it never claims a native action happened and never mutates state.
    #[allow(dead_code)]
    pub(crate) fn execute_lifecycle_preview(
        &self,
        command: &'static str,
        requested_scope: &ScopeArgs,
        target: &TargetArgs,
        source: Option<&str>,
        name: Option<&str>,
    ) -> Outcome {
        let (documents, mut outcome) = match self.load_documents(command) {
            Ok(value) => value,
            Err(outcome) => return *outcome,
        };
        let status_args = StatusArgs {
            target: target.clone(),
            scope: requested_scope.clone(),
            output: OutputArgs::default(),
        };
        let scope = match StatusScope::resolve(self, &status_args, &documents) {
            Ok(scope) => scope,
            Err(error) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(error);
            }
        };
        outcome.scope = Some(scope.output.clone());
        let targets = match StatusTargets::resolve(&status_args, &documents) {
            Ok(targets) => targets,
            Err(StatusTargetError::NoneEnabled) => {
                return outcome
                    .with_error(ErrorDetail::new(
                        "no_enabled_harnesses",
                        "No harness is enabled in skilltap configuration.",
                    ))
                    .with_next_action(
                        NextAction::new("enable_harness", "Enable Codex or Claude management.")
                            .with_command("skilltap harness enable <codex|claude>"),
                    );
            }
            Err(StatusTargetError::NotEnabled) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "target_not_enabled",
                    "The requested harness target is not enabled.",
                ));
            }
        };
        let source = source.unwrap_or("not supplied");
        let name = name.unwrap_or("derived by lifecycle adapter");
        let mut operation_count = 0_u64;
        for concrete_scope in &scope.resolved {
            for harness in targets.iter() {
                operation_count += 1;
                outcome = outcome.with_operation(
                    crate::OperationOutcome::new(
                        format!("{command}:{harness}:{}", scope_label(concrete_scope)),
                        "planned",
                    )
                    .with_field("target", harness.as_str())
                    .with_field("scope", scope_label(concrete_scope))
                    .with_field("source", source)
                    .with_field("name", name),
                );
            }
        }
        outcome
            .with_summary("operations", operation_count)
            .with_summary("changed", false)
            .with_warning(Warning::new(
                "mutation_adapter_pending",
                "The lifecycle request is planned but not applied until its native or managed adapter is available.",
            ))
            .with_next_action(NextAction::new(
                "inspect_plan",
                "Review the planned operation before the lifecycle adapter is enabled.",
        ))
    }

    /// Apply one native marketplace/plugin lifecycle request through the core
    /// lock, plan, bounded process, and state-journal boundaries.
    pub(crate) fn execute_native_lifecycle(
        &self,
        command: &'static str,
        kind: NativeLifecycleKind,
        requested_scope: &ScopeArgs,
        target: &TargetArgs,
        source_value: Option<&str>,
        name_value: Option<&str>,
    ) -> Outcome {
        let (documents, mut outcome) = match self.load_documents(command) {
            Ok(value) => value,
            Err(outcome) => return *outcome,
        };
        let status_args = StatusArgs {
            target: target.clone(),
            scope: requested_scope.clone(),
            output: OutputArgs::default(),
        };
        let scope = match StatusScope::resolve(self, &status_args, &documents) {
            Ok(scope) => scope,
            Err(error) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(error);
            }
        };
        outcome.scope = Some(scope.output.clone());
        let targets = match StatusTargets::resolve(&status_args, &documents) {
            Ok(targets) => targets,
            Err(StatusTargetError::NoneEnabled) => {
                return outcome
                    .with_error(ErrorDetail::new(
                        "no_enabled_harnesses",
                        "No harness is enabled in skilltap configuration.",
                    ))
                    .with_next_action(
                        NextAction::new("enable_harness", "Enable Codex or Claude management.")
                            .with_command("skilltap harness enable <codex|claude>"),
                    );
            }
            Err(StatusTargetError::NotEnabled) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "target_not_enabled",
                    "The requested harness target is not enabled.",
                ));
            }
        };

        let update_all = name_value.is_none()
            && matches!(
                kind,
                NativeLifecycleKind::MarketplaceUpdate | NativeLifecycleKind::PluginUpdate
            );
        let request = if update_all {
            None
        } else {
            match NativeLifecycleSpec::parse(kind, source_value, name_value) {
                Ok(request) => Some(request),
                Err(error) => {
                    outcome.result = ResultClass::Invalid;
                    return outcome.with_error(error);
                }
            }
        };
        let paths = match PlatformPaths::resolve(&ProcessEnvironment) {
            Ok(paths) => paths,
            Err(_) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "platform_paths_unavailable",
                    "The skilltap configuration paths could not be resolved.",
                ));
            }
        };
        let mut inventory = documents.inventory.clone().unwrap_or_else(|| {
            InventoryDocument::new(skilltap_core::storage::INVENTORY_SCHEMA_VERSION, [], [])
                .expect("empty inventory is valid")
        });
        let original_inventory = inventory.clone();
        let removal = matches!(
            kind,
            NativeLifecycleKind::MarketplaceRemove | NativeLifecycleKind::PluginRemove
        );
        let mut operations = Vec::new();
        let mut requests = Vec::new();
        let mut seeds = BTreeMap::new();
        let process_limits = ProcessLimits::new(5_000, 256 * 1024, 256 * 1024, 512 * 1024)
            .expect("bounded lifecycle process limits are valid");
        let json_limits =
            JsonLimits::new(256 * 1024, 64).expect("bounded lifecycle JSON limits are valid");
        let search_path = std::env::var_os("PATH");
        let timestamp = Timestamp::from_system_time(std::time::SystemTime::now()).map_err(|_| ());

        for concrete_scope in &scope.resolved {
            let scope_requests = match request.as_ref() {
                Some(request) => vec![request.clone()],
                None => inventory
                    .resources()
                    .values()
                    .filter(|resource| {
                        resource.scope() == concrete_scope
                            && resource.kind() == native_resource_kind(kind)
                    })
                    .filter_map(|resource| {
                        resource
                            .key()
                            .id()
                            .as_str()
                            .strip_prefix(native_resource_prefix(kind))
                            .and_then(|name| {
                                NativeLifecycleSpec::parse(kind, None, Some(name)).ok()
                            })
                    })
                    .collect::<Vec<_>>(),
            };
            for request in scope_requests {
                let resource = if request.is_update() {
                    let key = request.resource_key(concrete_scope).map_err(|_| {
                        ErrorDetail::new(
                            "resource_id_invalid",
                            "The requested native resource identifier is invalid.",
                        )
                    });
                    let key = match key {
                        Ok(key) => key,
                        Err(error) => {
                            outcome.result = ResultClass::Invalid;
                            return outcome.with_error(error);
                        }
                    };
                    match inventory.resources().get(&key) {
                        Some(existing) => existing.clone(),
                        None => match request.desired_resource(concrete_scope, &targets.resolved) {
                            Ok(resource) => resource,
                            Err(error) => {
                                outcome.result = ResultClass::Invalid;
                                return outcome.with_error(error);
                            }
                        },
                    }
                } else {
                    match request.desired_resource(concrete_scope, &targets.resolved) {
                        Ok(resource) => resource,
                        Err(error) => {
                            outcome.result = ResultClass::Invalid;
                            return outcome.with_error(error);
                        }
                    }
                };
                if request.retains_desired() && !request.is_update() {
                    match inventory.with_resource(resource.clone()) {
                        Ok(next) => inventory = next,
                        Err(_) => {
                            outcome.result = ResultClass::AttentionRequired;
                            return outcome
                            .with_error(ErrorDetail::new(
                                "inventory_resource_conflict",
                                "The requested resource conflicts with an existing desired definition.",
                            ))
                            .with_next_action(NextAction::new(
                                "inspect_inventory",
                                "Inspect the existing resource definition before retrying.",
                            ));
                        }
                    }
                } else if let Some(next) = inventory.without_resource(resource.key()) {
                    inventory = next;
                }

                let mut native_ids = BTreeMap::new();
                for target_id in targets.iter() {
                    let Some((harness, configured, executable, capability)) =
                        configured_native_profile(
                            &documents.config,
                            target_id,
                            concrete_scope,
                            process_limits,
                            json_limits,
                            search_path.clone(),
                            match kind {
                                NativeLifecycleKind::MarketplaceAdd => "marketplace.register",
                                NativeLifecycleKind::MarketplaceRemove => "marketplace.remove",
                                NativeLifecycleKind::MarketplaceUpdate => "marketplace.update",
                                NativeLifecycleKind::PluginInstall => "plugin.install",
                                NativeLifecycleKind::PluginRemove => "plugin.remove",
                                NativeLifecycleKind::PluginUpdate => "plugin.update",
                            },
                        )
                    else {
                        outcome.result = ResultClass::AttentionRequired;
                        outcome = outcome
                        .with_resource(
                            OutputEntry::new(
                                format!("{}:{}", target_id, resource.key()),
                                "mutation_unavailable",
                            )
                            .with_field("target", target_id.as_str())
                            .with_field("scope", scope_label(concrete_scope)),
                        )
                        .with_warning(
                            Warning::new(
                                "native_profile_unavailable",
                                "The selected harness is not mutation-authorized for this lifecycle action.",
                            )
                            .with_context("harness", target_id.as_str()),
                        );
                        continue;
                    };
                    if capability != CapabilitySupport::Supported {
                        outcome.result = ResultClass::AttentionRequired;
                        outcome = outcome.with_warning(
                            Warning::new(
                                "native_capability_unverified",
                                "The selected harness capability is not verified for mutation.",
                            )
                            .with_context("harness", target_id.as_str())
                            .with_context("scope", scope_label(concrete_scope)),
                        );
                        continue;
                    }
                    let native_request = request.native_request(harness, concrete_scope.clone());
                    let arguments = match native_arguments(&native_request) {
                        Ok(arguments) => arguments,
                        Err(_) => {
                            outcome.result = ResultClass::AttentionRequired;
                            outcome = outcome.with_warning(
                            Warning::new(
                                "native_scope_unsupported",
                                "The selected harness has no verified lifecycle command for this scope.",
                            )
                            .with_context("harness", target_id.as_str())
                            .with_context("scope", scope_label(concrete_scope)),
                        );
                            continue;
                        }
                    };
                    let operation_id = lifecycle_operation_id(kind, target_id, resource.key());
                    native_ids.insert(target_id.clone(), request.native_name.clone());
                    if previously_applied(documents.state.as_ref(), resource.key(), &operation_id) {
                        outcome = outcome.with_operation(
                            crate::OperationOutcome::new(operation_id.to_string(), "no_change")
                                .with_field("target", target_id.as_str())
                                .with_field("scope", scope_label(concrete_scope)),
                        );
                        continue;
                    }
                    let command_arguments = match command_arguments(arguments) {
                        Ok(arguments) => arguments,
                        Err(_) => {
                            outcome.result = ResultClass::Invalid;
                            return outcome.with_error(ErrorDetail::new(
                                "native_argument_encoding",
                                "The native lifecycle arguments could not be represented safely.",
                            ));
                        }
                    };
                    let operation = match native_operation(
                        operation_id.clone(),
                        target_id.clone(),
                        resource.key().clone(),
                        request.operation_action(),
                        executable,
                        command_arguments,
                    ) {
                        Ok(operation) => operation,
                        Err(_) => {
                            outcome.result = ResultClass::Invalid;
                            return outcome.with_error(ErrorDetail::new(
                                "operation_contract_invalid",
                                "The native lifecycle operation could not be constructed safely.",
                            ));
                        }
                    };
                    operations.push(operation);
                    requests.push((
                        operation_id,
                        configured,
                        search_path.clone(),
                        process_limits,
                        native_request,
                    ));
                }
                if !native_ids.is_empty() {
                    let observed_at = match timestamp {
                        Ok(timestamp) => timestamp,
                        Err(()) => {
                            outcome.result = ResultClass::Invalid;
                            return outcome.with_error(ErrorDetail::new(
                                "clock_unavailable",
                                "The operation timestamp could not be recorded safely.",
                            ));
                        }
                    };
                    let native_state = match ResourceState::new(
                        resource.key().clone(),
                        native_ids,
                        Provenance::Native,
                        Ownership::Harness,
                        resource
                            .source()
                            .cloned()
                            .or_else(|| request.source.clone()),
                        None,
                        None,
                        None,
                        None,
                        observed_at,
                        None,
                    ) {
                        Ok(state) => state,
                        Err(_) => {
                            outcome.result = ResultClass::Invalid;
                            return outcome.with_error(ErrorDetail::new(
                                "state_seed_invalid",
                                "The native lifecycle state seed was invalid.",
                            ));
                        }
                    };
                    seeds.insert(resource.key().clone(), native_state);
                }
            }
        }

        let inventory_changed = inventory != original_inventory;
        if inventory_changed && !removal && self.inventory.replace(&inventory).is_err() {
            outcome.result = ResultClass::Invalid;
            return outcome.with_error(ErrorDetail::new(
                "inventory_publish_failed",
                "The desired inventory could not be published before the native operation.",
            ));
        }
        if operations.is_empty() {
            if inventory_changed && removal && self.inventory.replace(&inventory).is_err() {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "inventory_publish_failed",
                    "The desired inventory could not be published after the native removal.",
                ));
            }
            if let Err(()) = seed_state_if_missing(self.state, &seeds) {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "state_seed_publish_failed",
                    "The native lifecycle state could not be recorded safely.",
                ));
            }
            if outcome.errors.is_empty() && outcome.warnings.is_empty() {
                outcome.result = ResultClass::Completed;
            }
            let operation_count = outcome.operations.len() as u64;
            return outcome
                .with_summary("operations", operation_count)
                .with_summary("changed", false)
                .with_next_action(NextAction::new(
                    "inspect_status",
                    "Inspect status if the native resource may have drifted externally.",
                ));
        }

        let plan = match Plan::new(operations) {
            Ok(plan) => plan,
            Err(_) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "operation_plan_invalid",
                    "The native lifecycle operation plan was invalid.",
                ));
            }
        };
        let port = NativeLifecyclePort::new_per_operation(requests);
        let journal = StateExecutionJournal {
            plan: &plan,
            state: self.state,
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
        let report =
            match execute_plan(&SystemConfigurationLock, &lock_path, &port, &journal, &plan) {
                Ok(report) => report,
                Err(error) => {
                    outcome.result = ResultClass::AttentionRequired;
                    return outcome
                    .with_error(native_execution_error(&error))
                    .with_next_action(NextAction::new(
                        "reobserve_before_retry",
                        "Re-observe the selected harness before retrying the lifecycle operation.",
                    ));
                }
            };
        let observation = NativeObservation::run(&documents, &scope, &targets);
        for resource in observation.resources.iter().cloned() {
            outcome = outcome.with_resource(resource);
        }
        for warning in observation.warnings.iter().cloned() {
            outcome = outcome.with_warning(warning);
        }
        if observation.failed_targets > 0 {
            outcome.result = ResultClass::AttentionRequired;
            outcome = outcome.with_warning(Warning::new(
                "post_mutation_observation_incomplete",
                "The native operation completed, but fresh post-mutation observation was incomplete.",
            ));
        }
        for result in report.result.operations().values() {
            let status = operation_result_status(result.outcome());
            outcome = outcome.with_operation(crate::OperationOutcome::new(
                result.operation_id().to_string(),
                status,
            ));
            if matches!(
                result.outcome(),
                OperationOutcome::Failed { .. }
                    | OperationOutcome::Blocked { .. }
                    | OperationOutcome::SkippedDependency { .. }
                    | OperationOutcome::Pending
            ) {
                outcome.result = ResultClass::AttentionRequired;
            }
        }
        let successful = report.result.operations().values().all(|result| {
            matches!(
                result.outcome(),
                OperationOutcome::Applied | OperationOutcome::NoChange
            )
        });
        if removal && inventory_changed && successful && self.inventory.replace(&inventory).is_err()
        {
            outcome.result = ResultClass::Invalid;
            return outcome.with_error(ErrorDetail::new(
                "inventory_publish_failed",
                "The desired inventory could not be published after the native removal.",
            ));
        }
        if report.changed && observation.failed_targets == 0 && successful {
            outcome.result = ResultClass::Completed;
        }
        outcome
            .with_summary("operations", report.result.operations().len() as u64)
            .with_summary("changed", report.changed)
            .with_next_action(NextAction::new(
                "verify_status",
                "Run status to verify the fresh native observation and recorded state.",
            ))
    }

    /// Install an explicit local complete skill tree into native skill paths.
    /// Git-backed sources deliberately remain a separate adapter until their
    /// clone/resolve boundary is available.
    pub(crate) fn execute_skill_install(
        &self,
        command: &'static str,
        requested_scope: &ScopeArgs,
        target: &TargetArgs,
        request: SkillInstallRequest<'_>,
    ) -> Outcome {
        let (documents, mut outcome) = match self.load_documents(command) {
            Ok(value) => value,
            Err(outcome) => return *outcome,
        };
        let status_args = StatusArgs {
            target: target.clone(),
            scope: requested_scope.clone(),
            output: OutputArgs::default(),
        };
        let scope = match StatusScope::resolve(self, &status_args, &documents) {
            Ok(scope) => scope,
            Err(error) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(error);
            }
        };
        outcome.scope = Some(scope.output.clone());
        let targets = match StatusTargets::resolve(&status_args, &documents) {
            Ok(targets) => targets,
            Err(StatusTargetError::NoneEnabled) => {
                return outcome.with_error(ErrorDetail::new(
                    "no_enabled_harnesses",
                    "No harness is enabled in skilltap configuration.",
                ));
            }
            Err(StatusTargetError::NotEnabled) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "target_not_enabled",
                    "The requested harness target is not enabled.",
                ));
            }
        };
        let locator = match SourceLocator::new(request.source) {
            Ok(locator) => locator,
            Err(_) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "invalid_skill_source",
                    "The explicit skill source is invalid.",
                ));
            }
        };
        let requested_revision = match request.requested_revision {
            Some(value) => match skilltap_core::domain::RequestedRevision::new(value) {
                Ok(value) => Some(value),
                Err(_) => {
                    outcome.result = ResultClass::Invalid;
                    return outcome.with_error(ErrorDetail::new(
                        "invalid_requested_revision",
                        "The requested Git revision is invalid.",
                    ));
                }
            },
            None => None,
        };
        let subdirectory = match request.subdirectory {
            Some(value) => match skilltap_core::domain::RelativeArtifactPath::new(value) {
                Ok(value) => Some(value),
                Err(_) => {
                    outcome.result = ResultClass::Invalid;
                    return outcome.with_error(ErrorDetail::new(
                        "invalid_skill_subdirectory",
                        "The skill source subdirectory is invalid.",
                    ));
                }
            },
            None => None,
        };
        let (source_root, source_kind, git_commit) = match AbsolutePath::new(locator.as_str()) {
            Ok(path) => match append_skill_subdirectory(path, subdirectory.as_ref()) {
                Some(path) => (path, SourceKind::Local, None),
                None => {
                    outcome.result = ResultClass::Invalid;
                    return outcome.with_error(ErrorDetail::new(
                        "invalid_skill_subdirectory",
                        "The skill source subdirectory could not be joined safely.",
                    ));
                }
            },
            Err(_) => {
                let paths = match PlatformPaths::resolve(&ProcessEnvironment) {
                    Ok(paths) => paths,
                    Err(_) => {
                        outcome.result = ResultClass::Invalid;
                        return outcome.with_error(ErrorDetail::new(
                            "platform_paths_unavailable",
                            "The skilltap configuration paths could not be resolved.",
                        ));
                    }
                };
                match resolve_git_skill_source(
                    &paths,
                    &locator,
                    requested_revision.as_ref(),
                    subdirectory.as_ref(),
                ) {
                    Ok(resolved) => (resolved.root, SourceKind::Git, Some(resolved.commit)),
                    Err(_) => {
                        outcome.result = ResultClass::AttentionRequired;
                        return outcome
                            .with_warning(Warning::new(
                                "git_skill_source_unavailable",
                                "The Git skill source could not be cloned and checked out safely.",
                            ))
                            .with_next_action(NextAction::new(
                                "verify_git_source",
                                "Verify the Git source, revision, and credentials before retrying.",
                            ));
                    }
                }
            }
        };
        let limits =
            ExternalTreeLimits::new(64, 100_000, 64 * 1024 * 1024, 1024 * 1024 * 1024, 64 * 1024)
                .expect("bounded skill tree limits are valid");
        let filesystem = SystemFileSystem;
        let source_snapshot = match SystemExternalTreeObserver
            .observe(&ExternalTreeRequest::new(source_root.clone(), limits))
            .and_then(|snapshot| snapshot.without_top_level_directory(".git", limits))
        {
            Ok(snapshot) => snapshot,
            Err(_) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "skill_source_unavailable",
                    "The explicit local skill directory could not be observed safely.",
                ));
            }
        };
        let skill = match ValidatedSkillTree::validate(&source_snapshot) {
            Ok(skill) => skill,
            Err(_) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "skill_tree_invalid",
                    "The skill source must be a complete directory with a top-level SKILL.md.",
                ));
            }
        };
        for compatibility in SkillCompatibility::evaluate(&skill, &targets.resolved) {
            match compatibility.class() {
                SkillCompatibilityClass::Blocked => {
                    outcome.result = ResultClass::AttentionRequired;
                    return outcome.with_warning(
                        Warning::new(
                            "skill_incompatible",
                            "The skill frontmatter is not loadable by the selected harness.",
                        )
                        .with_context("harness", compatibility.target().as_str()),
                    );
                }
                SkillCompatibilityClass::Warning => {
                    outcome = outcome.with_warning(
                        Warning::new(
                            "skill_frontmatter_warning",
                            "The skill is loadable but its frontmatter is not fully strict.",
                        )
                        .with_context("harness", compatibility.target().as_str()),
                    );
                }
                SkillCompatibilityClass::Compatible => {}
            }
        }
        let name = match request.name {
            Some(name) => NativeId::new(name).map_err(|_| ()).ok(),
            None => derive_skill_name(&locator, subdirectory.as_ref()),
        };
        let Some(name) = name else {
            outcome.result = ResultClass::Invalid;
            return outcome.with_error(ErrorDetail::new(
                "skill_name_required",
                "The skill name could not be derived; provide --name.",
            ));
        };
        if request.name.is_some() && skill.declared_name().as_ref() != Some(&name) {
            outcome.result = ResultClass::Invalid;
            return outcome.with_error(ErrorDetail::new(
                "skill_name_mismatch",
                "The supplied --name must match the SKILL.md frontmatter name.",
            ));
        }
        let destination = match skill_relative_destination(&name) {
            Some(destination) => destination,
            None => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "skill_name_invalid",
                    "The skill name cannot be used as a safe directory component.",
                ));
            }
        };
        let paths = match PlatformPaths::resolve(&ProcessEnvironment) {
            Ok(paths) => paths,
            Err(_) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "platform_paths_unavailable",
                    "The skilltap configuration paths could not be resolved.",
                ));
            }
        };
        let source = Source::new_with_subdirectory(
            source_kind,
            locator,
            requested_revision,
            subdirectory.clone(),
        )
        .map_err(|_| ())
        .ok();
        let Some(source) = source else {
            outcome.result = ResultClass::Invalid;
            return outcome.with_error(ErrorDetail::new(
                "invalid_skill_source",
                "The local skill source could not be represented safely.",
            ));
        };
        let update_intent = if source
            .requested_revision()
            .and_then(|revision| GitCommit::new(revision.as_str()).ok())
            .is_some()
        {
            UpdateIntent::Pinned
        } else {
            UpdateIntent::Track
        };
        let mut inventory = documents.inventory.clone().unwrap_or_else(|| {
            InventoryDocument::new(skilltap_core::storage::INVENTORY_SCHEMA_VERSION, [], [])
                .expect("empty inventory is valid")
        });
        let mut operations = Vec::new();
        let mut entries = BTreeMap::new();
        let mut seeds = BTreeMap::new();
        let timestamp = match Timestamp::from_system_time(std::time::SystemTime::now()) {
            Ok(timestamp) => timestamp,
            Err(_) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "clock_unavailable",
                    "The skill operation timestamp could not be recorded safely.",
                ));
            }
        };
        for concrete_scope in &scope.resolved {
            let key = ResourceKey::new(
                match ResourceId::new(format!("skill:{}", name.as_str())) {
                    Ok(id) => id,
                    Err(_) => {
                        outcome.result = ResultClass::Invalid;
                        return outcome.with_error(ErrorDetail::new(
                            "skill_name_invalid",
                            "The skill name cannot be represented as a resource identifier.",
                        ));
                    }
                },
                concrete_scope.clone(),
            );
            let desired = match DesiredResource::new(
                key.clone(),
                ResourceKind::StandaloneSkill,
                targets.resolved.clone(),
                DesiredOrigin::Direct,
                Some(source.clone()),
                update_intent,
                ComponentGraph::new([]).expect("empty component graph is valid"),
                BTreeMap::new(),
                BTreeMap::new(),
                BTreeSet::new(),
            ) {
                Ok(desired) => desired,
                Err(_) => {
                    outcome.result = ResultClass::Invalid;
                    return outcome.with_error(ErrorDetail::new(
                        "skill_resource_invalid",
                        "The skill resource could not be represented safely.",
                    ));
                }
            };
            inventory = match inventory.with_resource(desired) {
                Ok(inventory) => inventory,
                Err(_) => {
                    outcome.result = ResultClass::AttentionRequired;
                    return outcome.with_error(ErrorDetail::new(
                        "inventory_resource_conflict",
                        "The requested skill conflicts with an existing desired definition.",
                    ));
                }
            };
            let mut native_ids = BTreeMap::new();
            let destinations =
                match skill_destinations(&paths, concrete_scope, &targets.resolved, &destination) {
                    Some(destinations) => destinations,
                    None => {
                        outcome.result = ResultClass::Invalid;
                        return outcome.with_error(ErrorDetail::new(
                            "skill_destination_invalid",
                            "The selected harness skill destination could not be resolved.",
                        ));
                    }
                };
            for destination_entry in destinations {
                let SkillDestination {
                    target,
                    canonical,
                    root,
                    full_path,
                } = destination_entry;
                let target_id = &target;
                let current = match SystemExternalTreeObserver
                    .observe(&ExternalTreeRequest::new(full_path.clone(), limits))
                {
                    Ok(snapshot) => match ValidatedSkillTree::validate(&snapshot) {
                        Ok(current) => Some(current),
                        Err(_) => {
                            outcome.result = ResultClass::AttentionRequired;
                            outcome = outcome.with_warning(
                                Warning::new(
                                    "skill_destination_invalid",
                                    "An existing skill destination is not a valid complete skill tree.",
                                )
                                .with_context("target", target_id.as_str())
                                .with_context("scope", scope_label(concrete_scope)),
                            );
                            None
                        }
                    },
                    Err(_) => None,
                };
                if let Some(current) = current {
                    if current.fingerprint() == skill.fingerprint() {
                        outcome = outcome.with_operation(crate::OperationOutcome::new(
                            format!("skill:{}:{}", target_id, name),
                            "no_change",
                        ));
                        native_ids.insert(target_id.clone(), name.clone());
                        continue;
                    }
                    let managed_fingerprint = documents
                        .state
                        .as_ref()
                        .and_then(|state| state.resources().get(&key))
                        .and_then(|state| state.fingerprint());
                    if managed_fingerprint != Some(current.fingerprint()) {
                        outcome.result = ResultClass::AttentionRequired;
                        outcome = outcome.with_warning(
                            Warning::new(
                                "skill_destination_drifted",
                                "The installed skill has local drift; no replacement was made. `--yes` does not override unidentified edits.",
                            )
                            .with_context("target", target_id.as_str())
                            .with_context("scope", scope_label(concrete_scope)),
                        );
                        continue;
                    }
                    if command != "skill update" {
                        outcome.result = ResultClass::AttentionRequired;
                        outcome = outcome.with_warning(
                            Warning::new(
                                "skill_update_required",
                                "The source changed while the installed tree is intact; use `skill update <name>` to replace it explicitly.",
                            )
                            .with_context("target", target_id.as_str())
                            .with_context("scope", scope_label(concrete_scope)),
                        );
                        continue;
                    }
                    let (identity, _) = match filesystem.load_tree_no_follow(&root, &destination) {
                        Ok(value) => value,
                        Err(_) => {
                            outcome.result = ResultClass::AttentionRequired;
                            return outcome.with_warning(Warning::new(
                                "skill_destination_changed",
                                "The skill destination changed before a safe replacement could be planned.",
                            ));
                        }
                    };
                    let operation_id = if canonical {
                        skill_canonical_operation_id(&key)
                    } else {
                        skill_operation_id(target_id, &key)
                    };
                    let operation =
                        match skilltap_core::lifecycle_operation::faithful_file_operation(
                            operation_id.clone(),
                            target_id.clone(),
                            key.clone(),
                            OperationAction::SkillInstall,
                            full_path,
                        ) {
                            Ok(operation) => operation,
                            Err(_) => {
                                outcome.result = ResultClass::Invalid;
                                return outcome.with_error(ErrorDetail::new(
                                "operation_contract_invalid",
                                "The managed skill replacement operation could not be constructed safely.",
                            ));
                            }
                        };
                    operations.push(operation);
                    entries.insert(
                        operation_id,
                        ManagedSkillEntry {
                            root,
                            destination: destination.clone(),
                            tree: skill.tree().clone(),
                            backup_tree: Some(current.tree().clone()),
                            action: ManagedSkillAction::Replace,
                            expected_identity: Some(identity),
                            owner: Some(key.clone()),
                            config_root: Some(paths.skilltap_config().clone()),
                        },
                    );
                    native_ids.insert(target_id.clone(), name.clone());
                    continue;
                }
                let operation_id = if canonical {
                    skill_canonical_operation_id(&key)
                } else {
                    skill_operation_id(target_id, &key)
                };
                let operation = match skilltap_core::lifecycle_operation::faithful_file_operation(
                    operation_id.clone(),
                    target_id.clone(),
                    key.clone(),
                    OperationAction::SkillInstall,
                    full_path,
                ) {
                    Ok(operation) => operation,
                    Err(_) => {
                        outcome.result = ResultClass::Invalid;
                        return outcome.with_error(ErrorDetail::new(
                            "operation_contract_invalid",
                            "The managed skill operation could not be constructed safely.",
                        ));
                    }
                };
                operations.push(operation);
                entries.insert(
                    operation_id,
                    ManagedSkillEntry {
                        root,
                        destination: destination.clone(),
                        tree: skill.tree().clone(),
                        backup_tree: None,
                        action: ManagedSkillAction::Install,
                        expected_identity: None,
                        owner: None,
                        config_root: None,
                    },
                );
                native_ids.insert(target_id.clone(), name.clone());
            }
            if !native_ids.is_empty() {
                let state = match ResourceState::new(
                    key.clone(),
                    native_ids,
                    Provenance::Direct,
                    Ownership::Skilltap,
                    Some(source.clone()),
                    None,
                    Some(skill.fingerprint().clone()),
                    git_commit
                        .clone()
                        .map(skilltap_core::domain::ResolvedRevision::GitCommit),
                    None,
                    timestamp,
                    None,
                ) {
                    Ok(state) => state,
                    Err(_) => {
                        outcome.result = ResultClass::Invalid;
                        return outcome.with_error(ErrorDetail::new(
                            "state_seed_invalid",
                            "The standalone skill state seed was invalid.",
                        ));
                    }
                };
                seeds.insert(key, state);
            }
        }
        let empty_inventory = documents.inventory.clone().unwrap_or_else(|| {
            InventoryDocument::new(skilltap_core::storage::INVENTORY_SCHEMA_VERSION, [], [])
                .expect("empty inventory is valid")
        });
        if inventory != empty_inventory && self.inventory.replace(&inventory).is_err() {
            outcome.result = ResultClass::Invalid;
            return outcome.with_error(ErrorDetail::new(
                "inventory_publish_failed",
                "The skill inventory could not be published before installation.",
            ));
        }
        if operations.is_empty() {
            if let Err(()) = seed_state_if_missing(self.state, &seeds) {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "state_seed_publish_failed",
                    "The standalone skill state could not be recorded safely.",
                ));
            }
            if outcome.errors.is_empty() && outcome.warnings.is_empty() {
                outcome.result = ResultClass::Completed;
            }
            let operation_count = outcome.operations.len() as u64;
            return outcome
                .with_summary("operations", operation_count)
                .with_summary("changed", false);
        }
        let plan = match Plan::new(operations) {
            Ok(plan) => plan,
            Err(_) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "operation_plan_invalid",
                    "The standalone skill operation plan was invalid.",
                ));
            }
        };
        let port = ManagedSkillPort {
            filesystem: &filesystem,
            entries,
        };
        let journal = StateExecutionJournal {
            plan: &plan,
            state: self.state,
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
        let report =
            match execute_plan(&SystemConfigurationLock, &lock_path, &port, &journal, &plan) {
                Ok(report) => report,
                Err(error) => {
                    outcome.result = ResultClass::AttentionRequired;
                    return outcome.with_error(native_execution_error(&error));
                }
            };
        for result in report.result.operations().values() {
            outcome = outcome.with_operation(crate::OperationOutcome::new(
                result.operation_id().to_string(),
                operation_result_status(result.outcome()),
            ));
            if !matches!(
                result.outcome(),
                OperationOutcome::Applied | OperationOutcome::NoChange
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

    /// Refresh a managed skill from the source recorded in state. The normal
    /// install path performs the bounded resolution and replacement planning;
    /// this command only supplies the recorded source identity.
    pub(crate) fn execute_skill_update(
        &self,
        command: &'static str,
        requested_scope: &ScopeArgs,
        target: &TargetArgs,
        skill_name: Option<&str>,
    ) -> Outcome {
        let (documents, _) = match self.load_documents(command) {
            Ok(value) => value,
            Err(outcome) => return *outcome,
        };
        let status_args = StatusArgs {
            target: target.clone(),
            scope: requested_scope.clone(),
            output: OutputArgs::default(),
        };
        let scope = match StatusScope::resolve(self, &status_args, &documents) {
            Ok(scope) => scope,
            Err(error) => {
                return Outcome::new(command, ResultClass::Invalid).with_error(error);
            }
        };

        let Some(skill_name) = skill_name else {
            let candidates = documents
                .inventory
                .as_ref()
                .into_iter()
                .flat_map(|inventory| inventory.resources().values())
                .filter(|resource| {
                    resource.kind() == ResourceKind::StandaloneSkill
                        && scope
                            .resolved
                            .iter()
                            .any(|selected| selected == resource.scope())
                        && resource
                            .targets()
                            .iter()
                            .any(|selected| match target.target.as_ref() {
                                None | Some(skilltap_core::domain::TargetSelection::All) => true,
                                Some(skilltap_core::domain::TargetSelection::Only(requested)) => {
                                    requested == selected
                                }
                            })
                })
                .filter_map(|resource| {
                    let name = resource
                        .id()
                        .as_str()
                        .strip_prefix("skill:")
                        .and_then(|value| NativeId::new(value).ok())?;
                    let source = documents
                        .state
                        .as_ref()
                        .and_then(|state| state.resources().get(resource.key()))
                        .and_then(|state| state.source())
                        .cloned();
                    Some((name, resource.scope().clone(), source))
                })
                .collect::<Vec<_>>();
            if candidates.is_empty() {
                return Outcome::new(command, ResultClass::Completed)
                    .with_scope(scope.output)
                    .with_summary("operations", 0_u64)
                    .with_summary("changed", false);
            }
            let mut aggregate = Outcome::new(command, ResultClass::Completed)
                .with_scope(scope.output)
                .with_summary("operations", 0_u64)
                .with_summary("changed", false);
            for (name, concrete_scope, source) in candidates {
                if source.is_none() {
                    aggregate.result =
                        merge_result(aggregate.result, ResultClass::AttentionRequired);
                    aggregate = aggregate.with_warning(
                        Warning::new(
                            "skill_source_unavailable",
                            "A selected skill has no recorded source; adopt or install it before updating.",
                        )
                        .with_context("skill", name.as_str())
                        .with_context("scope", scope_label(&concrete_scope)),
                    );
                    continue;
                }
                let child_scope = scope_args_for_scope(&concrete_scope);
                let child =
                    self.execute_skill_update(command, &child_scope, target, Some(name.as_str()));
                let child_changed =
                    child.summary.get("changed") == Some(&OutputValue::Boolean(true));
                let child_operations = child.operations.len() as u64;
                aggregate.result = merge_result(aggregate.result, child.result);
                aggregate.summary.insert(
                    "operations".to_owned(),
                    OutputValue::Unsigned(
                        aggregate
                            .summary
                            .get("operations")
                            .and_then(|value| match value {
                                OutputValue::Unsigned(value) => Some(*value),
                                _ => None,
                            })
                            .unwrap_or_default()
                            + child_operations,
                    ),
                );
                if child_changed {
                    aggregate
                        .summary
                        .insert("changed".to_owned(), OutputValue::Boolean(true));
                }
                aggregate.resources.extend(child.resources);
                aggregate.operations.extend(child.operations);
                aggregate.warnings.extend(child.warnings);
                aggregate.errors.extend(child.errors);
                aggregate.next_actions.extend(child.next_actions);
            }
            return aggregate;
        };
        let name = match NativeId::new(skill_name) {
            Ok(name) => name,
            Err(_) => {
                return Outcome::new(command, ResultClass::Invalid).with_error(ErrorDetail::new(
                    "skill_name_invalid",
                    "The skill name is not a valid managed resource identifier.",
                ));
            }
        };
        let mut source = None;
        for concrete_scope in &scope.resolved {
            let Some(key) = ResourceId::new(format!("skill:{}", name.as_str()))
                .ok()
                .map(|id| ResourceKey::new(id, concrete_scope.clone()))
            else {
                continue;
            };
            if let Some(value) = documents
                .state
                .as_ref()
                .and_then(|state| state.resources().get(&key))
                .and_then(|state| state.source())
            {
                source = Some(value.clone());
                break;
            }
        }
        let Some(source) = source else {
            return Outcome::new(command, ResultClass::AttentionRequired).with_warning(
                Warning::new(
                    "skill_source_unavailable",
                    "The selected skill has no recorded source; adopt or install it before updating.",
                ),
            );
        };
        self.execute_skill_install(
            command,
            requested_scope,
            target,
            SkillInstallRequest {
                source: source.locator().as_str(),
                name: None,
                requested_revision: source.requested_revision().map(|value| value.as_str()),
                subdirectory: source.subdirectory().map(|value| value.as_str()),
            },
        )
    }

    /// Remove a skill only when skilltap owns the exact current tree. An
    /// unmanaged or drifted tree is left untouched and reported for an
    /// explicit repair decision.
    pub(crate) fn execute_skill_remove(
        &self,
        command: &'static str,
        requested_scope: &ScopeArgs,
        target: &TargetArgs,
        skill_name: &str,
        acknowledged: bool,
    ) -> Outcome {
        let (documents, mut outcome) = match self.load_documents(command) {
            Ok(value) => value,
            Err(outcome) => return *outcome,
        };
        let name = match NativeId::new(skill_name) {
            Ok(name) => name,
            Err(_) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "skill_name_invalid",
                    "The skill name is not a valid managed resource identifier.",
                ));
            }
        };
        let status_args = StatusArgs {
            target: target.clone(),
            scope: requested_scope.clone(),
            output: OutputArgs::default(),
        };
        let scope = match StatusScope::resolve(self, &status_args, &documents) {
            Ok(scope) => scope,
            Err(error) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(error);
            }
        };
        outcome.scope = Some(scope.output.clone());
        let targets = match StatusTargets::resolve(&status_args, &documents) {
            Ok(targets) => targets,
            Err(StatusTargetError::NoneEnabled) => {
                return outcome.with_error(ErrorDetail::new(
                    "no_enabled_harnesses",
                    "No harness is enabled in skilltap configuration.",
                ));
            }
            Err(StatusTargetError::NotEnabled) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "target_not_enabled",
                    "The requested harness target is not enabled.",
                ));
            }
        };
        let paths = match PlatformPaths::resolve(&ProcessEnvironment) {
            Ok(paths) => paths,
            Err(_) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "platform_paths_unavailable",
                    "The skilltap configuration paths could not be resolved.",
                ));
            }
        };
        let destination = match skill_relative_destination(&name) {
            Some(destination) => destination,
            None => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "skill_name_invalid",
                    "The skill name cannot be used as a safe directory component.",
                ));
            }
        };
        let filesystem = SystemFileSystem;
        let limits =
            ExternalTreeLimits::new(64, 100_000, 64 * 1024 * 1024, 1024 * 1024 * 1024, 64 * 1024)
                .expect("bounded skill tree limits are valid");
        let mut inventory = documents.inventory.clone().unwrap_or_else(|| {
            InventoryDocument::new(skilltap_core::storage::INVENTORY_SCHEMA_VERSION, [], [])
                .expect("empty inventory is valid")
        });
        let mut operations = Vec::new();
        let mut entries = BTreeMap::new();
        let seeds = BTreeMap::new();
        for concrete_scope in &scope.resolved {
            let Some(key) = ResourceId::new(format!("skill:{}", name.as_str()))
                .ok()
                .map(|id| ResourceKey::new(id, concrete_scope.clone()))
            else {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "skill_resource_invalid",
                    "The standalone skill resource could not be represented safely.",
                ));
            };
            let Some(state) = documents
                .state
                .as_ref()
                .and_then(|document| document.resources().get(&key))
            else {
                outcome.result = ResultClass::AttentionRequired;
                outcome = outcome.with_warning(
                    Warning::new(
                        "skill_not_managed",
                        "The requested skill has no skilltap ownership record; no files were removed.",
                    )
                    .with_context("scope", scope_label(concrete_scope)),
                );
                continue;
            };
            if state.ownership() != Ownership::Skilltap {
                outcome.result = ResultClass::AttentionRequired;
                outcome = outcome.with_warning(
                    Warning::new(
                        "skill_not_owned",
                        "The requested skill is not owned by skilltap; no files were removed.",
                    )
                    .with_context("scope", scope_label(concrete_scope)),
                );
                continue;
            }
            let destinations =
                match skill_destinations(&paths, concrete_scope, &targets.resolved, &destination) {
                    Some(destinations) => destinations,
                    None => {
                        outcome.result = ResultClass::Invalid;
                        return outcome.with_error(ErrorDetail::new(
                            "skill_destination_invalid",
                            "The selected harness skill destination could not be resolved.",
                        ));
                    }
                };
            for destination_entry in destinations {
                let SkillDestination {
                    target,
                    canonical,
                    root,
                    full_path,
                } = destination_entry;
                let target_id = &target;
                let snapshot = match SystemExternalTreeObserver
                    .observe(&ExternalTreeRequest::new(full_path.clone(), limits))
                {
                    Ok(snapshot) => snapshot,
                    Err(_) => {
                        outcome = outcome.with_operation(crate::OperationOutcome::new(
                            format!("skill:{target_id}:{}", name.as_str()),
                            "no_change",
                        ));
                        continue;
                    }
                };
                let current = match ValidatedSkillTree::validate(&snapshot) {
                    Ok(current) => current,
                    Err(_) => {
                        outcome.result = ResultClass::AttentionRequired;
                        outcome = outcome.with_warning(
                            Warning::new(
                                "skill_destination_invalid",
                                "The existing skill destination is not a complete skill tree; no files were removed.",
                            )
                            .with_context("target", target_id.as_str()),
                        );
                        continue;
                    }
                };
                if state.fingerprint() != Some(current.fingerprint()) {
                    outcome.result = ResultClass::AttentionRequired;
                    outcome = outcome.with_warning(
                        Warning::new(
                            if acknowledged {
                                "skill_destination_drifted"
                            } else {
                                "skill_destination_drifted_requires_acknowledgment"
                            },
                            "The current skill tree differs from the skilltap-owned fingerprint; no files were removed.",
                        )
                        .with_context("target", target_id.as_str())
                        .with_context("scope", scope_label(concrete_scope)),
                    );
                    continue;
                }
                let (identity, files) = match filesystem.load_tree_no_follow(&root, &destination) {
                    Ok(value) => value,
                    Err(_) => continue,
                };
                let tree = match ArtifactTree::new(
                    files
                        .into_iter()
                        .map(|(path, bytes)| (path.as_str().to_owned(), bytes)),
                ) {
                    Ok(tree) => tree,
                    Err(_) => continue,
                };
                let operation_id = if canonical {
                    skill_canonical_remove_operation_id(&key)
                } else {
                    skill_remove_operation_id(target_id, &key)
                };
                let operation = match skilltap_core::lifecycle_operation::faithful_file_operation(
                    operation_id.clone(),
                    target_id.clone(),
                    key.clone(),
                    OperationAction::SkillRemove,
                    full_path,
                ) {
                    Ok(operation) => operation,
                    Err(_) => {
                        outcome.result = ResultClass::Invalid;
                        return outcome.with_error(ErrorDetail::new(
                            "operation_contract_invalid",
                            "The managed skill removal operation could not be constructed safely.",
                        ));
                    }
                };
                operations.push(operation);
                entries.insert(
                    operation_id,
                    ManagedSkillEntry {
                        root,
                        destination: destination.clone(),
                        tree,
                        backup_tree: None,
                        action: ManagedSkillAction::Remove,
                        expected_identity: Some(identity),
                        owner: None,
                        config_root: None,
                    },
                );
            }
            if inventory.resources().contains_key(&key) {
                inventory = inventory.without_resource(&key).unwrap_or(inventory);
            }
        }
        let empty_inventory = documents.inventory.clone().unwrap_or_else(|| {
            InventoryDocument::new(skilltap_core::storage::INVENTORY_SCHEMA_VERSION, [], [])
                .expect("empty inventory is valid")
        });
        if inventory != empty_inventory && self.inventory.replace(&inventory).is_err() {
            outcome.result = ResultClass::Invalid;
            return outcome.with_error(ErrorDetail::new(
                "inventory_publish_failed",
                "The skill inventory could not be updated safely.",
            ));
        }
        if operations.is_empty() {
            if outcome.errors.is_empty() && outcome.warnings.is_empty() {
                outcome.result = ResultClass::Completed;
            }
            let operation_count = outcome.operations.len() as u64;
            return outcome
                .with_summary("operations", operation_count)
                .with_summary("changed", false);
        }
        let plan = match Plan::new(operations) {
            Ok(plan) => plan,
            Err(_) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "operation_plan_invalid",
                    "The managed skill removal plan was invalid.",
                ));
            }
        };
        let port = ManagedSkillPort {
            filesystem: &filesystem,
            entries,
        };
        let journal = StateExecutionJournal {
            plan: &plan,
            state: self.state,
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
        let report =
            match execute_plan(&SystemConfigurationLock, &lock_path, &port, &journal, &plan) {
                Ok(report) => report,
                Err(error) => {
                    outcome.result = ResultClass::AttentionRequired;
                    return outcome.with_error(native_execution_error(&error));
                }
            };
        for result in report.result.operations().values() {
            outcome = outcome.with_operation(crate::OperationOutcome::new(
                result.operation_id().to_string(),
                operation_result_status(result.outcome()),
            ));
            if !matches!(
                result.outcome(),
                OperationOutcome::Applied | OperationOutcome::NoChange
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

    pub(crate) fn execute_instruction_setup(
        &self,
        command: &'static str,
        requested_scope: &ScopeArgs,
        mode: Option<ClaudeInstructionMode>,
        acknowledged: bool,
        repair: bool,
    ) -> Outcome {
        let (documents, mut outcome) = match self.load_documents(command) {
            Ok(value) => value,
            Err(outcome) => return *outcome,
        };
        let status_args = StatusArgs {
            target: TargetArgs::default(),
            scope: requested_scope.clone(),
            output: OutputArgs::default(),
        };
        let scope = match StatusScope::resolve(self, &status_args, &documents) {
            Ok(scope) => scope,
            Err(error) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(error);
            }
        };
        outcome.scope = Some(scope.output.clone());
        let enabled = enabled_harnesses(&documents.config);
        if enabled.is_empty() {
            outcome.result = ResultClass::AttentionRequired;
            return outcome.with_error(ErrorDetail::new(
                "no_enabled_harnesses",
                "No harness is enabled in skilltap configuration.",
            ));
        }
        let mode = mode.unwrap_or(documents.config.instructions().claude_mode);
        let paths = match PlatformPaths::resolve(&ProcessEnvironment) {
            Ok(paths) => paths,
            Err(_) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "platform_paths_unavailable",
                    "The skilltap configuration paths could not be resolved.",
                ));
            }
        };
        let filesystem = SystemFileSystem;
        let mut inventory = documents.inventory.clone().unwrap_or_else(|| {
            InventoryDocument::new(skilltap_core::storage::INVENTORY_SCHEMA_VERSION, [], [])
                .expect("empty inventory is valid")
        });
        let mut operations = Vec::new();
        let mut entries = BTreeMap::new();
        let mut seeds = BTreeMap::new();
        let timestamp = match Timestamp::from_system_time(std::time::SystemTime::now()) {
            Ok(timestamp) => timestamp,
            Err(_) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "clock_unavailable",
                    "The instruction operation timestamp could not be recorded safely.",
                ));
            }
        };
        for concrete_scope in &scope.resolved {
            let (canonical, mut bridges) = instruction_locations(&paths, concrete_scope, &enabled);
            let mut duplicate_nested = None;
            if let Scope::Project(project) = concrete_scope
                && enabled.iter().any(|target| target.as_str() == "claude")
            {
                let root = AbsolutePath::new(format!("{}/CLAUDE.md", project.as_str()))
                    .expect("project Claude bridge path is valid");
                let nested = AbsolutePath::new(format!("{}/.claude/CLAUDE.md", project.as_str()))
                    .expect("nested project Claude bridge path is valid");
                let root_missing = filesystem
                    .inspect(&root)
                    .map(|metadata| metadata.kind() == FileKind::Missing)
                    .unwrap_or(false);
                let nested_present = filesystem
                    .inspect(&nested)
                    .map(|metadata| metadata.kind() != FileKind::Missing)
                    .unwrap_or(false);
                if !root_missing && nested_present {
                    let nested_kind = filesystem
                        .inspect(&nested)
                        .ok()
                        .map(|metadata| metadata.kind());
                    if !matches!(nested_kind, Some(FileKind::RegularFile | FileKind::Symlink)) {
                        outcome.result = ResultClass::AttentionRequired;
                        outcome = outcome
                            .with_warning(
                                Warning::new(
                                    "instruction_duplicate_bridge_broken",
                                    "The nested project Claude entry is not a removable regular file or symlink; consolidation is blocked.",
                                )
                                .with_context("scope", scope_label(concrete_scope)),
                            )
                            .with_next_action(NextAction::new(
                                "repair_duplicate_bridge_manually",
                                "Replace the broken nested Claude entry with a regular file or symlink, then retry repair.",
                            ));
                        continue;
                    }
                    duplicate_nested = Some(nested.clone());
                    if !(repair && acknowledged) {
                        outcome.result = ResultClass::AttentionRequired;
                        outcome = outcome
                            .with_warning(
                                Warning::new(
                                    "instruction_duplicate_claude_bridge",
                                    "Both project Claude instruction locations exist; use repair with --yes to consolidate to the root bridge.",
                                )
                                .with_context("scope", scope_label(concrete_scope)),
                            )
                            .with_next_action(NextAction::new(
                                "repair_duplicate_bridge",
                                "Run instructions repair --project --yes to keep the root Claude bridge and remove the nested duplicate.",
                            ));
                        continue;
                    }
                } else if root_missing && nested_present {
                    bridges = vec![(
                        HarnessId::new("claude").expect("known harness id is valid"),
                        nested,
                    )];
                }
            }
            let canonical_id = instruction_operation_id(concrete_scope, "canonical", "root");
            let canonical_resource =
                match instruction_resource_key(concrete_scope, "canonical", "root") {
                    Some(key) => key,
                    None => {
                        outcome.result = ResultClass::Invalid;
                        return outcome.with_error(ErrorDetail::new(
                            "instruction_resource_invalid",
                            "The instruction resource identifier could not be represented safely.",
                        ));
                    }
                };
            let canonical_missing = match filesystem.inspect(&canonical) {
                Ok(metadata) => match metadata.kind() {
                    FileKind::Missing => true,
                    FileKind::RegularFile => false,
                    _ => {
                        outcome.result = ResultClass::AttentionRequired;
                        outcome = outcome.with_warning(Warning::new(
                            "instruction_canonical_conflict",
                            "The canonical AGENTS.md path is not a regular file; no change was made.",
                        ));
                        false
                    }
                },
                Err(_) => {
                    outcome.result = ResultClass::AttentionRequired;
                    outcome = outcome.with_warning(Warning::new(
                        "instruction_canonical_unreadable",
                        "The canonical AGENTS.md path could not be inspected safely.",
                    ));
                    false
                }
            };
            let mut canonical_dependency = None;
            if canonical_missing {
                let operation = match skilltap_core::lifecycle_operation::faithful_file_operation(
                    canonical_id.clone(),
                    enabled.first().expect("enabled set is non-empty").clone(),
                    canonical_resource.clone(),
                    OperationAction::InstructionSetup,
                    canonical.clone(),
                ) {
                    Ok(operation) => operation,
                    Err(_) => {
                        outcome.result = ResultClass::Invalid;
                        return outcome.with_error(ErrorDetail::new(
                            "operation_contract_invalid",
                            "The canonical instruction operation was invalid.",
                        ));
                    }
                };
                operations.push(operation);
                entries.insert(
                    canonical_id.clone(),
                    InstructionEntry {
                        path: canonical.clone(),
                        write: InstructionWrite::Canonical,
                        action: OperationAction::InstructionSetup,
                        backup: None,
                    },
                );
                canonical_dependency = Some(canonical_id);
            }
            let canonical_desired = instruction_desired_resource(
                canonical_resource.clone(),
                enabled.first().expect("enabled set is non-empty").clone(),
            );
            inventory = match inventory.with_resource(canonical_desired) {
                Ok(inventory) => inventory,
                Err(_) => {
                    outcome.result = ResultClass::AttentionRequired;
                    return outcome.with_error(ErrorDetail::new(
                        "inventory_resource_conflict",
                        "The canonical instruction resource conflicts with desired state.",
                    ));
                }
            };
            let canonical_state = ResourceState::new(
                canonical_resource,
                BTreeMap::from([(
                    enabled.first().expect("enabled set is non-empty").clone(),
                    NativeId::new(canonical.as_str()).expect("absolute path is valid native id"),
                )]),
                Provenance::Direct,
                Ownership::Skilltap,
                None,
                None,
                Some(fingerprint_contents(&[])),
                None,
                None,
                timestamp,
                None,
            )
            .map_err(|_| ())
            .ok();
            if let Some(state) = canonical_state {
                seeds.insert(state.key().clone(), state);
            }

            if let Some(nested) = duplicate_nested {
                let nested_resource = match instruction_resource_key(
                    concrete_scope,
                    "bridge-nested",
                    "claude",
                ) {
                    Some(key) => key,
                    None => {
                        outcome.result = ResultClass::Invalid;
                        return outcome.with_error(ErrorDetail::new(
                            "instruction_resource_invalid",
                            "The duplicate instruction resource identifier could not be represented safely.",
                        ));
                    }
                };
                let nested_id = instruction_operation_id(concrete_scope, "bridge-nested", "claude");
                let nested_operation =
                    match skilltap_core::lifecycle_operation::faithful_file_operation(
                        nested_id.clone(),
                        HarnessId::new("claude").expect("known harness id is valid"),
                        nested_resource.clone(),
                        OperationAction::InstructionRepair,
                        nested.clone(),
                    ) {
                        Ok(operation) => operation,
                        Err(_) => {
                            outcome.result = ResultClass::Invalid;
                            return outcome.with_error(ErrorDetail::new(
                                "operation_contract_invalid",
                                "The duplicate instruction removal operation was invalid.",
                            ));
                        }
                    };
                let nested_metadata = filesystem.inspect(&nested).ok();
                let backup = nested_metadata
                    .as_ref()
                    .filter(|metadata| metadata.kind() == FileKind::RegularFile)
                    .map(|_| instruction_backup_path(&paths, &nested));
                let nested_bytes = nested_metadata
                    .as_ref()
                    .filter(|metadata| metadata.kind() == FileKind::RegularFile)
                    .and_then(|_| filesystem.read(&nested).ok())
                    .unwrap_or_default();
                let nested_state = ResourceState::new(
                    nested_resource,
                    BTreeMap::from([(
                        HarnessId::new("claude").expect("known harness id is valid"),
                        NativeId::new(nested.as_str()).expect("absolute path is valid native id"),
                    )]),
                    Provenance::Direct,
                    Ownership::Skilltap,
                    None,
                    None,
                    Some(fingerprint_contents(&nested_bytes)),
                    None,
                    None,
                    timestamp,
                    None,
                )
                .map_err(|_| ())
                .ok();
                operations.push(nested_operation);
                entries.insert(
                    nested_id,
                    InstructionEntry {
                        path: nested,
                        write: InstructionWrite::Remove,
                        action: OperationAction::InstructionRepair,
                        backup,
                    },
                );
                if let Some(state) = nested_state {
                    seeds.insert(state.key().clone(), state);
                }
                outcome = outcome.with_warning(Warning::new(
                    "instruction_bridge_consolidation",
                    "The root project Claude bridge is canonical; the nested duplicate will be backed up and removed.",
                ));
            }

            for (target, bridge) in bridges {
                let nested_project_bridge = matches!(concrete_scope, Scope::Project(_))
                    && bridge.as_str().ends_with("/.claude/CLAUDE.md");
                let expected_symlink = RelativeSymlinkTarget::new(
                    if nested_project_bridge || matches!(concrete_scope, Scope::Global) {
                        "../AGENTS.md"
                    } else {
                        "AGENTS.md"
                    },
                );
                let (write, expected_bytes) = match mode {
                    ClaudeInstructionMode::Symlink => (
                        InstructionWrite::Symlink {
                            target: expected_symlink.clone().expect("static link target valid"),
                        },
                        Vec::new(),
                    ),
                    ClaudeInstructionMode::Import => {
                        let bytes = if matches!(concrete_scope, Scope::Global) {
                            b"@~/AGENTS.md\n".to_vec()
                        } else if nested_project_bridge {
                            b"@../AGENTS.md\n".to_vec()
                        } else {
                            b"@AGENTS.md\n".to_vec()
                        };
                        (
                            InstructionWrite::Import {
                                contents: bytes.clone(),
                            },
                            bytes,
                        )
                    }
                };
                let bridge_health = match instruction_bridge_status_with_target(
                    &filesystem,
                    &bridge,
                    mode,
                    if nested_project_bridge || matches!(concrete_scope, Scope::Global) {
                        "../AGENTS.md"
                    } else {
                        "AGENTS.md"
                    },
                    &expected_bytes,
                ) {
                    "missing" => InstructionBridgeHealth::Missing,
                    "managed" => InstructionBridgeHealth::Managed,
                    _ => InstructionBridgeHealth::Conflict,
                };
                let bridge_resource =
                    match instruction_resource_key(concrete_scope, "bridge", target.as_str()) {
                        Some(key) => key,
                        None => continue,
                    };
                let desired = instruction_desired_resource(bridge_resource.clone(), target.clone());
                inventory = match inventory.with_resource(desired) {
                    Ok(inventory) => inventory,
                    Err(_) => {
                        outcome.result = ResultClass::AttentionRequired;
                        return outcome.with_error(ErrorDetail::new(
                            "inventory_resource_conflict",
                            "The instruction bridge conflicts with desired state.",
                        ));
                    }
                };
                let observed_bytes = match &write {
                    InstructionWrite::Import { contents } => contents.clone(),
                    InstructionWrite::Canonical | InstructionWrite::Symlink { .. } => Vec::new(),
                    InstructionWrite::Remove => Vec::new(),
                };
                let bridge_state = ResourceState::new(
                    bridge_resource.clone(),
                    BTreeMap::from([(
                        target.clone(),
                        NativeId::new(bridge.as_str()).expect("absolute path is valid native id"),
                    )]),
                    Provenance::Direct,
                    Ownership::Skilltap,
                    None,
                    None,
                    Some(fingerprint_contents(&observed_bytes)),
                    None,
                    None,
                    timestamp,
                    None,
                )
                .map_err(|_| ())
                .ok();
                if let Some(state) = bridge_state {
                    seeds.insert(state.key().clone(), state);
                }
                if bridge_health == InstructionBridgeHealth::Managed {
                    outcome = outcome.with_operation(crate::OperationOutcome::new(
                        format!("instruction:{}:{}", target, scope_label(concrete_scope)),
                        "no_change",
                    ));
                    continue;
                }
                if bridge_health == InstructionBridgeHealth::Conflict {
                    let repairable = repair
                        && acknowledged
                        && filesystem
                            .inspect(&bridge)
                            .map(|metadata| metadata.kind() == FileKind::RegularFile)
                            .unwrap_or(false);
                    if repairable {
                        // The repair operation below creates a recoverable
                        // backup before removing the divergent regular file.
                    } else {
                        outcome.result = ResultClass::AttentionRequired;
                        outcome = outcome.with_warning(
                            Warning::new(
                                "instruction_bridge_conflict",
                                if repair {
                                    "The bridge requires --yes and must be a divergent regular file before repair."
                                } else {
                                    "The bridge contains existing content; use instructions repair with --yes."
                                },
                            )
                            .with_context("target", target.as_str()),
                        );
                        continue;
                    }
                }
                let repair_operation =
                    repair && acknowledged && bridge_health == InstructionBridgeHealth::Conflict;
                if repair_operation {
                    outcome.result = ResultClass::AttentionRequired;
                    outcome = outcome.with_warning(
                        Warning::new(
                            "instruction_bridge_repair",
                            "The divergent instruction bridge will be backed up before replacement.",
                        )
                        .with_context("target", target.as_str()),
                    );
                }
                let operation_id =
                    instruction_operation_id(concrete_scope, "bridge", target.as_str());
                let operation_action = if repair_operation {
                    OperationAction::InstructionRepair
                } else {
                    OperationAction::InstructionSetup
                };
                let operation = match skilltap_core::lifecycle_operation::faithful_file_operation_with_dependencies(
                    operation_id.clone(),
                    target.clone(),
                    bridge_resource,
                    operation_action,
                    bridge.clone(),
                    canonical_dependency
                        .clone()
                        .into_iter()
                        .map(skilltap_core::domain::OperationDependency::new),
                ) {
                    Ok(operation) => operation,
                    Err(_) => {
                        outcome.result = ResultClass::Invalid;
                        return outcome.with_error(ErrorDetail::new(
                            "operation_contract_invalid",
                            "The instruction bridge operation was invalid.",
                        ));
                    }
                };
                operations.push(operation);
                entries.insert(
                    operation_id,
                    InstructionEntry {
                        path: bridge.clone(),
                        write,
                        action: operation_action,
                        backup: repair_operation.then(|| instruction_backup_path(&paths, &bridge)),
                    },
                );
            }
        }
        let empty_inventory = documents.inventory.clone().unwrap_or_else(|| {
            InventoryDocument::new(skilltap_core::storage::INVENTORY_SCHEMA_VERSION, [], [])
                .expect("empty inventory is valid")
        });
        if inventory != empty_inventory && self.inventory.replace(&inventory).is_err() {
            outcome.result = ResultClass::Invalid;
            return outcome.with_error(ErrorDetail::new(
                "inventory_publish_failed",
                "The instruction inventory could not be published safely.",
            ));
        }
        if operations.is_empty() {
            if let Err(()) = seed_state_if_missing(self.state, &seeds) {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "state_seed_publish_failed",
                    "The instruction state could not be recorded safely.",
                ));
            }
            if outcome.errors.is_empty() && outcome.warnings.is_empty() {
                outcome.result = ResultClass::Completed;
            }
            let operation_count = outcome.operations.len() as u64;
            return outcome
                .with_summary("operations", operation_count)
                .with_summary("changed", false);
        }
        let plan = match Plan::new(operations) {
            Ok(plan) => plan,
            Err(_) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "operation_plan_invalid",
                    "The instruction operation plan was invalid.",
                ));
            }
        };
        let port = InstructionPort {
            filesystem: &filesystem,
            entries,
        };
        let journal = StateExecutionJournal {
            plan: &plan,
            state: self.state,
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
        let report =
            match execute_plan(&SystemConfigurationLock, &lock_path, &port, &journal, &plan) {
                Ok(report) => report,
                Err(error) => {
                    outcome.result = ResultClass::AttentionRequired;
                    return outcome.with_error(native_execution_error(&error));
                }
            };
        for result in report.result.operations().values() {
            outcome = outcome.with_operation(crate::OperationOutcome::new(
                result.operation_id().to_string(),
                operation_result_status(result.outcome()),
            ));
            if !matches!(
                result.outcome(),
                OperationOutcome::Applied | OperationOutcome::NoChange
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

    fn execute_reconciliation(
        &self,
        command: &'static str,
        target: &TargetArgs,
        requested_scope: &ScopeArgs,
        includes: &[NativeId],
        excludes: &[NativeId],
        acknowledged: bool,
    ) -> Outcome {
        let (documents, mut outcome) = match self.load_documents(command) {
            Ok(value) => value,
            Err(outcome) => return *outcome,
        };

        let status_args = StatusArgs {
            target: target.clone(),
            scope: requested_scope.clone(),
            output: OutputArgs::default(),
        };
        let scope = match StatusScope::resolve(self, &status_args, &documents) {
            Ok(scope) => scope,
            Err(error) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(error);
            }
        };
        outcome.scope = Some(scope.output.clone());

        let targets = match StatusTargets::resolve(&status_args, &documents) {
            Ok(targets) => targets,
            Err(StatusTargetError::NoneEnabled) => {
                return first_use_harness_report(
                    &documents.config,
                    outcome,
                    self.native_observation,
                    target.target.as_ref(),
                )
                .with_summary("scopes", scope.count)
                .with_summary("targets", 0_u64)
                .with_next_action(
                    NextAction::new("enable_harness", "Enable Codex or Claude management.")
                        .with_command("skilltap harness enable <codex|claude>"),
                );
            }
            Err(StatusTargetError::NotEnabled) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(
                    ErrorDetail::new(
                        "target_not_enabled",
                        "The requested harness target is not enabled.",
                    )
                    .with_next_action(
                        NextAction::new("enable_harness", "Enable the requested harness.")
                            .with_command("skilltap harness enable <codex|claude>"),
                    ),
                );
            }
        };

        if let Some(selector) = includes.first().or_else(|| excludes.first()) {
            outcome.result = ResultClass::Invalid;
            return outcome.with_error(
                ErrorDetail::new(
                    "selector_unavailable",
                    "The requested selector is not present in the current reconciliation plan.",
                )
                .with_context("selector", selector.as_str()),
            );
        }

        let observation = match self.native_observation {
            NativeObservationMode::Disabled => NativeObservation::default(),
            NativeObservationMode::System => NativeObservation::run(&documents, &scope, &targets),
        };
        for resource in observation.resources.iter().cloned() {
            outcome = outcome.with_resource(resource);
        }
        for warning in observation.warnings.iter().cloned() {
            outcome = outcome.with_warning(warning);
        }

        let planned = match plan_reconciliation(ReconciliationRequest::default()) {
            Ok(plan) => plan,
            Err(error) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(
                    ErrorDetail::new(
                        "reconciliation_plan_invalid",
                        "The reconciliation plan could not be validated safely.",
                    )
                    .with_context("detail", error.to_string()),
                );
            }
        };
        let operation_count = planned.plan.iter().count() as u64;
        let desired_count = documents
            .inventory
            .as_ref()
            .map_or(0, |inventory| inventory.resources().len());
        let mut result = if observation.failed_targets > 0 {
            ResultClass::AttentionRequired
        } else {
            ResultClass::Completed
        };
        if desired_count > 0 {
            result = ResultClass::AttentionRequired;
            outcome = outcome
                .with_warning(Warning::new(
                    "reconciliation_candidates_unavailable",
                    "Desired resources are present, but no lifecycle adapter can safely produce mutation candidates yet.",
                ))
                .with_next_action(NextAction::new(
                    "review_lifecycle_support",
                    "Review the planned resource lifecycle support before retrying synchronization.",
                ));
        }
        if acknowledged {
            outcome = outcome.with_warning(Warning::new(
                "acknowledgment_not_applicable",
                "--yes acknowledged no exact consequence because this plan contains no partial operation.",
            ));
        }
        outcome.result = result;
        outcome
            .with_summary("desired_resources", desired_count as u64)
            .with_summary("operations", operation_count)
            .with_summary("scopes", scope.count)
            .with_summary("targets", targets.iter().len() as u64)
            .with_summary("observed_targets", observation.observed_targets as u64)
            .with_summary("failed_targets", observation.failed_targets as u64)
            .with_summary("changed", false)
    }

    pub(crate) fn execute(&self, args: &StatusArgs) -> Outcome {
        let (documents, mut outcome) = match self.load_documents("status") {
            Ok(value) => value,
            Err(outcome) => return *outcome,
        };

        let scope = match StatusScope::resolve(self, args, &documents) {
            Ok(scope) => scope,
            Err(error) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(error);
            }
        };
        outcome.scope = Some(scope.output.clone());

        let targets = match StatusTargets::resolve(args, &documents) {
            Ok(targets) => targets,
            Err(StatusTargetError::NoneEnabled) => {
                return first_use_harness_report(
                    &documents.config,
                    outcome,
                    self.native_observation,
                    args.target.target.as_ref(),
                )
                .with_summary("targets", 0_u64)
                .with_error(ErrorDetail::new(
                    "no_enabled_harnesses",
                    "No harness is enabled in skilltap configuration.",
                ))
                .with_next_action(
                    NextAction::new("enable_harness", "Enable Codex or Claude management.")
                        .with_command("skilltap harness enable <codex|claude>"),
                );
            }
            Err(StatusTargetError::NotEnabled) => {
                outcome.result = ResultClass::Invalid;
                return outcome
                    .with_error(ErrorDetail::new(
                        "target_not_enabled",
                        "The requested harness target is not enabled.",
                    ))
                    .with_next_action(
                        NextAction::new("enable_harness", "Enable the requested harness.")
                            .with_command("skilltap harness enable <codex|claude>"),
                    );
            }
        };

        StatusProjection {
            documents: &documents,
            scope: &scope,
            targets: &targets,
            native_observation: self.native_observation,
        }
        .apply(outcome)
    }

    pub(crate) fn execute_adopt(&self, args: &AdoptArgs) -> Outcome {
        let (documents, mut outcome) = match self.load_documents("adopt") {
            Ok(value) => value,
            Err(outcome) => return *outcome,
        };

        let status_args = StatusArgs {
            target: TargetArgs {
                target: args.from.clone(),
            },
            scope: args.scope.clone(),
            output: OutputArgs::default(),
        };
        let scope = match StatusScope::resolve(self, &status_args, &documents) {
            Ok(scope) => scope,
            Err(error) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(error);
            }
        };
        let targets = match StatusTargets::resolve(&status_args, &documents) {
            Ok(targets) => targets,
            Err(StatusTargetError::NoneEnabled) => {
                outcome.result = ResultClass::AttentionRequired;
                return outcome
                    .with_error(ErrorDetail::new(
                        "no_enabled_harnesses",
                        "No harness is enabled in skilltap configuration.",
                    ))
                    .with_next_action(
                        NextAction::new("enable_harness", "Enable Codex or Claude management.")
                            .with_command("skilltap harness enable <codex|claude>"),
                    );
            }
            Err(StatusTargetError::NotEnabled) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "target_not_enabled",
                    "The requested harness target is not enabled.",
                ));
            }
        };
        outcome.scope = Some(scope.output.clone());

        let observation = NativeObservation::run(&documents, &scope, &targets);
        let Some(environment) = observation.environment.clone() else {
            outcome.result = ResultClass::AttentionRequired;
            return outcome
                .with_error(ErrorDetail::new(
                    "native_observation_unavailable",
                    "Native resources could not be observed safely; adoption did not write inventory.",
                ))
                .with_next_action(NextAction::new(
                    "repair_native_observation",
                    "Resolve the reported harness observation problem and retry adoption.",
                ));
        };
        for warning in observation.warnings {
            outcome = outcome.with_warning(warning);
        }
        outcome = outcome
            .with_summary("scopes", scope.count)
            .with_summary("targets", targets.iter().len() as u64)
            .with_summary("observed_targets", observation.observed_targets as u64)
            .with_summary("failed_targets", observation.failed_targets as u64)
            .with_summary("native_entries", observation.native_entries as u64);
        let selection = AdoptionSelection::new(targets.iter().flat_map(|harness| {
            scope
                .resolved
                .iter()
                .map(move |scope| ObservationTarget::new(harness.clone(), scope.clone()))
        }));
        let initial_plan =
            match plan_adoption(documents.inventory.as_ref(), &environment, &selection) {
                Ok(plan) => plan,
                Err(_) => {
                    outcome.result = ResultClass::Invalid;
                    return outcome.with_error(ErrorDetail::new(
                        "adoption_plan_invalid",
                        "Native observations could not be converted into a safe adoption plan.",
                    ));
                }
            };

        if initial_plan.additions.is_empty() {
            return project_adoption(outcome, &initial_plan, false, observation.failed_targets);
        }

        let paths = match PlatformPaths::resolve(&ProcessEnvironment) {
            Ok(paths) => paths,
            Err(_) => {
                return outcome
                    .with_error(ErrorDetail::new(
                        "platform_paths_unavailable",
                        "The skilltap configuration paths could not be resolved.",
                    ))
                    .with_next_action(NextAction::new(
                        "repair_environment",
                        "Repair the configured home and XDG paths before retrying adoption.",
                    ));
            }
        };
        let lock_path = match AbsolutePath::new(format!(
            "{}/skilltap.lock",
            paths.skilltap_config().as_str()
        )) {
            Ok(path) => path,
            Err(_) => {
                return outcome.with_error(ErrorDetail::new(
                    "lock_path_invalid",
                    "The skilltap configuration lock path is invalid.",
                ));
            }
        };
        let applied = apply_adoption(
            &SystemConfigurationLock,
            &lock_path,
            self.inventory,
            &initial_plan,
            |evidence| {
                let refreshed = NativeObservation::run(&documents, &scope, &targets);
                if refreshed.environment.is_none() {
                    return Err(AdoptionObservationError::Unavailable);
                }
                if refreshed.failed_targets > 0 {
                    // The normalized environment still carries healthy sibling
                    // observations; the core revalidation decides whether the
                    // selected evidence itself is stale.
                }
                let _ = evidence;
                Ok(refreshed.environment.expect("checked above"))
            },
        );
        match applied {
            Ok(result) => project_adoption(
                outcome,
                &result.plan,
                result.changed,
                observation.failed_targets,
            ),
            Err(error) => outcome
                .with_error(adoption_apply_error(&error))
                .with_next_action(adoption_next_action(&error)),
        }
    }

    fn load_documents(
        &self,
        command: &'static str,
    ) -> Result<(StatusDocuments, Outcome), Box<Outcome>> {
        let loaded = DocumentLoadPhase::execute(self);
        let mut outcome = loaded.project(Outcome::new(command, ResultClass::AttentionRequired));
        match loaded.finish() {
            Ok(documents) => Ok((documents, outcome)),
            Err(errors) => {
                outcome.result = ResultClass::Invalid;
                for error in errors {
                    outcome = outcome.with_error(error);
                }
                Err(Box::new(outcome.with_next_action(NextAction::new(
                    "repair_owned_documents",
                    "Repair the reported skilltap-owned documents before retrying.",
                ))))
            }
        }
    }

    fn scope_request(
        &self,
        args: &StatusArgs,
        inventory: Option<&InventoryDocument>,
    ) -> Result<ScopeRequest, ErrorDetail> {
        match args.scope.argument() {
            ScopeArgument::Global => Ok(ScopeRequest::Global),
            ScopeArgument::AllScopes => Ok(ScopeRequest::AllScopes {
                recorded_projects: inventory
                    .map(|value| value.projects().iter().cloned().collect())
                    .unwrap_or_default(),
            }),
            ScopeArgument::Project(None) => Ok(ScopeRequest::Project { path: None }),
            ScopeArgument::Project(Some(path)) => {
                let path =
                    absolute_project_argument(&path, self.working_directory).map_err(|_| {
                        ErrorDetail::new(
                            "invalid_project_path",
                            "The project path could not be converted to a canonical absolute path.",
                        )
                    })?;
                Ok(ScopeRequest::Project { path: Some(path) })
            }
        }
    }
}

fn skill_relative_destination(
    name: &NativeId,
) -> Option<skilltap_core::domain::RelativeArtifactPath> {
    skilltap_core::domain::RelativeArtifactPath::new(format!("skills/{}", name.as_str())).ok()
}

fn scope_args_for_scope(scope: &Scope) -> ScopeArgs {
    match scope {
        Scope::Global => ScopeArgs::default(),
        Scope::Project(path) => ScopeArgs {
            project: Some(Some(PathBuf::from(path.as_str()))),
            all_scopes: false,
        },
    }
}

fn merge_result(current: ResultClass, next: ResultClass) -> ResultClass {
    fn rank(result: ResultClass) -> u8 {
        match result {
            ResultClass::Completed => 0,
            ResultClass::Invalid => 1,
            ResultClass::AttentionRequired => 2,
            ResultClass::PartialApply => 3,
        }
    }

    if rank(next) > rank(current) {
        next
    } else {
        current
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InstructionBridgeHealth {
    Missing,
    Managed,
    Conflict,
}

fn instruction_locations(
    paths: &PlatformPaths,
    scope: &Scope,
    enabled: &[HarnessId],
) -> (AbsolutePath, Vec<(HarnessId, AbsolutePath)>) {
    match scope {
        Scope::Global => {
            let canonical = paths.global_agents().clone();
            let mut bridges = Vec::new();
            if enabled.iter().any(|target| target.as_str() == "codex") {
                bridges.push((
                    HarnessId::new("codex").expect("known harness id is valid"),
                    AbsolutePath::new(format!("{}/AGENTS.md", paths.codex_home().as_str()))
                        .expect("codex bridge path is valid"),
                ));
            }
            if enabled.iter().any(|target| target.as_str() == "claude") {
                bridges.push((
                    HarnessId::new("claude").expect("known harness id is valid"),
                    AbsolutePath::new(format!("{}/CLAUDE.md", paths.claude_home().as_str()))
                        .expect("claude bridge path is valid"),
                ));
            }
            (canonical, bridges)
        }
        Scope::Project(project) => {
            let canonical = AbsolutePath::new(format!("{}/AGENTS.md", project.as_str()))
                .expect("project canonical path is valid");
            let bridges = if enabled.iter().any(|target| target.as_str() == "claude") {
                vec![(
                    HarnessId::new("claude").expect("known harness id is valid"),
                    AbsolutePath::new(format!("{}/CLAUDE.md", project.as_str()))
                        .expect("project Claude bridge path is valid"),
                )]
            } else {
                Vec::new()
            };
            (canonical, bridges)
        }
    }
}

fn instruction_resource_key(scope: &Scope, role: &str, target: &str) -> Option<ResourceKey> {
    let scope_label = match scope {
        Scope::Global => "global".to_owned(),
        Scope::Project(path) => {
            let mut hash = 0xcbf29ce484222325_u64;
            for byte in path.as_str().bytes() {
                hash ^= u64::from(byte);
                hash = hash.wrapping_mul(0x100000001b3);
            }
            format!("project-{hash:016x}")
        }
    };
    ResourceId::new(format!("instructions:{scope_label}:{role}:{target}"))
        .ok()
        .map(|id| ResourceKey::new(id, scope.clone()))
}

fn instruction_operation_id(scope: &Scope, role: &str, target: &str) -> OperationId {
    let resource = instruction_resource_key(scope, role, target)
        .expect("instruction resource identity is valid");
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in resource.id().as_str().bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    OperationId::new(format!("instructions:{hash:016x}"))
        .expect("instruction operation id is valid")
}

fn instruction_backup_path(paths: &PlatformPaths, bridge: &AbsolutePath) -> AbsolutePath {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bridge.as_str().bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    AbsolutePath::new(format!(
        "{}/managed/backups/instructions/{hash:016x}.bak",
        paths.skilltap_config().as_str()
    ))
    .expect("instruction backup path is valid")
}

fn instruction_desired_resource(resource: ResourceKey, target: HarnessId) -> DesiredResource {
    DesiredResource::new(
        resource,
        ResourceKind::InstructionLocation,
        HarnessSet::new([target]).expect("instruction target set is non-empty"),
        DesiredOrigin::Direct,
        None,
        UpdateIntent::Pinned,
        ComponentGraph::new([]).expect("empty component graph is valid"),
        BTreeMap::new(),
        BTreeMap::new(),
        BTreeSet::new(),
    )
    .expect("instruction desired resource is valid")
}

fn instruction_bridge_status(
    filesystem: &dyn FileSystem,
    bridge: &AbsolutePath,
    scope: &Scope,
    mode: ClaudeInstructionMode,
) -> &'static str {
    let (symlink_target, import_contents) = match scope {
        Scope::Global => ("../AGENTS.md", b"@~/AGENTS.md\n".as_slice()),
        Scope::Project(_) => ("AGENTS.md", b"@AGENTS.md\n".as_slice()),
    };
    instruction_bridge_status_with_target(filesystem, bridge, mode, symlink_target, import_contents)
}

fn instruction_bridge_status_with_target(
    filesystem: &dyn FileSystem,
    bridge: &AbsolutePath,
    mode: ClaudeInstructionMode,
    symlink_target: &str,
    import_contents: &[u8],
) -> &'static str {
    let metadata = match filesystem.inspect(bridge) {
        Ok(metadata) => metadata,
        Err(_) => return "unreadable",
    };
    match metadata.kind() {
        FileKind::Missing => "missing",
        FileKind::Symlink => {
            if mode == ClaudeInstructionMode::Symlink
                && metadata
                    .link_target()
                    .is_some_and(|target| target == std::path::Path::new(symlink_target))
            {
                "managed"
            } else {
                "divergent"
            }
        }
        FileKind::RegularFile => {
            if mode == ClaudeInstructionMode::Import
                && filesystem.read(bridge).ok().as_deref() == Some(import_contents)
            {
                "managed"
            } else {
                "divergent"
            }
        }
        _ => "broken",
    }
}

fn skill_destination(
    paths: &PlatformPaths,
    scope: &Scope,
    target: &HarnessId,
    destination: &skilltap_core::domain::RelativeArtifactPath,
) -> Option<(AbsolutePath, AbsolutePath)> {
    let root = match (scope, target.as_str()) {
        (Scope::Global, "codex") => {
            AbsolutePath::new(format!("{}/.agents", paths.home().as_str())).ok()?
        }
        (Scope::Global, "claude") => paths.claude_home().clone(),
        (Scope::Project(project), "codex") => {
            AbsolutePath::new(format!("{}/.agents", project.as_str())).ok()?
        }
        (Scope::Project(project), "claude") => {
            AbsolutePath::new(format!("{}/.claude", project.as_str())).ok()?
        }
        _ => return None,
    };
    let full = AbsolutePath::new(format!("{}/{}", root.as_str(), destination.as_str())).ok()?;
    Some((root, full))
}

fn canonical_skill_destination(
    paths: &PlatformPaths,
    scope: &Scope,
    destination: &skilltap_core::domain::RelativeArtifactPath,
) -> Option<(AbsolutePath, AbsolutePath)> {
    let root = match scope {
        Scope::Global => AbsolutePath::new(format!("{}/.agents", paths.home().as_str())).ok()?,
        Scope::Project(project) => {
            AbsolutePath::new(format!("{}/.agents", project.as_str())).ok()?
        }
    };
    let full = AbsolutePath::new(format!("{}/{}", root.as_str(), destination.as_str())).ok()?;
    Some((root, full))
}

fn skill_destinations(
    paths: &PlatformPaths,
    scope: &Scope,
    targets: &HarnessSet,
    destination: &skilltap_core::domain::RelativeArtifactPath,
) -> Option<Vec<SkillDestination>> {
    let mut destinations = Vec::new();
    let codex_selected = targets.iter().any(|target| target.as_str() == "codex");
    if !codex_selected {
        let target = targets.iter().next()?.clone();
        let (root, full_path) = canonical_skill_destination(paths, scope, destination)?;
        destinations.push(SkillDestination {
            target,
            canonical: true,
            root,
            full_path,
        });
    }
    for target in targets.iter() {
        let (root, full_path) = skill_destination(paths, scope, target, destination)?;
        destinations.push(SkillDestination {
            target: target.clone(),
            canonical: false,
            root,
            full_path,
        });
    }
    Some(destinations)
}

struct ResolvedGitSkill {
    root: AbsolutePath,
    commit: GitCommit,
}

struct SkillDestination {
    target: HarnessId,
    canonical: bool,
    root: AbsolutePath,
    full_path: AbsolutePath,
}

/// Resolve a Git source into skilltap's private managed cache using bounded,
/// direct Git invocations. The cache identity is derived from the locator, so
/// repeated installs fetch into the same checkout and can observe a changed
/// commit without scanning unrelated repositories.
fn resolve_git_skill_source(
    paths: &PlatformPaths,
    locator: &SourceLocator,
    requested_revision: Option<&skilltap_core::domain::RequestedRevision>,
    subdirectory: Option<&skilltap_core::domain::RelativeArtifactPath>,
) -> Result<ResolvedGitSkill, ()> {
    let source_root = AbsolutePath::new(format!(
        "{}/managed/sources",
        paths.skilltap_config().as_str()
    ))
    .map_err(|_| ())?;
    SystemFileSystem
        .create_directory_all(&source_root)
        .map_err(|_| ())?;
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in locator.as_str().bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    let checkout =
        AbsolutePath::new(format!("{}/git-{hash:016x}", source_root.as_str())).map_err(|_| ())?;
    let git = NativeId::new("git").map_err(|_| ())?;
    let configured = ConfiguredBinary::path_lookup(git).map_err(|_| ())?;
    let executable = SystemExecutableResolver
        .resolve(&ExecutableResolutionRequest::new(
            configured,
            std::env::var_os("PATH"),
        ))
        .map_err(|_| ())?;
    let limits = ProcessLimits::new(120_000, 256 * 1024, 256 * 1024, 512 * 1024).map_err(|_| ())?;
    let filesystem = SystemFileSystem;
    let existing = filesystem.inspect(&checkout).map_err(|_| ())?;
    if existing.kind() == FileKind::Missing {
        let clone = NativeProcessRequest::new(
            executable.clone(),
            [
                OsString::from("clone"),
                OsString::from("--no-checkout"),
                OsString::from("--depth"),
                OsString::from("1"),
                OsString::from(locator.as_str()),
                OsString::from(checkout.as_str()),
            ],
            BTreeMap::new(),
            None,
            limits,
        );
        let output = SystemNativeProcessRunner.run(&clone).map_err(|_| ())?;
        if !output.status().success() {
            return Err(());
        }
    } else if existing.kind() != FileKind::Directory {
        return Err(());
    } else {
        let fetch = NativeProcessRequest::new(
            executable.clone(),
            [
                OsString::from("-C"),
                OsString::from(checkout.as_str()),
                OsString::from("fetch"),
                OsString::from("--prune"),
                OsString::from("origin"),
            ],
            BTreeMap::new(),
            None,
            limits,
        );
        let output = SystemNativeProcessRunner.run(&fetch).map_err(|_| ())?;
        if !output.status().success() {
            return Err(());
        }
        if requested_revision.is_none() {
            let set_head = NativeProcessRequest::new(
                executable.clone(),
                [
                    OsString::from("-C"),
                    OsString::from(checkout.as_str()),
                    OsString::from("remote"),
                    OsString::from("set-head"),
                    OsString::from("origin"),
                    OsString::from("--auto"),
                ],
                BTreeMap::new(),
                None,
                limits,
            );
            let _ = SystemNativeProcessRunner.run(&set_head);
        }
    }
    let revision = requested_revision
        .map(|revision| revision.as_str())
        .unwrap_or("origin/HEAD");
    let verify = NativeProcessRequest::new(
        executable.clone(),
        [
            OsString::from("-C"),
            OsString::from(checkout.as_str()),
            OsString::from("rev-parse"),
            OsString::from("--verify"),
            OsString::from(format!("{revision}^{{commit}}")),
        ],
        BTreeMap::new(),
        None,
        limits,
    );
    let output = SystemNativeProcessRunner.run(&verify).map_err(|_| ())?;
    if !output.status().success() {
        return Err(());
    }
    let commit_text = std::str::from_utf8(output.stdout())
        .map_err(|_| ())?
        .trim()
        .to_owned();
    let commit = GitCommit::new(commit_text).map_err(|_| ())?;
    let checkout_root = checkout.clone();
    let checkout = NativeProcessRequest::new(
        executable,
        [
            OsString::from("-C"),
            OsString::from(checkout.as_str()),
            OsString::from("checkout"),
            OsString::from("--detach"),
            OsString::from("--force"),
            OsString::from(commit.as_str()),
        ],
        BTreeMap::new(),
        None,
        limits,
    );
    let output = SystemNativeProcessRunner.run(&checkout).map_err(|_| ())?;
    if !output.status().success() {
        return Err(());
    }
    let root = append_skill_subdirectory(checkout_root, subdirectory).ok_or(())?;
    Ok(ResolvedGitSkill { root, commit })
}

fn append_skill_subdirectory(
    root: AbsolutePath,
    subdirectory: Option<&skilltap_core::domain::RelativeArtifactPath>,
) -> Option<AbsolutePath> {
    subdirectory
        .map(|path| AbsolutePath::new(format!("{}/{}", root.as_str(), path.as_str())).ok())
        .unwrap_or(Some(root))
}

fn skill_operation_id(target: &HarnessId, resource: &ResourceKey) -> OperationId {
    let label = format!("skill:{target}:{}", resource.id().as_str());
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in label.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    OperationId::new(format!("skill:{target}:{hash:016x}")).expect("skill operation id is valid")
}

fn skill_canonical_operation_id(resource: &ResourceKey) -> OperationId {
    let label = format!("skill-canonical:{}", resource.id().as_str());
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in label.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    OperationId::new(format!("skill-canonical:{hash:016x}"))
        .expect("canonical skill operation id is valid")
}

fn skill_remove_operation_id(target: &HarnessId, resource: &ResourceKey) -> OperationId {
    let label = format!("skill-remove:{target}:{}", resource.id().as_str());
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in label.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    OperationId::new(format!("skill-remove:{target}:{hash:016x}"))
        .expect("skill removal operation id is valid")
}

fn skill_canonical_remove_operation_id(resource: &ResourceKey) -> OperationId {
    let label = format!("skill-remove-canonical:{}", resource.id().as_str());
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in label.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    OperationId::new(format!("skill-remove-canonical:{hash:016x}"))
        .expect("canonical skill removal operation id is valid")
}

#[derive(Clone)]
struct NativeLifecycleSpec {
    operation_action: OperationAction,
    native_action: NativeLifecycleAction,
    resource_kind: ResourceKind,
    resource_prefix: &'static str,
    native_name: NativeId,
    source: Option<Source>,
}

fn native_resource_kind(kind: NativeLifecycleKind) -> ResourceKind {
    match kind {
        NativeLifecycleKind::MarketplaceAdd
        | NativeLifecycleKind::MarketplaceRemove
        | NativeLifecycleKind::MarketplaceUpdate => ResourceKind::Marketplace,
        NativeLifecycleKind::PluginInstall
        | NativeLifecycleKind::PluginRemove
        | NativeLifecycleKind::PluginUpdate => ResourceKind::Plugin,
    }
}

fn native_resource_prefix(kind: NativeLifecycleKind) -> &'static str {
    match native_resource_kind(kind) {
        ResourceKind::Marketplace => "marketplace:",
        ResourceKind::Plugin => "plugin:",
        _ => unreachable!("native lifecycle resources have a marketplace or plugin kind"),
    }
}

impl NativeLifecycleSpec {
    fn parse(
        kind: NativeLifecycleKind,
        source_value: Option<&str>,
        name_value: Option<&str>,
    ) -> Result<Self, ErrorDetail> {
        match kind {
            NativeLifecycleKind::MarketplaceAdd => {
                let source_value = source_value.ok_or_else(|| {
                    ErrorDetail::new(
                        "source_required",
                        "Marketplace registration requires an explicit source.",
                    )
                })?;
                let locator = SourceLocator::new(source_value).map_err(|_| {
                    ErrorDetail::new("invalid_source", "The marketplace source is invalid.")
                })?;
                let native_name = match name_value {
                    Some(name) => NativeId::new(name).map_err(|_| {
                        ErrorDetail::new("invalid_name", "The marketplace name is invalid.")
                    })?,
                    None => derive_marketplace_name(locator.as_str()).ok_or_else(|| {
                        ErrorDetail::new(
                            "name_required",
                            "The marketplace name could not be derived; provide --name.",
                        )
                    })?,
                };
                let source = Source::new(SourceKind::Git, locator, None).map_err(|_| {
                    ErrorDetail::new("invalid_source", "The marketplace source is invalid.")
                })?;
                Ok(Self {
                    operation_action: OperationAction::MarketplaceRegister,
                    native_action: NativeLifecycleAction::MarketplaceAdd,
                    resource_kind: ResourceKind::Marketplace,
                    resource_prefix: "marketplace",
                    native_name,
                    source: Some(source),
                })
            }
            NativeLifecycleKind::MarketplaceRemove | NativeLifecycleKind::MarketplaceUpdate => {
                let native_name = name_value
                    .ok_or_else(|| {
                        ErrorDetail::new(
                            "name_required",
                            "The marketplace name is required for this lifecycle operation.",
                        )
                    })
                    .and_then(|name| {
                        NativeId::new(name).map_err(|_| {
                            ErrorDetail::new("invalid_name", "The marketplace name is invalid.")
                        })
                    })?;
                Ok(Self {
                    operation_action: if kind == NativeLifecycleKind::MarketplaceRemove {
                        OperationAction::MarketplaceRemove
                    } else {
                        OperationAction::MarketplaceUpdate
                    },
                    native_action: if kind == NativeLifecycleKind::MarketplaceRemove {
                        NativeLifecycleAction::MarketplaceRemove
                    } else {
                        NativeLifecycleAction::MarketplaceUpdate
                    },
                    resource_kind: ResourceKind::Marketplace,
                    resource_prefix: "marketplace",
                    native_name,
                    source: None,
                })
            }
            NativeLifecycleKind::PluginInstall => {
                let selector = source_value.ok_or_else(|| {
                    ErrorDetail::new(
                        "plugin_required",
                        "Plugin installation requires an exact plugin@marketplace selector.",
                    )
                })?;
                skilltap_core::marketplace::PluginSelector::parse(selector).map_err(|_| {
                    ErrorDetail::new(
                        "invalid_plugin_selector",
                        "The plugin selector must be an exact plugin@marketplace value.",
                    )
                })?;
                let native_name = NativeId::new(selector).map_err(|_| {
                    ErrorDetail::new("invalid_plugin_selector", "The plugin selector is invalid.")
                })?;
                Ok(Self {
                    operation_action: OperationAction::PluginInstall,
                    native_action: NativeLifecycleAction::PluginInstall,
                    resource_kind: ResourceKind::Plugin,
                    resource_prefix: "plugin",
                    native_name,
                    source: None,
                })
            }
            NativeLifecycleKind::PluginRemove | NativeLifecycleKind::PluginUpdate => {
                let selector = name_value.ok_or_else(|| {
                    ErrorDetail::new(
                        "plugin_required",
                        "The plugin selector is required for this lifecycle operation.",
                    )
                })?;
                skilltap_core::marketplace::PluginSelector::parse(selector).map_err(|_| {
                    ErrorDetail::new(
                        "invalid_plugin_selector",
                        "The plugin selector must be an exact plugin@marketplace value.",
                    )
                })?;
                let native_name = NativeId::new(selector).map_err(|_| {
                    ErrorDetail::new("invalid_plugin_selector", "The plugin selector is invalid.")
                })?;
                Ok(Self {
                    operation_action: if kind == NativeLifecycleKind::PluginRemove {
                        OperationAction::PluginRemove
                    } else {
                        OperationAction::PluginUpdate
                    },
                    native_action: if kind == NativeLifecycleKind::PluginRemove {
                        NativeLifecycleAction::PluginRemove
                    } else {
                        NativeLifecycleAction::PluginUpdate
                    },
                    resource_kind: ResourceKind::Plugin,
                    resource_prefix: "plugin",
                    native_name,
                    source: None,
                })
            }
        }
    }

    fn retains_desired(&self) -> bool {
        !matches!(
            self.operation_action,
            OperationAction::MarketplaceRemove | OperationAction::PluginRemove
        )
    }

    fn is_update(&self) -> bool {
        matches!(
            self.operation_action,
            OperationAction::MarketplaceUpdate | OperationAction::PluginUpdate
        )
    }

    fn resource_key(&self, scope: &Scope) -> Result<ResourceKey, ErrorDetail> {
        ResourceId::new(format!(
            "{}:{}",
            self.resource_prefix,
            self.native_name.as_str()
        ))
        .map(|id| ResourceKey::new(id, scope.clone()))
        .map_err(|_| {
            ErrorDetail::new(
                "resource_id_invalid",
                "The requested native resource identifier is invalid.",
            )
        })
    }

    fn desired_resource(
        &self,
        scope: &Scope,
        targets: &HarnessSet,
    ) -> Result<DesiredResource, ErrorDetail> {
        let key = ResourceKey::new(
            ResourceId::new(format!(
                "{}:{}",
                self.resource_prefix,
                self.native_name.as_str()
            ))
            .map_err(|_| {
                ErrorDetail::new(
                    "resource_id_invalid",
                    "The requested native resource identifier is invalid.",
                )
            })?,
            scope.clone(),
        );
        DesiredResource::new(
            key,
            self.resource_kind,
            targets.clone(),
            DesiredOrigin::Direct,
            self.source.clone(),
            UpdateIntent::Track,
            ComponentGraph::new([]).expect("empty component graph is valid"),
            BTreeMap::new(),
            BTreeMap::new(),
            BTreeSet::new(),
        )
        .map_err(|_| {
            ErrorDetail::new(
                "resource_contract_invalid",
                "The requested native resource could not be represented safely.",
            )
        })
    }

    fn native_request(&self, harness: HarnessKind, scope: Scope) -> NativeLifecycleRequest {
        NativeLifecycleRequest {
            harness,
            action: self.native_action,
            scope,
            name: self.native_name.clone(),
            source: self.source.as_ref().map(|source| source.locator().clone()),
        }
    }

    const fn operation_action(&self) -> OperationAction {
        self.operation_action
    }
}

fn derive_marketplace_name(locator: &str) -> Option<NativeId> {
    let trimmed = locator.trim_end_matches('/');
    let segment = trimmed
        .rsplit('/')
        .next()?
        .strip_suffix(".git")
        .unwrap_or(trimmed.rsplit('/').next()?);
    NativeId::new(segment).ok()
}

fn derive_skill_name(
    locator: &SourceLocator,
    subdirectory: Option<&skilltap_core::domain::RelativeArtifactPath>,
) -> Option<NativeId> {
    let segment = subdirectory
        .and_then(|path| path.as_str().rsplit('/').next())
        .or_else(|| locator.as_str().trim_end_matches('/').rsplit('/').next())?;
    let segment = segment.strip_suffix(".git").unwrap_or(segment);
    NativeId::new(segment).ok()
}

fn configured_native_profile(
    config: &ConfigDocument,
    target: &HarnessId,
    scope: &Scope,
    process_limits: ProcessLimits,
    json_limits: JsonLimits,
    search_path: Option<std::ffi::OsString>,
    capability_name: &str,
) -> Option<(HarnessKind, ConfiguredBinary, NativeId, CapabilitySupport)> {
    let (harness, binary) = match target.as_str() {
        "codex" => (HarnessKind::Codex, config.harnesses().codex.binary.as_str()),
        "claude" => (
            HarnessKind::Claude,
            config.harnesses().claude.binary.as_str(),
        ),
        _ => return None,
    };
    let configured = configured_binary(binary).ok()?;
    let executable = NativeId::new(binary).ok()?;
    let installation = detect_configured_installation(
        harness,
        configured.clone(),
        search_path,
        process_limits,
        json_limits,
    )
    .ok()?;
    let HarnessReachability::Reachable { native_version, .. } = installation.reachability() else {
        return None;
    };
    let profile = select_profile(harness, native_version);
    let capability_id = CapabilityId::new(capability_name).ok()?;
    let capability = profile
        .mutation_capabilities()
        .and_then(|capabilities| capabilities.for_scope(scope).support(&capability_id))
        .unwrap_or(CapabilitySupport::Unverified);
    Some((harness, configured, executable, capability))
}

fn command_arguments(arguments: Vec<std::ffi::OsString>) -> Result<Vec<CommandArgument>, ()> {
    arguments
        .into_iter()
        .map(|argument| {
            let value = argument.into_string().map_err(|_| ())?;
            Ok(CommandArgument::literal(
                NativeId::new(value).map_err(|_| ())?,
            ))
        })
        .collect()
}

fn lifecycle_operation_id(
    kind: NativeLifecycleKind,
    target: &HarnessId,
    resource: &ResourceKey,
) -> OperationId {
    let label = format!("{kind:?}:{target}:{}", resource.id().as_str());
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in label.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    OperationId::new(format!("lifecycle:{}:{hash:016x}", target.as_str()))
        .expect("lifecycle operation id is valid")
}

fn previously_applied(
    state: Option<&StateDocument>,
    resource: &ResourceKey,
    operation: &OperationId,
) -> bool {
    state
        .and_then(|state| state.resources().get(resource))
        .and_then(|state| state.last_apply())
        .and_then(|apply| apply.operations().get(operation))
        .is_some_and(|result| {
            matches!(
                result.outcome(),
                OperationOutcome::Applied | OperationOutcome::NoChange
            )
        })
}

fn operation_result_status(outcome: &OperationOutcome) -> &'static str {
    match outcome {
        OperationOutcome::Applied => "applied",
        OperationOutcome::NoChange => "no_change",
        OperationOutcome::Failed { .. } => "failed",
        OperationOutcome::Blocked { .. } => "blocked",
        OperationOutcome::SkippedDependency { .. } => "skipped_dependency",
        OperationOutcome::Pending => "pending",
    }
}

fn native_execution_error(error: &ExecutionError) -> ErrorDetail {
    let (code, summary) = match error {
        ExecutionError::Lock(_) => (
            "configuration_locked",
            "Another skilltap mutation holds the configuration lock.",
        ),
        ExecutionError::Release(_) => (
            "configuration_lock_release_failed",
            "The configuration lock could not be released safely.",
        ),
        ExecutionError::Revalidation { .. } => (
            "stale_native_evidence",
            "Native lifecycle evidence changed before mutation.",
        ),
        ExecutionError::Apply { .. } => (
            "native_command_failed",
            "The native lifecycle command failed.",
        ),
        ExecutionError::Journal { after_apply, .. } if *after_apply => (
            "state_journal_failed_after_apply",
            "Native work may have completed but state journaling failed; re-observe before retrying.",
        ),
        ExecutionError::Journal { .. } | ExecutionError::JournalBoundary { .. } => (
            "state_journal_failed",
            "The native operation result could not be recorded safely.",
        ),
        ExecutionError::InvalidOutcome { .. } => (
            "native_outcome_invalid",
            "The native lifecycle adapter returned an invalid operation outcome.",
        ),
        ExecutionError::Graph(_) | ExecutionError::Contract(_) => (
            "operation_plan_invalid",
            "The native lifecycle operation plan was invalid.",
        ),
    };
    ErrorDetail::new(code, summary)
}

fn seed_state_if_missing(
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
    for (key, seed) in seeds {
        if document.resources().contains_key(key) {
            continue;
        }
        document = document.with_resource_state(seed.clone()).map_err(|_| ())?;
    }
    repository.replace(&document).map_err(|_| ())
}

fn first_use_harness_report(
    config: &ConfigDocument,
    mut outcome: Outcome,
    mode: NativeObservationMode,
    requested: Option<&skilltap_core::domain::TargetSelection>,
) -> Outcome {
    let process_limits = ProcessLimits::new(5_000, 256 * 1024, 256 * 1024, 512 * 1024)
        .expect("bounded status process limits are valid");
    let json_limits =
        JsonLimits::new(256 * 1024, 64).expect("bounded status JSON limits are valid");
    let search_path = std::env::var_os("PATH");
    let all_harnesses = [
        HarnessId::new("codex").expect("known harness"),
        HarnessId::new("claude").expect("known harness"),
    ];
    let selected = skilltap_core::runtime::resolve_targets(requested, all_harnesses.clone())
        .unwrap_or_else(|_| {
            skilltap_core::domain::HarnessSet::new(all_harnesses).expect("known harnesses")
        });
    for (harness, kind, binary) in [
        (
            "codex",
            HarnessKind::Codex,
            config.harnesses().codex.binary.as_str(),
        ),
        (
            "claude",
            HarnessKind::Claude,
            config.harnesses().claude.binary.as_str(),
        ),
    ]
    .into_iter()
    .filter(|(harness, _, _)| selected.iter().any(|value| value.as_str() == *harness))
    {
        let mut entry = OutputEntry::new(harness, "not_enabled").with_field("enabled", false);
        if mode == NativeObservationMode::Disabled {
            outcome = outcome.with_resource(entry);
            continue;
        }
        let configured = match configured_binary(binary) {
            Ok(value) => value,
            Err(_) => {
                outcome = outcome.with_warning(
                    Warning::new(
                        "invalid_harness_binary",
                        "The configured harness binary could not be resolved.",
                    )
                    .with_context("harness", harness),
                );
                outcome = outcome.with_resource(entry.with_field("reachable", false));
                continue;
            }
        };
        match detect_configured_installation(
            kind,
            configured,
            search_path.clone(),
            process_limits,
            json_limits,
        ) {
            Ok(installation) => {
                if let HarnessReachability::Reachable { native_version, .. } =
                    installation.reachability()
                {
                    entry.status = "installed".to_owned();
                    entry = entry
                        .with_field("reachable", true)
                        .with_field("version", native_version.as_str());
                }
            }
            Err(error) => {
                entry.status = "unreachable".to_owned();
                entry = entry.with_field("reachable", false);
                outcome = outcome.with_warning(
                    Warning::new(
                        "native_detection_failed",
                        "The known harness could not be detected during first-use status.",
                    )
                    .with_context("harness", harness)
                    .with_context("detail", error.to_string()),
                );
            }
        }
        outcome = outcome.with_resource(entry);
    }
    outcome
}

struct DocumentLoadPhase {
    config: Result<DocumentState<ConfigDocument>, StorageError>,
    inventory: Result<DocumentState<InventoryDocument>, StorageError>,
    state: Result<DocumentState<StateDocument>, StorageError>,
}

impl DocumentLoadPhase {
    fn execute(application: &StatusApplication<'_>) -> Self {
        Self {
            config: application.config.load(),
            inventory: application.inventory.load(),
            state: application.state.load(),
        }
    }

    fn project(&self, outcome: Outcome) -> Outcome {
        let outcome = document_result(outcome, "config", &self.config);
        let outcome = document_result(outcome, "inventory", &self.inventory);
        document_result(outcome, "state", &self.state)
    }

    fn finish(self) -> Result<StatusDocuments, Vec<ErrorDetail>> {
        let mut errors = Vec::new();
        if let Err(error) = &self.config {
            errors.push(storage_error(error));
        }
        if let Err(error) = &self.inventory {
            errors.push(storage_error(error));
        }
        if let Err(error) = &self.state {
            errors.push(storage_error(error));
        }
        if !errors.is_empty() {
            return Err(errors);
        }

        Ok(StatusDocuments {
            config: match self.config.expect("checked above") {
                DocumentState::Missing => ConfigDocument::defaults(),
                DocumentState::Present(config) => config,
            },
            inventory: match self.inventory.expect("checked above") {
                DocumentState::Missing => None,
                DocumentState::Present(inventory) => Some(inventory),
            },
            state: match self.state.expect("checked above") {
                DocumentState::Missing => None,
                DocumentState::Present(state) => Some(state),
            },
        })
    }
}

struct StatusDocuments {
    config: ConfigDocument,
    inventory: Option<InventoryDocument>,
    state: Option<StateDocument>,
}

struct StatusScope {
    output: OutputScope,
    count: u64,
    resolved: Vec<Scope>,
}

impl StatusScope {
    fn resolve(
        application: &StatusApplication<'_>,
        args: &StatusArgs,
        documents: &StatusDocuments,
    ) -> Result<Self, ErrorDetail> {
        let request = application.scope_request(args, documents.inventory.as_ref())?;
        let scopes = application.scopes.resolve(&request).map_err(|_| {
            ErrorDetail::new(
                "project_scope_unavailable",
                "The requested project scope could not be resolved.",
            )
        })?;
        let resolved = scopes.into_scopes();
        Ok(Self {
            output: output_scope(&args.scope.argument(), &resolved),
            count: resolved.len() as u64,
            resolved,
        })
    }
}

struct StatusTargets {
    resolved: HarnessSet,
}

enum StatusTargetError {
    NoneEnabled,
    NotEnabled,
}

impl StatusTargets {
    fn resolve(args: &StatusArgs, documents: &StatusDocuments) -> Result<Self, StatusTargetError> {
        let enabled = enabled_harnesses(&documents.config);
        if enabled.is_empty() {
            return Err(StatusTargetError::NoneEnabled);
        }
        resolve_targets(args.target.target.as_ref(), enabled)
            .map(|resolved| Self { resolved })
            .map_err(|_| StatusTargetError::NotEnabled)
    }

    fn iter(&self) -> impl ExactSizeIterator<Item = &HarnessId> {
        self.resolved.iter()
    }
}

struct StatusProjection<'a> {
    documents: &'a StatusDocuments,
    scope: &'a StatusScope,
    targets: &'a StatusTargets,
    native_observation: NativeObservationMode,
}

impl StatusProjection<'_> {
    fn apply(self, mut outcome: Outcome) -> Outcome {
        for target in self.targets.iter() {
            outcome = outcome.with_resource(OutputEntry::new(target.as_str(), "selected"));
        }
        let observation = match self.native_observation {
            NativeObservationMode::Disabled => NativeObservation::default(),
            NativeObservationMode::System => {
                NativeObservation::run(self.documents, self.scope, self.targets)
            }
        };
        for resource in observation.resources.iter().cloned() {
            outcome = outcome.with_resource(resource);
        }
        let (update_entries, update_warnings, available_updates) =
            status_update_projection(self.documents, self.scope, self.targets, &observation);
        for warning in observation.warnings.iter().cloned() {
            outcome = outcome.with_warning(warning);
        }
        for entry in update_entries {
            outcome = outcome.with_resource(entry);
        }
        for warning in update_warnings {
            outcome = outcome.with_warning(warning);
        }
        if let Some(state) = self.documents.state.as_ref() {
            outcome = outcome.with_resource(daemon_status_projection(state.daemon_run()));
        }
        if observation.failed_targets == 0 {
            outcome.result = ResultClass::Completed;
        }
        let mut outcome = outcome
            .with_summary(
                "desired_resources",
                self.documents
                    .inventory
                    .as_ref()
                    .map_or(0, |value| value.resources().len() as u64),
            )
            .with_summary(
                "recorded_resources",
                self.documents
                    .state
                    .as_ref()
                    .map_or(0, |value| value.resources().len() as u64),
            )
            .with_summary("scopes", self.scope.count)
            .with_summary("targets", self.targets.iter().len() as u64)
            .with_summary("observed_targets", observation.observed_targets as u64)
            .with_summary("failed_targets", observation.failed_targets as u64)
            .with_summary("native_entries", observation.native_entries as u64)
            .with_summary("available_updates", available_updates as u64);
        let desired_keys = self
            .documents
            .inventory
            .as_ref()
            .map(|inventory| {
                inventory
                    .resources()
                    .keys()
                    .collect::<std::collections::BTreeSet<_>>()
            })
            .unwrap_or_default();
        let recorded_keys = self
            .documents
            .state
            .as_ref()
            .map(|state| {
                state
                    .resources()
                    .keys()
                    .collect::<std::collections::BTreeSet<_>>()
            })
            .unwrap_or_default();
        if desired_keys != recorded_keys {
            outcome.result = ResultClass::AttentionRequired;
            outcome = outcome.with_warning(Warning::new(
                "resource.drifted",
                "Desired inventory and recorded state differ; no mutation is implied by status.",
            ));
        }
        if outcome
            .warnings
            .iter()
            .any(|warning| warning.code == "capability.unverified")
        {
            outcome.result = ResultClass::AttentionRequired;
        }
        if observation.failed_targets > 0 {
            outcome = outcome.with_next_action(NextAction::new(
                "inspect_native_observation",
                "Inspect the reported native observation warnings before planning changes.",
            ));
        }
        let desired_resources = self
            .documents
            .inventory
            .as_ref()
            .map_or(0, |value| value.resources().len());
        let recorded_resources = self
            .documents
            .state
            .as_ref()
            .map_or(0, |value| value.resources().len());
        if observation.failed_targets == 0 && (desired_resources > 0 || recorded_resources > 0) {
            outcome.result = ResultClass::AttentionRequired;
            outcome = outcome.with_warning(Warning::new(
                "status_comparison_unavailable",
                "Resource-level desired comparison is not available for the observed native surfaces; review is required before planning changes.",
            ));
        }
        outcome
    }
}

fn daemon_status_projection(
    record: Option<&skilltap_core::storage::DaemonRunRecord>,
) -> OutputEntry {
    let Some(record) = record else {
        return OutputEntry::new("daemon", "never_run");
    };
    let status = match record.result() {
        skilltap_core::storage::DaemonRunResult::Completed => "completed",
        skilltap_core::storage::DaemonRunResult::Pending => "pending",
        skilltap_core::storage::DaemonRunResult::Contended => "contended",
        skilltap_core::storage::DaemonRunResult::Failed => "failed",
    };
    let mut entry = OutputEntry::new("daemon", status)
        .with_field("last_run_seconds", record.at().seconds())
        .with_field("safe_operations", record.safe_operations())
        .with_field("pending_operations", record.pending_operations());
    if let Some(code) = record.failure_code() {
        entry = entry.with_field("failure", code.as_str());
    }
    entry
}

struct UnavailableSourceRevisionResolver;

impl SourceRevisionResolver for UnavailableSourceRevisionResolver {
    fn resolve(
        &self,
        source: &skilltap_core::domain::Source,
    ) -> Result<skilltap_core::domain::ResolvedRevision, ResolutionError> {
        Err(ResolutionError::UnsupportedSourceKind(source.kind()))
    }
}

fn status_update_projection(
    documents: &StatusDocuments,
    scope: &StatusScope,
    targets: &StatusTargets,
    observation: &NativeObservation,
) -> (Vec<OutputEntry>, Vec<Warning>, usize) {
    let Some(inventory) = documents.inventory.as_ref() else {
        return (Vec::new(), Vec::new(), 0);
    };
    let Some(environment) = observation.environment.as_ref() else {
        return (Vec::new(), Vec::new(), 0);
    };
    let process_limits = ProcessLimits::new(5_000, 256 * 1024, 256 * 1024, 512 * 1024)
        .expect("bounded update resolution process limits are valid");
    let git_resolver = GitSourceRevisionResolver::system(process_limits).ok();
    let native_resolver = ObservedNativeRevisionResolver::new(environment);
    let fallback_resolver = UnavailableSourceRevisionResolver;
    let mut entries = Vec::new();
    let mut warnings = Vec::new();
    let mut available_updates = 0;
    let update_mode = documents.config.updates().mode;
    for resource in inventory.resources().values().filter(|resource| {
        scope.resolved.contains(resource.scope())
            && resource
                .targets()
                .iter()
                .any(|target| targets.resolved.contains(target))
    }) {
        let installed = documents
            .state
            .as_ref()
            .and_then(|state| state.resources().get(resource.key()))
            .and_then(|state| state.installed_revision());
        if update_mode == skilltap_core::storage::UpdateMode::Off
            || resource.update() == UpdateIntent::Disabled
        {
            let candidate = UpdateCandidate {
                resource: resource.key().clone(),
                current_revision: installed.cloned(),
                available_revision: None,
                resolution_error: None,
                pinned: resource.update() == UpdateIntent::Pinned,
                drifted: false,
                compatibility_changed: false,
                requires_acknowledgment: false,
                intent: resource.update(),
                acknowledgment_selectors: BTreeSet::new(),
            };
            let decision = classify_update_with_mode(&candidate, update_mode);
            entries.push(update_projection_entry(resource, &candidate, decision));
            continue;
        }
        let request = UpdateResolutionRequest {
            resource,
            installed,
            drifted: false,
            compatibility_changed: false,
            requires_acknowledgment: false,
        };
        let resolved = if resource.source().is_some() {
            match git_resolver.as_ref() {
                Some(resolver) => resolve_candidate(resolver, &native_resolver, request),
                None => resolve_candidate(&fallback_resolver, &native_resolver, request),
            }
        } else {
            resolve_candidate(&fallback_resolver, &native_resolver, request)
        };
        if let Some(error) = resolved.error.as_ref() {
            warnings.push(
                Warning::new(
                    "update_resolution_unavailable",
                    "An available revision could not be resolved without mutation.",
                )
                .with_context("resource", resource.key().to_string())
                .with_context("reason", resolution_error_label(error)),
            );
        }
        let candidate = candidate_for(resource, &request, &resolved);
        let decision = classify_update_with_mode(&candidate, update_mode);
        if decision.safety != UpdateSafety::NoUpdate {
            available_updates += 1;
        }
        entries.push(update_projection_entry(resource, &candidate, decision));
    }
    (entries, warnings, available_updates)
}

fn update_projection_entry(
    resource: &DesiredResource,
    candidate: &UpdateCandidate,
    decision: UpdateDecision,
) -> OutputEntry {
    let status = match (decision.safety, decision.reason) {
        (UpdateSafety::NoUpdate, Some(UpdateDecisionReason::DisabledResource)) => "disabled",
        (UpdateSafety::Blocked, Some(UpdateDecisionReason::GlobalModeOff)) => "policy_off",
        (UpdateSafety::NeedsDecision, Some(UpdateDecisionReason::CheckOnly)) => "check_only",
        (UpdateSafety::NeedsDecision, Some(UpdateDecisionReason::PinnedResource)) => "pinned",
        (UpdateSafety::NoUpdate, _) => "up_to_date",
        (UpdateSafety::Safe, _) => "safe",
        (UpdateSafety::NeedsDecision, _) => "needs_decision",
        (UpdateSafety::Blocked, _) => "blocked",
    };
    let mut entry = OutputEntry::new(format!("update:{}", resource.key()), status)
        .with_field("resource", resource.key().to_string());
    if let Some(reason) = decision.reason {
        entry = entry.with_field("reason", update_decision_reason_label(reason));
    }
    if let Some(current) = candidate.current_revision.as_ref() {
        entry = entry.with_field("current", revision_label(current));
    }
    if let Some(available) = candidate.available_revision.as_ref() {
        entry = entry.with_field("available", revision_label(available));
    }
    entry
}

fn update_decision_reason_label(reason: UpdateDecisionReason) -> &'static str {
    match reason {
        UpdateDecisionReason::DisabledResource => "disabled_resource",
        UpdateDecisionReason::GlobalModeOff => "global_mode_off",
        UpdateDecisionReason::CheckOnly => "check_only",
        UpdateDecisionReason::PinnedResource => "pinned_resource",
        UpdateDecisionReason::Drifted => "drifted",
        UpdateDecisionReason::CompatibilityChanged => "compatibility_changed",
        UpdateDecisionReason::AcknowledgmentRequired => "acknowledgment_required",
        UpdateDecisionReason::ResolutionFailed => "resolution_failed",
    }
}

fn resolution_error_label(error: &ResolutionError) -> &'static str {
    match error {
        ResolutionError::UnreachableSource => "unreachable_source",
        ResolutionError::InvalidRequestedRevision => "invalid_requested_revision",
        ResolutionError::UnsupportedSourceKind(_) => "unsupported_source_kind",
        ResolutionError::NativeObservationUnavailable => "native_observation_unavailable",
        ResolutionError::TargetDisagreement => "target_disagreement",
    }
}

fn revision_label(revision: &skilltap_core::domain::ResolvedRevision) -> String {
    match revision {
        skilltap_core::domain::ResolvedRevision::GitCommit(commit) => {
            format!("git:{}", commit.as_str())
        }
        skilltap_core::domain::ResolvedRevision::Native(native) => {
            format!("native:{}", native.as_str())
        }
    }
}

#[derive(Default)]
struct NativeObservation {
    resources: Vec<OutputEntry>,
    warnings: Vec<Warning>,
    observed_targets: usize,
    failed_targets: usize,
    native_entries: usize,
    environment: Option<skilltap_core::domain::ObservedEnvironment>,
}

impl NativeObservation {
    fn run(documents: &StatusDocuments, scope: &StatusScope, targets: &StatusTargets) -> Self {
        let paths = match PlatformPaths::resolve(&ProcessEnvironment) {
            Ok(paths) => paths,
            Err(_) => {
                return Self {
                    resources: Vec::new(),
                    warnings: vec![Warning::new(
                        "native_paths_unavailable",
                        "Native harness paths could not be resolved for read-only status.",
                    )],
                    observed_targets: 0,
                    failed_targets: targets.iter().len() * scope.resolved.len(),
                    native_entries: 0,
                    environment: None,
                };
            }
        };

        let process_limits = ProcessLimits::new(5_000, 256 * 1024, 256 * 1024, 512 * 1024)
            .expect("bounded status process limits are valid");
        let json_limits =
            JsonLimits::new(256 * 1024, 64).expect("bounded status JSON limits are valid");
        let tree_limits =
            ExternalTreeLimits::new(16, 10_000, 4 * 1024 * 1024, 64 * 1024 * 1024, 64 * 1024)
                .expect("bounded status tree limits are valid");
        let search_path = std::env::var_os("PATH");
        let mut result = Self::default();
        let mut requests = Vec::new();
        let mut outcomes = Vec::new();
        let mut metadata = Vec::new();

        for target in targets.iter() {
            let (kind, binary) = match target.as_str() {
                "codex" => (
                    HarnessKind::Codex,
                    documents.config.harnesses().codex.binary.as_str(),
                ),
                "claude" => (
                    HarnessKind::Claude,
                    documents.config.harnesses().claude.binary.as_str(),
                ),
                _ => {
                    result.failed_targets += scope.resolved.len();
                    result.warnings.push(
                        Warning::new(
                            "unsupported_harness",
                            "The selected harness is not supported.",
                        )
                        .with_context("harness", target.as_str()),
                    );
                    continue;
                }
            };
            let configured = match configured_binary(binary) {
                Ok(binary) => binary,
                Err(_) => {
                    result.failed_targets += scope.resolved.len();
                    result.warnings.push(
                        Warning::new(
                            "invalid_harness_binary",
                            "The configured harness binary could not be resolved.",
                        )
                        .with_context("harness", target.as_str()),
                    );
                    continue;
                }
            };
            let installation = match detect_configured_installation(
                kind,
                configured,
                search_path.clone(),
                process_limits,
                json_limits,
            ) {
                Ok(installation) => installation,
                Err(error) => {
                    result.failed_targets += scope.resolved.len();
                    result.resources.extend(scope.resolved.iter().map(|scope| {
                        OutputEntry::new(observation_id(target, scope), "unreachable")
                            .with_field("harness", target.as_str())
                            .with_field("scope", scope_label(scope))
                    }));
                    result.warnings.push(
                        Warning::new(
                            "native_detection_failed",
                            "The configured harness could not be detected.",
                        )
                        .with_context("harness", target.as_str())
                        .with_context("detail", error.to_string()),
                    );
                    continue;
                }
            };

            let HarnessReachability::Reachable { native_version, .. } = installation.reachability()
            else {
                result.failed_targets += scope.resolved.len();
                continue;
            };
            let profile = select_profile(kind, native_version);
            for current_scope in &scope.resolved {
                let evidence = match ObservationEvidence::new(&installation, profile.clone()) {
                    Ok(value) => value,
                    Err(_) => {
                        result.failed_targets += 1;
                        continue;
                    }
                };
                let request = ObservationRequest::new(current_scope.clone(), evidence);
                requests.push(request.clone());
                metadata.push((
                    target.clone(),
                    current_scope.clone(),
                    native_version.clone(),
                    profile.clone(),
                ));
                match observe_trees(kind, &paths, current_scope, tree_limits) {
                    Ok(snapshots) => {
                        let mut resources = snapshots
                            .iter()
                            .map(|snapshot| {
                                native_surface_resource(
                                    target,
                                    current_scope,
                                    &snapshot.root,
                                    profile.authority(),
                                    snapshot.snapshot.entries().len(),
                                )
                            })
                            .collect::<Vec<_>>();
                        resources.extend(
                            instruction_surface_labels(kind, &paths, current_scope)
                                .into_iter()
                                .map(|root| {
                                    native_surface_resource(
                                        target,
                                        current_scope,
                                        root,
                                        profile.authority(),
                                        0,
                                    )
                                }),
                        );
                        let findings = if profile.authority() == ProfileAuthority::ObserveOnly {
                            vec![ObservationFinding::new(
                                ObservationFindingCode::CapabilityUnverified,
                                ObservationSummary::CapabilityUnverified,
                                ObservationSeverity::Warning,
                                ObservationSubject::Harness {
                                    harness: target.clone(),
                                    scope: current_scope.clone(),
                                },
                                ObservationFields::default(),
                            )]
                        } else {
                            Vec::new()
                        };
                        match HarnessObservation::new(request.clone(), resources, findings) {
                            Ok(observation) => {
                                outcomes.push(HarnessObservationOutcome::observed(observation));
                            }
                            Err(_) => outcomes.push(HarnessObservationOutcome::failed(
                                request,
                                ObservationAdapterError::NativeShapeUnsupported {},
                            )),
                        }
                    }
                    Err(error) => outcomes.push(HarnessObservationOutcome::failed(
                        request,
                        observation_error(error),
                    )),
                }
            }
        }
        let batch = match ObservationBatch::new(requests) {
            Ok(batch) => batch,
            Err(_) => {
                result.warnings.push(Warning::new(
                    "native_observation_contract",
                    "Native observations could not be normalized safely.",
                ));
                result.failed_targets += metadata.len();
                return result;
            }
        };
        let environment = match normalize_observations(batch, outcomes) {
            Ok(environment) => environment,
            Err(_) => {
                result.warnings.push(Warning::new(
                    "native_observation_contract",
                    "Native observations could not be normalized safely.",
                ));
                result.failed_targets += metadata.len();
                return result;
            }
        };
        result.environment = Some(environment.clone());
        for (_, outcome) in environment.iter() {
            match outcome {
                HarnessObservationOutcome::Observed { observation } => {
                    result.observed_targets += 1;
                    let evidence = observation.request().evidence();
                    let profile = evidence.profile();
                    let target = observation.target();
                    let scope = target.scope();
                    result.resources.push(
                        OutputEntry::new(
                            observation_id(target.harness(), target.scope()),
                            "observed",
                        )
                        .with_field("harness", target.harness().as_str())
                        .with_field("scope", scope_label(target.scope()))
                        .with_field("typed", true)
                        .with_field("version", evidence.native_version().as_str())
                        .with_field("profile_authority", profile_authority(profile.authority()))
                        .with_field(
                            "capabilities_supported",
                            capability_count(profile, scope, CapabilitySupport::Supported) as u64,
                        )
                        .with_field(
                            "capabilities_unverified",
                            capability_count(profile, scope, CapabilitySupport::Unverified) as u64,
                        )
                        .with_field(
                            "capabilities_unsupported",
                            capability_count(profile, scope, CapabilitySupport::Unsupported) as u64,
                        )
                        .with_field("native_entries", observation.resources().len() as u64),
                    );
                    for resource in observation.resources().values() {
                        let mut entry = OutputEntry::new(
                            resource_identity(resource),
                            resource_health(resource.health()),
                        )
                        .with_field("harness", observation.target().harness().as_str())
                        .with_field("scope", scope_label(resource.scope()))
                        .with_field("kind", resource_kind(resource.kind()))
                        .with_field("native_identity", resource.native_identity().as_str());
                        if resource.native_identity().as_str().contains(".") {
                            entry = entry.with_field("native_entries", 0_u64);
                        }
                        result.resources.push(entry);
                        result.native_entries += 1;
                    }
                    for finding in observation.findings() {
                        result.warnings.push(finding_warning(finding));
                    }
                }
                HarnessObservationOutcome::Failed { request, error } => {
                    result.failed_targets += 1;
                    result.resources.push(
                        OutputEntry::new(
                            observation_id(request.target().harness(), request.scope()),
                            "observation_failed",
                        )
                        .with_field("harness", request.target().harness().as_str())
                        .with_field("scope", scope_label(request.scope())),
                    );
                    result.warnings.push(
                        Warning::new(
                            "native_observation_failed",
                            "Native harness state could not be observed within the safety limits.",
                        )
                        .with_context("harness", request.target().harness().as_str())
                        .with_context("scope", scope_label(request.scope()))
                        .with_context("detail", error.to_string()),
                    );
                }
            }
        }
        result
    }
}

fn project_adoption(
    mut outcome: Outcome,
    plan: &skilltap_core::adoption::AdoptionPlan,
    changed: bool,
    failed_targets: usize,
) -> Outcome {
    let mut adopted = 0_u64;
    let mut coalesced = 0_u64;
    let mut already_managed = 0_u64;
    let mut attention = failed_targets > 0;
    for decision in &plan.decisions {
        let (key, status) = match decision {
            AdoptionDecision::Adopted(candidate) => {
                adopted += 1;
                (candidate.desired.key(), "adopted")
            }
            AdoptionDecision::Coalesced(candidate) => {
                coalesced += 1;
                (candidate.desired.key(), "coalesced")
            }
            AdoptionDecision::AlreadyManaged { key } => {
                already_managed += 1;
                (key, "already_managed")
            }
            AdoptionDecision::Conflict { key, .. } => {
                attention = true;
                (key, "conflict")
            }
            AdoptionDecision::Unadoptable { key, .. } => {
                attention = true;
                (key, "unadoptable")
            }
            AdoptionDecision::Unchanged { key } => (key, "unchanged"),
        };
        outcome = outcome.with_resource(OutputEntry::new(key.to_string(), status));
    }
    outcome = outcome
        .with_summary("adopted", adopted)
        .with_summary("coalesced", coalesced)
        .with_summary("already_managed", already_managed)
        .with_summary("changed", changed);
    if attention {
        outcome.result = ResultClass::AttentionRequired;
        if failed_targets > 0 {
            outcome = outcome.with_warning(Warning::new(
                "partial_native_observation",
                "Some selected harness scopes could not be observed; only validated siblings were considered.",
            ));
        }
    } else {
        outcome.result = ResultClass::Completed;
    }
    outcome
}

fn adoption_apply_error(error: &AdoptionApplyError) -> ErrorDetail {
    let code = match error {
        AdoptionApplyError::Lock(_) => "configuration_locked",
        AdoptionApplyError::Inventory(_) => "inventory_unavailable",
        AdoptionApplyError::Observation(_) => "native_observation_unavailable",
        AdoptionApplyError::StaleEvidence => "stale_observation",
        AdoptionApplyError::Plan(_) => "adoption_plan_invalid",
        AdoptionApplyError::Release(_) => "configuration_lock_release_failed",
    };
    let summary = match error {
        AdoptionApplyError::Lock(_) => "Another skilltap mutation holds the configuration lock.",
        AdoptionApplyError::Inventory(_) => {
            "The skilltap inventory could not be safely loaded or published."
        }
        AdoptionApplyError::Observation(_) => {
            "Native resources could not be re-observed safely before publication."
        }
        AdoptionApplyError::StaleEvidence => {
            "Native adoption evidence changed before publication; no inventory was written."
        }
        AdoptionApplyError::Plan(_) => {
            "The refreshed native observations no longer form a safe adoption plan."
        }
        AdoptionApplyError::Release(_) => {
            "The configuration lock could not be released after adoption."
        }
    };
    ErrorDetail::new(code, summary)
}

fn adoption_next_action(error: &AdoptionApplyError) -> NextAction {
    match error {
        AdoptionApplyError::Lock(_) => NextAction::new(
            "retry_after_lock",
            "Wait for the other skilltap mutation to finish, then retry adoption.",
        ),
        AdoptionApplyError::StaleEvidence => NextAction::new(
            "reobserve_and_retry",
            "Review native changes and retry adoption to use fresh evidence.",
        ),
        _ => NextAction::new(
            "repair_and_retry",
            "Resolve the reported adoption boundary error and retry.",
        ),
    }
}

fn configured_binary(binary: &str) -> Result<ConfiguredBinary, ()> {
    if Path::new(binary).is_absolute() {
        AbsolutePath::new(binary)
            .map(ConfiguredBinary::absolute)
            .map_err(|_| ())
    } else {
        let id = skilltap_core::domain::NativeId::new(binary).map_err(|_| ())?;
        ConfiguredBinary::path_lookup(id).map_err(|_| ())
    }
}

fn observe_trees(
    kind: HarnessKind,
    paths: &PlatformPaths,
    scope: &Scope,
    limits: ExternalTreeLimits,
) -> Result<Vec<CanonicalObservation>, skilltap_core::runtime::ObservationRuntimeError> {
    match kind {
        HarnessKind::Codex => {
            let inputs =
                skilltap_harnesses::codex_observation_paths(paths, scope).map_err(|_| {
                    skilltap_core::runtime::ObservationRuntimeError::TreeRootUnavailable
                })?;
            observe_codex_canonical_resources(&inputs, scope, limits)
        }
        HarnessKind::Claude => {
            let inputs =
                skilltap_harnesses::claude_observation_paths(paths, scope).map_err(|_| {
                    skilltap_core::runtime::ObservationRuntimeError::TreeRootUnavailable
                })?;
            observe_claude_canonical_resources(&inputs, scope, limits)
        }
    }
}

fn instruction_surface_labels(
    kind: HarnessKind,
    paths: &PlatformPaths,
    scope: &Scope,
) -> Vec<&'static str> {
    match kind {
        HarnessKind::Codex => {
            let inputs = match skilltap_harnesses::codex_observation_paths(paths, scope) {
                Ok(inputs) => inputs,
                Err(_) => return Vec::new(),
            };
            match scope {
                Scope::Global => {
                    let mut labels = Vec::new();
                    if path_exists(inputs.global_agents.as_str()) {
                        labels.push("codex.global.instructions");
                    }
                    if child_path_exists(paths.home().as_str(), ".agents/plugins/marketplace.json")
                    {
                        labels.push("codex.global.marketplace");
                    }
                    if child_path_exists(paths.codex_home().as_str(), "config.toml") {
                        labels.push("codex.global.config");
                    }
                    labels
                }
                Scope::Project(_) => {
                    let mut labels = Vec::new();
                    let project = match scope {
                        Scope::Project(project) => project,
                        Scope::Global => unreachable!(),
                    };
                    if inputs
                        .project_agents
                        .as_ref()
                        .is_some_and(|path| path_exists(path.as_str()))
                    {
                        labels.push("project.agents.instructions");
                    }
                    if inputs
                        .project_override
                        .as_ref()
                        .is_some_and(|path| path_exists(path.as_str()))
                    {
                        labels.push("project.agents.override");
                    }
                    if child_path_exists(project.as_str(), ".agents/plugins/marketplace.json") {
                        labels.push("project.marketplace");
                    }
                    if child_path_exists(project.as_str(), ".codex/config.toml") {
                        labels.push("project.codex.config");
                    }
                    labels
                }
            }
        }
        HarnessKind::Claude => {
            let inputs = match skilltap_harnesses::claude_observation_paths(paths, scope) {
                Ok(inputs) => inputs,
                Err(_) => return Vec::new(),
            };
            match scope {
                Scope::Global => {
                    let mut labels = Vec::new();
                    if path_exists(inputs.global_settings.as_str()) {
                        labels.push("claude.settings");
                    }
                    if child_path_exists(
                        paths.claude_home().as_str(),
                        "plugins/known_marketplaces.json",
                    ) {
                        labels.push("claude.marketplace");
                    }
                    if child_path_exists(paths.claude_home().as_str(), "CLAUDE.md") {
                        labels.push("claude.instructions");
                    }
                    labels
                }
                Scope::Project(project) => {
                    let mut labels = Vec::new();
                    if inputs
                        .project_settings
                        .as_ref()
                        .is_some_and(|path| path_exists(path.as_str()))
                    {
                        labels.push("project.claude.settings");
                    }
                    if child_path_exists(project.as_str(), "CLAUDE.md")
                        || child_path_exists(project.as_str(), ".claude/CLAUDE.md")
                    {
                        labels.push("project.claude.instructions");
                    }
                    labels
                }
            }
        }
    }
}

fn path_exists(path: &str) -> bool {
    std::fs::symlink_metadata(path).is_ok()
}

fn child_path_exists(root: &str, child: &str) -> bool {
    path_exists(Path::new(root).join(child).to_string_lossy().as_ref())
}

fn native_surface_resource(
    harness: &HarnessId,
    scope: &Scope,
    root: &str,
    authority: ProfileAuthority,
    entries: usize,
) -> ObservedResource {
    let id = ResourceId::new(stable_resource_id(harness, root))
        .expect("stable native surface identifier is valid");
    let key = ResourceKey::new(id, scope.clone());
    let observation_key = ObservationKey::new(key, harness.clone(), ObservationLayer::Effective);
    let kind = native_surface_kind(root);
    let native_identity = NativeId::new(format!("{harness}:{root}:entries-{entries}"))
        .expect("native surface identity is valid");
    ObservedResource::new(
        observation_key,
        kind,
        Provenance::Native,
        Ownership::Unmanaged,
        if authority == ProfileAuthority::ObserveOnly {
            ResourceHealth::Unknown
        } else {
            ResourceHealth::Healthy
        },
        None,
        ComponentGraph::new([]).expect("empty component graph is valid"),
        BTreeSet::new(),
        native_identity,
        None,
        None,
    )
}

fn stable_resource_id(harness: &HarnessId, root: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in format!("{harness}:{root}").bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("native-{hash:016x}")
}

fn native_surface_kind(root: &str) -> ResourceKind {
    if root.ends_with("skills") {
        ResourceKind::StandaloneSkill
    } else if root.contains("marketplace") {
        ResourceKind::Marketplace
    } else if root.ends_with("plugins") || root.ends_with("claude") {
        ResourceKind::Plugin
    } else {
        ResourceKind::InstructionLocation
    }
}

fn observation_error(
    error: skilltap_core::runtime::ObservationRuntimeError,
) -> ObservationAdapterError {
    use skilltap_core::runtime::ObservationRuntimeError as RuntimeError;
    match error {
        RuntimeError::TreeDepthLimitExceeded
        | RuntimeError::TreeEntryLimitExceeded
        | RuntimeError::TreeFileLimitExceeded
        | RuntimeError::TreeTotalLimitExceeded
        | RuntimeError::TreeSymlinkTargetLimitExceeded => {
            ObservationAdapterError::ResourceLimitExceeded {}
        }
        RuntimeError::ProcessDeadlineExceeded => ObservationAdapterError::DeadlineExceeded {},
        RuntimeError::TreeEntryUnsupported
        | RuntimeError::TreeEntryNonUtf8
        | RuntimeError::DuplicateTreeEntry => ObservationAdapterError::NativeShapeUnsupported {},
        _ => ObservationAdapterError::NativeStateUnreadable {},
    }
}

fn resource_identity(resource: &ObservedResource) -> String {
    format!(
        "{}:{}:{}",
        resource.key().harness(),
        resource.key().resource().id(),
        match resource.key().layer() {
            ObservationLayer::Declared => "declared",
            ObservationLayer::Effective => "effective",
        }
    )
}

fn resource_health(health: ResourceHealth) -> &'static str {
    match health {
        ResourceHealth::Healthy => "healthy",
        ResourceHealth::Drifted => "drifted",
        ResourceHealth::Degraded => "degraded",
        ResourceHealth::Unknown => "unknown",
    }
}

fn resource_kind(kind: ResourceKind) -> &'static str {
    match kind {
        ResourceKind::Harness => "harness",
        ResourceKind::Marketplace => "marketplace",
        ResourceKind::Plugin => "plugin",
        ResourceKind::StandaloneSkill => "standalone_skill",
        ResourceKind::InstructionLocation => "instruction_location",
    }
}

fn profile_authority(authority: ProfileAuthority) -> &'static str {
    match authority {
        ProfileAuthority::VerifiedCompiled => "verified_compiled",
        ProfileAuthority::ObserveOnly => "observe_only",
    }
}

fn capability_count(
    profile: &skilltap_core::domain::CapabilityProfileSelection,
    scope: &Scope,
    support: CapabilitySupport,
) -> usize {
    profile
        .observation_capabilities()
        .for_scope(scope)
        .iter()
        .filter(|(_, value)| *value == support)
        .count()
}

fn finding_warning(finding: &ObservationFinding) -> Warning {
    Warning::new(finding.code().as_str(), finding.summary().as_str()).with_context(
        "severity",
        format!("{:?}", finding.severity()).to_lowercase(),
    )
}

fn scope_label(scope: &Scope) -> String {
    match scope {
        Scope::Global => "global".to_owned(),
        Scope::Project(path) => path.as_str().to_owned(),
    }
}

fn observation_id(harness: &HarnessId, scope: &Scope) -> String {
    format!("{}:{}", harness, scope_label(scope))
}

fn document_result<T>(
    outcome: Outcome,
    name: &str,
    result: &Result<DocumentState<T>, StorageError>,
) -> Outcome {
    let status = match result {
        Ok(DocumentState::Missing) if name == "config" => "missing; using defaults",
        Ok(DocumentState::Missing) => "missing",
        Ok(DocumentState::Present(_)) => "valid",
        Err(_) => "invalid",
    };
    outcome.with_resource(OutputEntry::new(name, status))
}

fn storage_error(error: &StorageError) -> ErrorDetail {
    let (code, summary) = match error.failure() {
        StorageFailure::Runtime => (
            "owned_document_unreadable",
            "A skilltap-owned document could not be read safely.",
        ),
        StorageFailure::Malformed => (
            "owned_document_malformed",
            "A skilltap-owned document is malformed.",
        ),
        StorageFailure::Invalid => (
            "owned_document_invalid",
            "A skilltap-owned document failed validation.",
        ),
        StorageFailure::UnsupportedSchema { .. } => (
            "owned_document_schema_unsupported",
            "A skilltap-owned document uses an unsupported schema version.",
        ),
    };
    ErrorDetail::new(code, summary)
        .with_context("document", error.document().to_string())
        .with_context("action", error.action().to_string())
}

fn enabled_harnesses(config: &ConfigDocument) -> Vec<HarnessId> {
    [
        ("codex", config.harnesses().codex.enabled),
        ("claude", config.harnesses().claude.enabled),
    ]
    .into_iter()
    .filter(|(_, enabled)| *enabled)
    .map(|(name, _)| HarnessId::new(name).expect("known harness identifier"))
    .collect()
}

fn output_scope(requested: &ScopeArgument, resolved: &[Scope]) -> OutputScope {
    match requested {
        ScopeArgument::Global => OutputScope::Global,
        ScopeArgument::AllScopes => OutputScope::All,
        ScopeArgument::Project(_) => OutputScope::Project {
            path: match resolved {
                [Scope::Project(path)] => path,
                _ => unreachable!("a resolved project request contains one project scope"),
            }
            .as_str()
            .to_owned(),
        },
    }
}

fn absolute_project_argument(
    argument: &Path,
    working_directory: &dyn WorkingDirectory,
) -> Result<AbsolutePath, ()> {
    let path = if argument.is_absolute() {
        argument.to_path_buf()
    } else {
        PathBuf::from(
            working_directory
                .current_directory()
                .map_err(|_| ())?
                .as_str(),
        )
        .join(argument)
    };
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::RootDir | Component::Prefix(_) | Component::Normal(_) => {
                normalized.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    return Err(());
                }
            }
        }
    }
    let value = normalized.to_str().ok_or(())?;
    AbsolutePath::new(value).map_err(|_| ())
}

#[cfg(test)]
mod tests;
