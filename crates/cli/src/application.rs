use std::{
    collections::BTreeSet,
    path::{Component, Path, PathBuf},
};

use skilltap_core::{
    adoption::{
        AdoptionApplyError, AdoptionDecision, AdoptionObservationError, AdoptionSelection,
        apply_adoption, plan_adoption,
    },
    domain::{
        AbsolutePath, CapabilitySupport, ComponentGraph, ConfiguredBinary, HarnessId,
        HarnessObservation, HarnessObservationOutcome, HarnessReachability, HarnessSet, NativeId,
        ObservationAdapterError, ObservationBatch, ObservationEvidence, ObservationFields,
        ObservationFinding, ObservationFindingCode, ObservationKey, ObservationLayer,
        ObservationRequest, ObservationSeverity, ObservationSubject, ObservationSummary,
        ObservationTarget, ObservedResource, Ownership, ProfileAuthority, Provenance,
        ResourceHealth, ResourceId, ResourceKey, ResourceKind, Scope,
    },
    reconciliation::{ReconciliationRequest, plan_reconciliation},
    runtime::{
        ExternalTreeLimits, JsonLimits, PlatformPaths, ProcessEnvironment, ProcessLimits,
        ScopeRequest, ScopeResolver, SystemConfigurationLock, WorkingDirectory, resolve_targets,
    },
    storage::{
        ConfigDocument, ConfigRepository, DocumentState, InventoryDocument, InventoryRepository,
        StateDocument, StateRepository, StorageError, StorageFailure,
    },
};
use skilltap_harnesses::{
    CanonicalObservation, HarnessKind, detect_configured_installation, normalize_observations,
    observe_claude_canonical_resources, observe_codex_canonical_resources, select_profile,
};

use crate::{
    ErrorDetail, NextAction, Outcome, OutputEntry, OutputScope, ResultClass, Warning,
    command::{
        AdoptArgs, OutputArgs, PlanArgs, ScopeArgs, ScopeArgument, ScopedTargetArgs, StatusArgs,
        SyncArgs, TargetArgs,
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
        let documents = DocumentLoadPhase::execute(self);
        let mut outcome = documents.project(Outcome::new(command, ResultClass::AttentionRequired));
        let documents = match documents.finish() {
            Ok(documents) => documents,
            Err(errors) => {
                outcome.result = ResultClass::Invalid;
                for error in errors {
                    outcome = outcome.with_error(error);
                }
                return outcome;
            }
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

    fn execute_reconciliation(
        &self,
        command: &'static str,
        target: &TargetArgs,
        requested_scope: &ScopeArgs,
        includes: &[NativeId],
        excludes: &[NativeId],
        acknowledged: bool,
    ) -> Outcome {
        let documents = DocumentLoadPhase::execute(self);
        let mut outcome = documents.project(Outcome::new(command, ResultClass::AttentionRequired));
        let documents = match documents.finish() {
            Ok(documents) => documents,
            Err(errors) => {
                outcome.result = ResultClass::Invalid;
                for error in errors {
                    outcome = outcome.with_error(error);
                }
                return outcome.with_next_action(NextAction::new(
                    "repair_owned_documents",
                    "Repair the reported skilltap-owned documents before retrying.",
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
        let documents = DocumentLoadPhase::execute(self);
        let mut outcome = documents.project(Outcome::new("status", ResultClass::AttentionRequired));
        let documents = match documents.finish() {
            Ok(documents) => documents,
            Err(errors) => {
                outcome.result = ResultClass::Invalid;
                for error in errors {
                    outcome = outcome.with_error(error);
                }
                return outcome.with_next_action(NextAction::new(
                    "repair_owned_documents",
                    "Repair the reported skilltap-owned documents before retrying.",
                ));
            }
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
        let documents = DocumentLoadPhase::execute(self);
        let mut outcome = documents.project(Outcome::new("adopt", ResultClass::AttentionRequired));
        let documents = match documents.finish() {
            Ok(documents) => documents,
            Err(errors) => {
                outcome.result = ResultClass::Invalid;
                for error in errors {
                    outcome = outcome.with_error(error);
                }
                return outcome.with_next_action(NextAction::new(
                    "repair_owned_documents",
                    "Repair the reported skilltap-owned documents before retrying adoption.",
                ));
            }
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
        for resource in observation.resources {
            outcome = outcome.with_resource(resource);
        }
        for warning in observation.warnings {
            outcome = outcome.with_warning(warning);
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
            .with_summary("native_entries", observation.native_entries as u64);
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
