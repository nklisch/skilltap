use std::path::{Component, Path, PathBuf};

use skilltap_core::{
    domain::{AbsolutePath, HarnessId, Scope},
    runtime::{ScopeRequest, ScopeResolver, WorkingDirectory, resolve_targets},
    storage::{
        ConfigDocument, ConfigRepository, DocumentState, InventoryDocument, InventoryRepository,
        StateRepository, StorageError, StorageFailure,
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
        let config = self.config.load();
        let inventory = self.inventory.load();
        let state = self.state.load();

        let mut outcome = Outcome::new("status", ResultClass::AttentionRequired);
        outcome = document_result(outcome, "config", &config);
        outcome = document_result(outcome, "inventory", &inventory);
        outcome = document_result(outcome, "state", &state);

        let mut storage_errors = Vec::new();
        if let Err(error) = &config {
            storage_errors.push(storage_error(error));
        }
        if let Err(error) = &inventory {
            storage_errors.push(storage_error(error));
        }
        if let Err(error) = &state {
            storage_errors.push(storage_error(error));
        }
        if !storage_errors.is_empty() {
            outcome.result = ResultClass::Invalid;
            for error in storage_errors {
                outcome = outcome.with_error(error);
            }
            return outcome.with_next_action(NextAction::new(
                "repair_owned_documents",
                "Repair the reported skilltap-owned documents before retrying.",
            ));
        }

        let config = match config.expect("checked above") {
            DocumentState::Missing => ConfigDocument::defaults(),
            DocumentState::Present(config) => config,
        };
        let inventory = match inventory.expect("checked above") {
            DocumentState::Missing => None,
            DocumentState::Present(inventory) => Some(inventory),
        };
        let state = match state.expect("checked above") {
            DocumentState::Missing => None,
            DocumentState::Present(state) => Some(state),
        };

        let scope_request = match self.scope_request(args, inventory.as_ref()) {
            Ok(request) => request,
            Err(error) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(error);
            }
        };
        let scopes = match self.scopes.resolve(&scope_request) {
            Ok(scopes) => scopes,
            Err(_) => {
                outcome.result = ResultClass::Invalid;
                return outcome.with_error(ErrorDetail::new(
                    "project_scope_unavailable",
                    "The requested project scope could not be resolved.",
                ));
            }
        };
        let resolved_scopes = scopes.into_scopes();
        let scope_count = resolved_scopes.len() as u64;
        outcome.scope = Some(output_scope(&args.scope.argument(), &resolved_scopes));

        let enabled = enabled_harnesses(&config);
        if enabled.is_empty() {
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
        let targets = match resolve_targets(args.target.target.as_ref(), enabled) {
            Ok(targets) => targets,
            Err(_) => {
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

        let target_count = targets.iter().len() as u64;
        for target in targets.iter() {
            outcome = outcome.with_resource(OutputEntry::new(target.as_str(), "selected"));
        }
        outcome = outcome
            .with_summary(
                "desired_resources",
                inventory
                    .as_ref()
                    .map_or(0, |value| value.resources().len() as u64),
            )
            .with_summary(
                "recorded_resources",
                state
                    .as_ref()
                    .map_or(0, |value| value.resources().len() as u64),
            )
            .with_summary("scopes", scope_count)
            .with_summary("targets", target_count)
            .with_warning(Warning::new(
                "native_observation_unavailable",
                "Native harness observation is not available in this build.",
            ))
            .with_next_action(NextAction::new(
                "retry_after_native_observation",
                "Retry status when native harness observation is available.",
            ));
        outcome
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
