use std::path::{Component, Path, PathBuf};

use skilltap_core::{
    domain::{
        AbsolutePath, CapabilitySupport, ConfiguredBinary, HarnessId, HarnessReachability,
        HarnessSet, ProfileAuthority, Scope,
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
    HarnessKind, detect_configured_installation, observe_claude_resources, observe_codex_resources,
    select_profile,
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
                return outcome
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
}

impl StatusProjection<'_> {
    fn apply(self, mut outcome: Outcome) -> Outcome {
        for target in self.targets.iter() {
            outcome = outcome.with_resource(OutputEntry::new(target.as_str(), "selected"));
        }
        let observation = NativeObservation::run(self.documents, self.scope, self.targets);
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
        if observation.failed_targets > 0 {
            outcome = outcome.with_next_action(NextAction::new(
                "inspect_native_observation",
                "Inspect the reported native observation warnings before planning changes.",
            ));
        }
        outcome
    }
}

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
        let mut result = Self {
            resources: Vec::new(),
            warnings: Vec::new(),
            observed_targets: 0,
            failed_targets: 0,
            native_entries: 0,
        };

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
                let id = observation_id(target, current_scope);
                let mut entry = OutputEntry::new(id, "observed")
                    .with_field("harness", target.as_str())
                    .with_field("scope", scope_label(current_scope))
                    .with_field("version", native_version.as_str())
                    .with_field("profile_authority", profile_authority(profile.authority()))
                    .with_field(
                        "capabilities_supported",
                        capability_count(&profile, current_scope, CapabilitySupport::Supported)
                            as u64,
                    )
                    .with_field(
                        "capabilities_unverified",
                        capability_count(&profile, current_scope, CapabilitySupport::Unverified)
                            as u64,
                    )
                    .with_field(
                        "capabilities_unsupported",
                        capability_count(&profile, current_scope, CapabilitySupport::Unsupported)
                            as u64,
                    );
                if let Some(profile_id) = profile.profile_id() {
                    entry = entry.with_field("profile", profile_id.as_str());
                }
                match observe_tree(kind, &paths, current_scope, tree_limits) {
                    Ok(snapshot) => {
                        result.observed_targets += 1;
                        result.native_entries += snapshot.entries().len();
                        entry = entry.with_field("native_entries", snapshot.entries().len() as u64);
                    }
                    Err(error) => {
                        result.failed_targets += 1;
                        entry.status = "observation_failed".to_owned();
                        result.warnings.push(
                            Warning::new(
                                "native_observation_failed",
                                "Native harness state could not be observed within the safety limits.",
                            )
                            .with_context("harness", target.as_str())
                            .with_context("scope", scope_label(current_scope))
                            .with_context("detail", error.to_string()),
                        );
                    }
                }
                result.resources.push(entry);
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

fn observe_tree(
    kind: HarnessKind,
    paths: &PlatformPaths,
    scope: &Scope,
    limits: ExternalTreeLimits,
) -> Result<
    skilltap_core::runtime::ExternalTreeSnapshot,
    skilltap_core::runtime::ObservationRuntimeError,
> {
    match kind {
        HarnessKind::Codex => {
            let inputs =
                skilltap_harnesses::codex_observation_paths(paths, scope).map_err(|_| {
                    skilltap_core::runtime::ObservationRuntimeError::TreeRootUnavailable
                })?;
            if matches!(scope, Scope::Global) {
                observe_codex_resources(&inputs, limits)
            } else {
                // Project instruction files are single documented inputs. Do not
                // recursively walk an arbitrary project root during status.
                Err(skilltap_core::runtime::ObservationRuntimeError::TreeRootUnavailable)
            }
        }
        HarnessKind::Claude => {
            let inputs =
                skilltap_harnesses::claude_observation_paths(paths, scope).map_err(|_| {
                    skilltap_core::runtime::ObservationRuntimeError::TreeRootUnavailable
                })?;
            if matches!(scope, Scope::Global) {
                observe_claude_resources(&inputs, limits)
            } else {
                // Claude project settings are a documented file input; avoid a
                // broad recursive walk of user project content.
                Err(skilltap_core::runtime::ObservationRuntimeError::TreeRootUnavailable)
            }
        }
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

fn profile_authority(authority: ProfileAuthority) -> &'static str {
    match authority {
        ProfileAuthority::VerifiedCompiled => "verified_compiled",
        ProfileAuthority::ObserveOnly => "observe_only",
    }
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
