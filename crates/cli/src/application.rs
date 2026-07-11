use std::path::{Component, Path, PathBuf};

use skilltap_core::{
    domain::{AbsolutePath, HarnessId, HarnessSet, Scope},
    runtime::{ScopeRequest, ScopeResolver, WorkingDirectory, resolve_targets},
    storage::{
        ConfigDocument, ConfigRepository, DocumentState, InventoryDocument, InventoryRepository,
        StateDocument, StateRepository, StorageError, StorageFailure,
    },
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
        outcome
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
            .with_warning(Warning::new(
                "native_observation_unavailable",
                "Native harness observation is not available in this build.",
            ))
            .with_next_action(NextAction::new(
                "retry_after_native_observation",
                "Retry status when native harness observation is available.",
            ))
    }
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
