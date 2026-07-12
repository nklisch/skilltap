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
        OperationResult, OperationSelector, Ownership, Plan, ProfileAuthority, Provenance,
        ResourceHealth, ResourceId, ResourceKey, ResourceKind, Scope, Source, SourceKind,
        SourceLocator, UpdateIntent,
    },
    executor::{ExecutionError, ExecutionJournal, ExecutionPort, execute_plan},
    foreground_update::{
        ForegroundUpdateRequest, plan_foreground_updates,
        select_foreground_updates_with_acknowledgment,
    },
    instructions::fingerprint_contents,
    lifecycle_operation::native_operation,
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
    NativeLifecyclePort, NativeLifecycleRequest, NativeResourcePresence,
    ObservedNativeRevisionResolver, detect_configured_installation, native_arguments,
    normalize_observations, observe_claude_canonical_resources, observe_codex_canonical_resources,
    observe_native_resource, select_profile,
};

use crate::{
    ErrorDetail, NextAction, Outcome, OutputEntry, OutputScope, OutputValue, ResultClass, Warning,
    command::{
        AdoptArgs, OutputArgs, PlanArgs, ScopeArgs, ScopeArgument, ScopedOutputArgs,
        ScopedTargetArgs, StatusArgs, SyncArgs, TargetArgs,
    },
};

mod execution;
mod instructions;
mod lifecycle;

use execution::{
    InstructionEntry, InstructionPort, InstructionWrite, ManagedSkillAction, ManagedSkillEntry,
    ManagedSkillPort, StateExecutionJournal,
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
    pub(crate) preserve_name: bool,
    pub(crate) requested_revision: Option<&'a str>,
    pub(crate) subdirectory: Option<&'a str>,
}

/// State-backed journal for mutating lifecycle composition. It resolves each
/// result through the validated plan, seeds only explicitly planned resources,
/// updates exact resource records, and publishes atomically through the
/// repository port.
impl StatusApplication<'_> {
    /// Build a fresh reconciliation plan from the current documents and
    /// bounded native observation.  The desired inventory is the source of
    /// lifecycle candidates: each resource is projected onto its selected
    /// scope and harness target and then routed through the same adapter used
    /// by the corresponding explicit lifecycle command.
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

    /// Render a deterministic lifecycle preview while the resource-specific
    /// mutation adapter is unavailable. This keeps command output useful and
    /// safe: it never claims a native action happened and never mutates state.
    #[allow(dead_code)]
    /// Apply one native marketplace/plugin lifecycle request through the core
    /// lock, plan, bounded process, and state-journal boundaries.
    /// Install an explicit local complete skill tree into native skill paths.
    /// Git-backed sources deliberately remain a separate adapter until their
    /// clone/resolve boundary is available.
    /// Refresh a managed skill from the source recorded in state. The normal
    /// install path performs the bounded resolution and replacement planning;
    /// this command only supplies the recorded source identity.
    /// Remove a skill only when skilltap owns the exact current tree. An
    /// unmanaged or drifted tree is left untouched and reported for an
    /// explicit repair decision.
    /// Run instruction setup/repair for one selected harness when
    /// reconciliation targets a single bridge. Explicit instruction commands
    /// continue to use the all-enabled behavior above.
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
        if acknowledged {
            // Carry the generic foreground acknowledgment through the
            // reconciliation boundary. Resource adapters decide which
            // consequences are eligible; hard blocks and drift are never
            // bypassed by this flag.
            outcome = outcome.with_summary("acknowledged", true);
        }
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

        let desired = documents
            .inventory
            .as_ref()
            .map(|inventory| inventory.resources().values().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        let selected = reconciliation_selections(&desired, &scope, &targets, includes, excludes);
        let desired_count = selected
            .iter()
            .map(|(resource, _)| resource.key().to_string())
            .collect::<BTreeSet<_>>()
            .len() as u64;

        // `plan` is deliberately side-effect free.  Its candidates are
        // assembled by the same resource-to-lifecycle projection used by
        // `sync`, but rendered as planned operations instead of executing a
        // native command or publishing state.
        if command == "plan" {
            for (resource, target_id) in selected.iter().copied() {
                let child_scope = scope_args_for_scope(resource.scope());
                let (source, name) = reconciliation_source_and_name(resource);
                let child = match resource.kind() {
                    ResourceKind::Marketplace
                    | ResourceKind::Plugin
                    | ResourceKind::StandaloneSkill => self.execute_lifecycle_preview(
                        "plan",
                        &child_scope,
                        &TargetArgs {
                            target: Some(skilltap_core::domain::TargetSelection::Only(
                                target_id.clone(),
                            )),
                        },
                        resource.kind(),
                        source,
                        name,
                    ),
                    ResourceKind::InstructionLocation => self
                        .execute_instruction_reconciliation_preview(
                            &child_scope,
                            target_id,
                            resource,
                        ),
                    _ => Outcome::new("plan", ResultClass::Completed).with_operation(
                        crate::OperationOutcome::new(
                            format!("reconcile:{}:{}", target_id, resource.key()),
                            "planned",
                        )
                        .with_field("target", target_id.as_str())
                        .with_field("scope", scope_label(resource.scope())),
                    ),
                };
                merge_reconciliation_outcome(&mut outcome, child);
            }
            let operation_count = outcome.operations.len() as u64;
            if observation.failed_targets > 0 {
                outcome.result = ResultClass::AttentionRequired;
            } else if outcome.errors.is_empty() && outcome.warnings.is_empty() {
                // A plan is an attention result whenever it contains work for
                // the caller to inspect, even when every operation is safe or
                // already satisfied.  Only an empty plan is a completed
                // no-change result; sync keeps its normal applied/no-change
                // classification below.
                outcome.result = if operation_count > 0 {
                    ResultClass::AttentionRequired
                } else {
                    ResultClass::Completed
                };
            }
            return outcome
                .with_summary("desired_resources", desired_count)
                .with_summary("operations", operation_count)
                .with_summary("scopes", scope.count)
                .with_summary("targets", targets.iter().len() as u64)
                .with_summary("observed_targets", observation.observed_targets as u64)
                .with_summary("failed_targets", observation.failed_targets as u64)
                .with_summary("changed", false);
        }

        // `sync` delegates each candidate to its existing lifecycle adapter.
        // This keeps locking, revalidation, native process bounds, journaling,
        // and post-mutation observation identical to explicit commands.
        for (resource, target_id) in selected.iter().copied() {
            let child_scope = scope_args_for_scope(resource.scope());
            let (source, name) = reconciliation_source_and_name(resource);
            let child_target = TargetArgs {
                target: Some(skilltap_core::domain::TargetSelection::Only(
                    target_id.clone(),
                )),
            };
            let child = match resource.kind() {
                ResourceKind::Marketplace => self.execute_native_lifecycle(
                    "sync",
                    NativeLifecycleKind::MarketplaceAdd,
                    &child_scope,
                    &child_target,
                    source,
                    name,
                ),
                ResourceKind::Plugin => self.execute_native_lifecycle(
                    "sync",
                    NativeLifecycleKind::PluginInstall,
                    &child_scope,
                    &child_target,
                    name,
                    None,
                ),
                ResourceKind::StandaloneSkill => match source {
                    Some(source) => self.execute_skill_install(
                        "sync",
                        &child_scope,
                        &child_target,
                        acknowledged,
                        SkillInstallRequest {
                            source,
                            name,
                            preserve_name: true,
                            requested_revision: resource
                                .source()
                                .and_then(|value| value.requested_revision())
                                .map(|value| value.as_str()),
                            subdirectory: resource
                                .source()
                                .and_then(|value| value.subdirectory())
                                .map(|value| value.as_str()),
                        },
                    ),
                    None => Outcome::new("sync", ResultClass::AttentionRequired).with_warning(
                        Warning::new(
                            "skill_source_unavailable",
                            "The desired skill has no source locator and cannot be synchronized.",
                        ),
                    ),
                },
                ResourceKind::InstructionLocation => self.execute_instruction_setup_for_target(
                    "sync",
                    &child_scope,
                    None,
                    acknowledged,
                    true,
                    Some(target_id),
                ),
                ResourceKind::Harness => Outcome::new("sync", ResultClass::Completed),
            };
            merge_reconciliation_outcome(&mut outcome, child);
        }
        if observation.failed_targets > 0 {
            outcome.result = ResultClass::AttentionRequired;
        } else if outcome.errors.is_empty() && outcome.warnings.is_empty() {
            outcome.result = ResultClass::Completed;
        }
        let changed = outcome.summary.get("changed") == Some(&OutputValue::Boolean(true));
        let operation_count = outcome.operations.len() as u64;
        outcome
            .with_summary("desired_resources", desired_count)
            .with_summary("operations", operation_count)
            .with_summary("scopes", scope.count)
            .with_summary("targets", targets.iter().len() as u64)
            .with_summary("observed_targets", observation.observed_targets as u64)
            .with_summary("failed_targets", observation.failed_targets as u64)
            .with_summary("changed", changed)
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

fn skill_install_can_complete(outcome: &Outcome, acknowledged: bool) -> bool {
    outcome.errors.is_empty()
        && (outcome.warnings.is_empty()
            || (acknowledged
                && outcome
                    .warnings
                    .iter()
                    .all(|warning| warning.code == "skill_frontmatter_warning")))
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

/// A document-load phase starts outcomes in `attention_required` so malformed
/// owned documents cannot be mistaken for successful commands. A daemon cycle
/// that completes safe updates may inherit that provisional class even when
/// every child update completed cleanly. The absence of warnings/errors and
/// pending work is the evidence needed to normalize that aggregate result.
fn normalize_daemon_noop_result(
    outcome: &mut Outcome,
    safe_operations: u64,
    pending_operations: u64,
) {
    if safe_operations > 0
        && pending_operations == 0
        && outcome.warnings.is_empty()
        && outcome.errors.is_empty()
    {
        outcome.result = ResultClass::Completed;
    }
}

/// Merge one resource-scoped lifecycle result into the aggregate reconciliation
/// result.  Child adapters own their operation and journal details; the
/// reconciliation command only combines those already-rendered records.
fn merge_reconciliation_outcome(aggregate: &mut Outcome, child: Outcome) {
    let child_changed = child.summary.get("changed") == Some(&OutputValue::Boolean(true));
    aggregate.result = merge_result(aggregate.result, child.result);
    aggregate.resources.extend(child.resources);
    aggregate.operations.extend(child.operations);
    aggregate.warnings.extend(child.warnings);
    aggregate.errors.extend(child.errors);
    aggregate.next_actions.extend(child.next_actions);
    if child_changed {
        aggregate
            .summary
            .insert("changed".to_owned(), OutputValue::Boolean(true));
    }
}

fn reconciliation_selector_matches(
    resource: &DesiredResource,
    includes: &[NativeId],
    excludes: &[NativeId],
) -> bool {
    let id = resource.id().as_str();
    let included = includes.is_empty()
        || includes.iter().any(|selector| {
            selector.as_str() == id || selector.as_str() == resource.key().to_string()
        });
    let excluded = excludes
        .iter()
        .any(|selector| selector.as_str() == id || selector.as_str() == resource.key().to_string());
    included && !excluded
}

fn reconciliation_selections<'a>(
    desired: &'a [DesiredResource],
    scope: &StatusScope,
    targets: &StatusTargets,
    includes: &[NativeId],
    excludes: &[NativeId],
) -> Vec<(&'a DesiredResource, &'a HarnessId)> {
    desired
        .iter()
        .filter(|resource| {
            scope
                .resolved
                .iter()
                .any(|selected| selected == resource.scope())
                && reconciliation_selector_matches(resource, includes, excludes)
        })
        .flat_map(|resource| {
            resource
                .targets()
                .iter()
                .filter(|target| targets.resolved.contains(target))
                .map(move |target| (resource, target))
        })
        .collect()
}

fn reconciliation_source_and_name(resource: &DesiredResource) -> (Option<&str>, Option<&str>) {
    let source = resource.source().map(|source| source.locator().as_str());
    let name = match resource.kind() {
        ResourceKind::Marketplace => resource.id().as_str().strip_prefix("marketplace:"),
        ResourceKind::Plugin => resource.id().as_str().strip_prefix("plugin:"),
        ResourceKind::StandaloneSkill => resource.id().as_str().strip_prefix("skill:"),
        _ => None,
    };
    (source, name)
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

/// Resolve the materialized Claude bridge for a project using setup's
/// nested-only preservation policy. The inventory identity remains the
/// ordinary project bridge resource even when setup keeps `.claude/CLAUDE.md`.
fn preferred_instruction_bridge_path(
    filesystem: &dyn FileSystem,
    scope: &Scope,
    target: &HarnessId,
    root_bridge: AbsolutePath,
) -> AbsolutePath {
    if target.as_str() != "claude" {
        return root_bridge;
    }
    let Scope::Project(project) = scope else {
        return root_bridge;
    };
    let nested = AbsolutePath::new(format!("{}/.claude/CLAUDE.md", project.as_str()))
        .expect("nested project Claude bridge path is valid");
    let root_missing = filesystem
        .inspect(&root_bridge)
        .map(|metadata| metadata.kind() == FileKind::Missing)
        .unwrap_or(false);
    let nested_present = filesystem
        .inspect(&nested)
        .map(|metadata| metadata.kind() != FileKind::Missing)
        .unwrap_or(false);
    if root_missing && nested_present {
        nested
    } else {
        root_bridge
    }
}

fn instruction_resource_key(scope: &Scope, role: &str, target: &str) -> Option<ResourceKey> {
    let scope_label = match scope {
        Scope::Global => "global".to_owned(),
        Scope::Project(path) => format!("project-{:016x}", stable_hash(path.as_str())),
    };
    ResourceId::new(format!("instructions:{scope_label}:{role}:{target}"))
        .ok()
        .map(|id| ResourceKey::new(id, scope.clone()))
}

fn instruction_operation_id(scope: &Scope, role: &str, target: &str) -> OperationId {
    let resource = instruction_resource_key(scope, role, target)
        .expect("instruction resource identity is valid");
    let hash = stable_hash(resource.id().as_str());
    OperationId::new(format!("instructions:{hash:016x}"))
        .expect("instruction operation id is valid")
}

fn instruction_backup_path(paths: &PlatformPaths, bridge: &AbsolutePath) -> AbsolutePath {
    let hash = stable_hash(bridge.as_str());
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

fn is_option_like_git_value(value: &str) -> bool {
    value.starts_with('-')
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
    // Keep this private boundary defensive even though callers validate the
    // typed values first. Git treats leading-dash positional values as
    // options unless an argument delimiter is present.
    if is_option_like_git_value(locator.as_str())
        || requested_revision
            .as_ref()
            .is_some_and(|revision| is_option_like_git_value(revision.as_str()))
    {
        return Err(());
    }
    let source_root = AbsolutePath::new(format!(
        "{}/managed/sources",
        paths.skilltap_config().as_str()
    ))
    .map_err(|_| ())?;
    SystemFileSystem
        .create_directory_all(&source_root)
        .map_err(|_| ())?;
    let hash = stable_hash(locator.as_str());
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
                OsString::from("--"),
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
    if let Some(revision) = requested_revision {
        let fetch_revision = NativeProcessRequest::new(
            executable.clone(),
            [
                OsString::from("-C"),
                OsString::from(checkout.as_str()),
                OsString::from("fetch"),
                OsString::from("--depth"),
                OsString::from("1"),
                OsString::from("origin"),
                OsString::from("--"),
                OsString::from(revision.as_str()),
            ],
            BTreeMap::new(),
            None,
            limits,
        );
        let output = SystemNativeProcessRunner
            .run(&fetch_revision)
            .map_err(|_| ())?;
        if !output.status().success() {
            return Err(());
        }
    }
    let revision = requested_revision
        .map(|_| "FETCH_HEAD")
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
    let hash = stable_hash(&label);
    OperationId::new(format!("skill:{target}:{hash:016x}")).expect("skill operation id is valid")
}

fn skill_canonical_operation_id(resource: &ResourceKey) -> OperationId {
    let label = format!("skill-canonical:{}", resource.id().as_str());
    let hash = stable_hash(&label);
    OperationId::new(format!("skill-canonical:{hash:016x}"))
        .expect("canonical skill operation id is valid")
}

fn skill_remove_operation_id(target: &HarnessId, resource: &ResourceKey) -> OperationId {
    let label = format!("skill-remove:{target}:{}", resource.id().as_str());
    let hash = stable_hash(&label);
    OperationId::new(format!("skill-remove:{target}:{hash:016x}"))
        .expect("skill removal operation id is valid")
}

fn skill_canonical_remove_operation_id(resource: &ResourceKey) -> OperationId {
    let label = format!("skill-remove-canonical:{}", resource.id().as_str());
    let hash = stable_hash(&label);
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
    scope: &Scope,
    resource: &ResourceKey,
) -> OperationId {
    let label = format!(
        "{kind:?}:{target}:{}:{}",
        scope_label(scope),
        resource.id().as_str()
    );
    let hash = stable_hash(&label);
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

fn lifecycle_preview_presence(
    documents: &StatusDocuments,
    kind: ResourceKind,
    harness: &HarnessId,
    scope: &Scope,
    name: &str,
) -> NativeResourcePresence {
    let action = match kind {
        ResourceKind::Marketplace => NativeLifecycleAction::MarketplaceAdd,
        ResourceKind::Plugin => NativeLifecycleAction::PluginInstall,
        ResourceKind::StandaloneSkill
        | ResourceKind::InstructionLocation
        | ResourceKind::Harness => return NativeResourcePresence::Unknown,
    };
    let harness_kind = match harness.as_str() {
        "codex" => HarnessKind::Codex,
        "claude" => HarnessKind::Claude,
        _ => return NativeResourcePresence::Unknown,
    };
    let configured = match harness_kind {
        HarnessKind::Codex => configured_binary(documents.config.harnesses().codex.binary.as_str()),
        HarnessKind::Claude => {
            configured_binary(documents.config.harnesses().claude.binary.as_str())
        }
    };
    let Ok(configured) = configured else {
        return NativeResourcePresence::Unknown;
    };
    let Ok(name) = NativeId::new(name) else {
        return NativeResourcePresence::Unknown;
    };
    let request = NativeLifecycleRequest {
        harness: harness_kind,
        action,
        scope: scope.clone(),
        name,
        source: None,
    };
    let process_limits = ProcessLimits::new(5_000, 256 * 1024, 256 * 1024, 512 * 1024)
        .expect("bounded lifecycle process limits are valid");
    let json_limits =
        JsonLimits::new(256 * 1024, 64).expect("bounded lifecycle JSON limits are valid");
    observe_native_resource(
        configured,
        std::env::var_os("PATH"),
        &request,
        process_limits,
        json_limits,
    )
    .unwrap_or(NativeResourcePresence::Unknown)
}

fn lifecycle_presence_label(presence: NativeResourcePresence) -> &'static str {
    match presence {
        NativeResourcePresence::Present => "present",
        NativeResourcePresence::Missing => "missing",
        NativeResourcePresence::Unknown => "unknown",
    }
}

fn lifecycle_recorded_state(
    documents: &StatusDocuments,
    kind: ResourceKind,
    scope: &Scope,
    name: &str,
) -> bool {
    let prefix = match kind {
        ResourceKind::Marketplace => "marketplace:",
        ResourceKind::Plugin => "plugin:",
        ResourceKind::StandaloneSkill
        | ResourceKind::InstructionLocation
        | ResourceKind::Harness => return false,
    };
    let Ok(id) = ResourceId::new(format!("{prefix}{name}")) else {
        return false;
    };
    let key = ResourceKey::new(id, scope.clone());
    documents
        .state
        .as_ref()
        .is_some_and(|state| state.resources().contains_key(&key))
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
        if !document.resources().contains_key(key) {
            document = document.with_resource_state(seed.clone()).map_err(|_| ())?;
        }
    }
    repository.replace(&document).map_err(|_| ())
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
    for (key, seed) in seeds {
        if let Some(existing) = document.resources().get(key) {
            if state_seed_matches(existing, seed) {
                continue;
            }
            document = document
                .refresh_resource_state(seed.clone())
                .map_err(|_| ())?;
        } else {
            document = document.with_resource_state(seed.clone()).map_err(|_| ())?;
        }
    }
    repository.replace(&document).map_err(|_| ())
}

fn state_seed_matches(existing: &ResourceState, seed: &ResourceState) -> bool {
    existing.native_ids() == seed.native_ids()
        && existing.provenance() == seed.provenance()
        && existing.ownership() == seed.ownership()
        && existing.source() == seed.source()
        && existing.managed_artifact() == seed.managed_artifact()
        && existing.fingerprint() == seed.fingerprint()
        && existing.installed_revision() == seed.installed_revision()
        && existing.available_revision() == seed.available_revision()
}

fn project_inventory_targets(
    inventory: &InventoryDocument,
    keys: &BTreeSet<ResourceKey>,
    selected: &HarnessSet,
) -> Result<InventoryDocument, ()> {
    let mut next = inventory.clone();
    for key in keys {
        let Some(existing) = next.resources().get(key).cloned() else {
            continue;
        };
        let remaining = existing
            .targets()
            .iter()
            .filter(|target| !selected.contains(target))
            .cloned()
            .collect::<Vec<_>>();
        if remaining.is_empty() {
            next = next.without_resource(key).ok_or(())?;
        } else {
            let targets = HarnessSet::new(remaining).map_err(|_| ())?;
            let projected = existing.with_targets(targets).map_err(|_| ())?;
            next = next.replace_resource(projected).map_err(|_| ())?;
        }
    }
    Ok(next)
}

fn project_state_targets_after_remove(
    repository: &dyn StateRepository,
    keys: &BTreeSet<ResourceKey>,
    selected: &HarnessSet,
) -> Result<(), ()> {
    if keys.is_empty() {
        return Ok(());
    }
    let mut document = match repository.load().map_err(|_| ())? {
        DocumentState::Present(document) => document,
        DocumentState::Missing => return Ok(()),
    };
    let mut changed = false;
    for key in keys {
        let Some(existing) = document.resources().get(key).cloned() else {
            continue;
        };
        let mut native_ids = existing.native_ids().clone();
        native_ids.retain(|harness, _| !selected.contains(harness));
        if native_ids.is_empty() {
            document = document.without_resource(key).map_err(|_| ())?;
        } else if native_ids != *existing.native_ids() {
            let projected = ResourceState::new(
                existing.key().clone(),
                native_ids,
                existing.provenance(),
                existing.ownership(),
                existing.source().cloned(),
                existing.managed_artifact().cloned(),
                existing.fingerprint().cloned(),
                existing.installed_revision().cloned(),
                existing.available_revision().cloned(),
                existing.observed_at(),
                existing.last_apply().cloned(),
            )
            .map_err(|_| ())?;
            document = document.refresh_resource_state(projected).map_err(|_| ())?;
        } else {
            continue;
        }
        changed = true;
    }
    if changed {
        repository.replace(&document).map_err(|_| ())?;
    }
    Ok(())
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

fn git_revision_changed(
    old: Option<&skilltap_core::domain::ResolvedRevision>,
    new: Option<&GitCommit>,
) -> bool {
    match (old, new) {
        (Some(skilltap_core::domain::ResolvedRevision::GitCommit(old)), Some(new)) => old != new,
        (None, Some(_)) => true,
        _ => false,
    }
}

fn harnesses_label(harnesses: &HarnessSet) -> String {
    harnesses
        .iter()
        .map(HarnessId::as_str)
        .collect::<Vec<_>>()
        .join(",")
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
    let hash = stable_hash(&format!("{harness}:{root}"));
    format!("native-{hash:016x}")
}

fn stable_hash(input: &str) -> u64 {
    input.bytes().fold(0xcbf29ce484222325_u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
    })
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
