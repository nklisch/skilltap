use std::{
    collections::BTreeSet,
    path::{Component, Path, PathBuf},
};

use skilltap_core::{
    domain::{
        AbsolutePath, ComponentGraph, ConfiguredBinary, HarnessId, HarnessObservation,
        HarnessObservationOutcome, HarnessReachability, HarnessSet, NativeId,
        ObservationAdapterError, ObservationBatch, ObservationEvidence, ObservationFields,
        ObservationFinding, ObservationFindingCode, ObservationKey, ObservationLayer,
        ObservationRequest, ObservationSeverity, ObservationSubject, ObservationSummary,
        ObservedResource, Ownership, ProfileAuthority, Provenance, ResourceHealth, ResourceId,
        ResourceKey, ResourceKind, Scope,
    },
    runtime::{
        ExternalTreeLimits, JsonLimits, PlatformPaths, ProcessEnvironment, ProcessLimits,
        ScopeRequest, ScopeResolver, WorkingDirectory, resolve_targets,
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
    command::{ScopeArgument, StatusArgs},
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
) -> Outcome {
    let process_limits = ProcessLimits::new(5_000, 256 * 1024, 256 * 1024, 512 * 1024)
        .expect("bounded status process limits are valid");
    let json_limits =
        JsonLimits::new(256 * 1024, 64).expect("bounded status JSON limits are valid");
    let search_path = std::env::var_os("PATH");
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
    ] {
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
        if observation.failed_targets == 0 && desired_resources > observation.native_entries {
            outcome.result = ResultClass::AttentionRequired;
            outcome = outcome.with_warning(Warning::new(
                "status_resource_incomplete",
                "Desired resources exceed the resources observed in native roots.",
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
        for (_, outcome) in environment.iter() {
            match outcome {
                HarnessObservationOutcome::Observed { observation } => {
                    result.observed_targets += 1;
                    result.resources.push(
                        OutputEntry::new(
                            observation_id(
                                observation.target().harness(),
                                observation.target().scope(),
                            ),
                            "observed",
                        )
                        .with_field("harness", observation.target().harness().as_str())
                        .with_field("scope", scope_label(observation.target().scope()))
                        .with_field("typed", true),
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
            let _ = skilltap_harnesses::codex_observation_paths(paths, scope);
            match scope {
                Scope::Global => vec!["codex.global.instructions"],
                Scope::Project(_) => vec!["project.agents.instructions", "project.agents.override"],
            }
        }
        HarnessKind::Claude => {
            let _ = skilltap_harnesses::claude_observation_paths(paths, scope);
            match scope {
                Scope::Global => vec!["claude.settings"],
                Scope::Project(_) => vec!["project.claude.settings"],
            }
        }
    }
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
