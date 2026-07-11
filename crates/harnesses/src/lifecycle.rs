//! Native lifecycle command vectors for verified Codex and Claude contracts.

use std::{collections::BTreeMap, ffi::OsString, fmt};

use skilltap_core::{
    domain::{
        ConfiguredBinary, EvidenceCode, EvidenceDetail, HarnessId, NativeId, Operation,
        OperationId, OperationOutcome, Plan, Scope, SourceLocator,
    },
    executor::{ExecutionError, ExecutionPort},
    runtime::{
        ExecutableResolutionRequest, ExecutableResolver, NativeProcessOutput, NativeProcessRequest,
        NativeProcessRunner, ObservationRuntimeError, ProcessLimits, SystemExecutableResolver,
        SystemNativeProcessRunner,
    },
};

use crate::HarnessKind;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeLifecycleAction {
    MarketplaceAdd,
    MarketplaceRemove,
    MarketplaceUpdate,
    PluginInstall,
    PluginRemove,
    PluginUpdate,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeLifecycleRequest {
    pub harness: HarnessKind,
    pub action: NativeLifecycleAction,
    pub scope: Scope,
    pub name: NativeId,
    pub source: Option<SourceLocator>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NativeLifecycleError {
    MissingSource,
    UnsupportedProjectScope,
    Runtime(ObservationRuntimeError),
}

impl fmt::Display for NativeLifecycleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::MissingSource => "marketplace add requires an explicit source",
            Self::UnsupportedProjectScope => {
                "the native harness has no verified project-scoped lifecycle command"
            }
            Self::Runtime(error) => return error.fmt(formatter),
        })
    }
}

impl std::error::Error for NativeLifecycleError {}

impl From<ObservationRuntimeError> for NativeLifecycleError {
    fn from(error: ObservationRuntimeError) -> Self {
        Self::Runtime(error)
    }
}

/// Build a direct native argument vector. The caller still owns executable
/// resolution, profile authority, bounded execution, and post-mutation
/// observation; this function never shells out.
pub fn native_arguments(
    request: &NativeLifecycleRequest,
) -> Result<Vec<OsString>, NativeLifecycleError> {
    let project = matches!(request.scope, Scope::Project(_));
    match request.harness {
        HarnessKind::Codex if project => Err(NativeLifecycleError::UnsupportedProjectScope),
        HarnessKind::Claude => {
            let scope = if project { "local" } else { "user" };
            Ok(claude_arguments(request, scope))
        }
        HarnessKind::Codex => Ok(codex_arguments(request)),
    }
}

/// Execute one already-authorized lifecycle vector through the bounded native
/// process boundary. Profile selection and post-mutation observation remain
/// caller responsibilities.
pub fn run_native_lifecycle(
    configured: ConfiguredBinary,
    search_path: Option<OsString>,
    request: &NativeLifecycleRequest,
    limits: ProcessLimits,
) -> Result<NativeProcessOutput, NativeLifecycleError> {
    let executable = SystemExecutableResolver
        .resolve(&ExecutableResolutionRequest::new(configured, search_path))?;
    let working_directory = match &request.scope {
        Scope::Global => None,
        Scope::Project(path) => Some(path.clone()),
    };
    Ok(SystemNativeProcessRunner.run(&NativeProcessRequest::new(
        executable,
        native_arguments(request)?,
        std::collections::BTreeMap::new(),
        working_directory,
        limits,
    ))?)
}

/// Execution adapter for a validated set of native lifecycle requests.
///
/// The CLI/application layer chooses the requests only after profile and
/// scope validation. This adapter then enforces the smaller boundary: every
/// executable operation in the plan must have an exact request, every request
/// must target the operation's harness/scope, and native execution is bounded
/// and direct-argument only.
pub struct NativeLifecyclePort {
    entries: BTreeMap<OperationId, NativeLifecycleEntry>,
}

struct NativeLifecycleEntry {
    configured: ConfiguredBinary,
    search_path: Option<OsString>,
    limits: ProcessLimits,
    request: NativeLifecycleRequest,
}

impl NativeLifecyclePort {
    pub fn new(
        configured: ConfiguredBinary,
        search_path: Option<OsString>,
        limits: ProcessLimits,
        requests: impl IntoIterator<Item = (OperationId, NativeLifecycleRequest)>,
    ) -> Self {
        Self::new_per_operation(
            requests.into_iter().map(|(id, request)| {
                (id, configured.clone(), search_path.clone(), limits, request)
            }),
        )
    }

    pub fn new_per_operation(
        entries: impl IntoIterator<
            Item = (
                OperationId,
                ConfiguredBinary,
                Option<OsString>,
                ProcessLimits,
                NativeLifecycleRequest,
            ),
        >,
    ) -> Self {
        Self {
            entries: entries
                .into_iter()
                .map(|(id, configured, search_path, limits, request)| {
                    (
                        id,
                        NativeLifecycleEntry {
                            configured,
                            search_path,
                            limits,
                            request,
                        },
                    )
                })
                .collect(),
        }
    }
}

impl ExecutionPort for NativeLifecyclePort {
    fn revalidate(&self, plan: &Plan) -> Result<(), ExecutionError> {
        for (_, operation) in plan.iter() {
            if !matches!(
                operation.action(),
                skilltap_core::domain::OperationAction::MarketplaceRegister
                    | skilltap_core::domain::OperationAction::MarketplaceRemove
                    | skilltap_core::domain::OperationAction::MarketplaceUpdate
                    | skilltap_core::domain::OperationAction::PluginInstall
                    | skilltap_core::domain::OperationAction::PluginRemove
                    | skilltap_core::domain::OperationAction::PluginUpdate
            ) {
                continue;
            }
            let Some(entry) = self.entries.get(operation.id()) else {
                return Err(ExecutionError::revalidation(
                    EvidenceCode::new("native.request_missing")
                        .expect("static evidence code is valid"),
                    EvidenceDetail::new(
                        "The native lifecycle adapter did not receive a request for a planned operation.",
                    )
                    .expect("static evidence detail is valid"),
                ));
            };
            if entry.request.scope != *operation.scope()
                || !action_matches(operation.action(), entry.request.action)
                || HarnessId::new(entry.request.harness.id())
                    .expect("harness kind identifier is valid")
                    != *operation.target()
            {
                return Err(ExecutionError::revalidation(
                    EvidenceCode::new("native.request_mismatch")
                        .expect("static evidence code is valid"),
                    EvidenceDetail::new(
                        "The native lifecycle request no longer matches the validated operation.",
                    )
                    .expect("static evidence detail is valid"),
                ));
            }
        }
        Ok(())
    }

    fn apply(&self, operation: &Operation) -> Result<OperationOutcome, ExecutionError> {
        let Some(entry) = self.entries.get(operation.id()) else {
            return Err(ExecutionError::revalidation(
                EvidenceCode::new("native.request_missing")
                    .expect("static evidence code is valid"),
                EvidenceDetail::new(
                    "The native lifecycle adapter did not receive a request for a planned operation.",
                )
                .expect("static evidence detail is valid"),
            ));
        };
        let output = run_native_lifecycle(
            entry.configured.clone(),
            entry.search_path.clone(),
            &entry.request,
            entry.limits,
        )
        .map_err(|_| native_apply_failure("The native lifecycle command could not be run."))?;
        if output.status().success() {
            Ok(OperationOutcome::Applied)
        } else {
            Err(native_apply_failure(
                "The native lifecycle command returned a nonzero status.",
            ))
        }
    }
}

fn native_apply_failure(detail: &'static str) -> ExecutionError {
    ExecutionError::apply_failure(skilltap_core::domain::AttentionReason::operation_failed(
        EvidenceCode::new("native.command_failed").expect("static evidence code is valid"),
        EvidenceDetail::new(detail).expect("static evidence detail is valid"),
    ))
}

fn action_matches(
    action: skilltap_core::domain::OperationAction,
    native: NativeLifecycleAction,
) -> bool {
    matches!(
        (action, native),
        (
            skilltap_core::domain::OperationAction::MarketplaceRegister,
            NativeLifecycleAction::MarketplaceAdd
        ) | (
            skilltap_core::domain::OperationAction::MarketplaceRemove,
            NativeLifecycleAction::MarketplaceRemove
        ) | (
            skilltap_core::domain::OperationAction::MarketplaceUpdate,
            NativeLifecycleAction::MarketplaceUpdate
        ) | (
            skilltap_core::domain::OperationAction::PluginInstall,
            NativeLifecycleAction::PluginInstall
        ) | (
            skilltap_core::domain::OperationAction::PluginRemove,
            NativeLifecycleAction::PluginRemove
        ) | (
            skilltap_core::domain::OperationAction::PluginUpdate,
            NativeLifecycleAction::PluginUpdate
        )
    )
}

fn codex_arguments(request: &NativeLifecycleRequest) -> Vec<OsString> {
    let mut args = vec![OsString::from("plugin")];
    match request.action {
        NativeLifecycleAction::MarketplaceAdd => {
            args.extend(["marketplace", "add"].into_iter().map(OsString::from));
            args.push(OsString::from(
                request.source.as_ref().expect("validated source").as_str(),
            ));
        }
        NativeLifecycleAction::MarketplaceRemove => args.extend(
            ["marketplace", "remove", request.name.as_str()]
                .into_iter()
                .map(OsString::from),
        ),
        NativeLifecycleAction::MarketplaceUpdate => args.extend(
            ["marketplace", "upgrade", request.name.as_str()]
                .into_iter()
                .map(OsString::from),
        ),
        NativeLifecycleAction::PluginInstall => args.extend(
            ["add", request.name.as_str()]
                .into_iter()
                .map(OsString::from),
        ),
        NativeLifecycleAction::PluginRemove => args.extend(
            ["remove", request.name.as_str()]
                .into_iter()
                .map(OsString::from),
        ),
        NativeLifecycleAction::PluginUpdate => args.extend(
            ["update", request.name.as_str()]
                .into_iter()
                .map(OsString::from),
        ),
    }
    args
}

fn claude_arguments(request: &NativeLifecycleRequest, scope: &str) -> Vec<OsString> {
    let mut args = vec![OsString::from("plugin")];
    match request.action {
        NativeLifecycleAction::MarketplaceAdd => {
            args.extend(["marketplace", "add"].into_iter().map(OsString::from));
            args.push(OsString::from(
                request.source.as_ref().expect("validated source").as_str(),
            ));
        }
        NativeLifecycleAction::MarketplaceRemove => args.extend(
            ["marketplace", "remove", request.name.as_str()]
                .into_iter()
                .map(OsString::from),
        ),
        NativeLifecycleAction::MarketplaceUpdate => args.extend(
            ["marketplace", "update", request.name.as_str()]
                .into_iter()
                .map(OsString::from),
        ),
        NativeLifecycleAction::PluginInstall => args.extend(
            ["install", request.name.as_str()]
                .into_iter()
                .map(OsString::from),
        ),
        NativeLifecycleAction::PluginRemove => args.extend(
            ["uninstall", request.name.as_str()]
                .into_iter()
                .map(OsString::from),
        ),
        NativeLifecycleAction::PluginUpdate => args.extend(
            ["update", request.name.as_str()]
                .into_iter()
                .map(OsString::from),
        ),
    }
    args.extend(["--scope", scope].into_iter().map(OsString::from));
    args
}

#[cfg(test)]
mod tests {
    use super::*;
    use skilltap_core::domain::AbsolutePath;

    fn request(
        harness: HarnessKind,
        action: NativeLifecycleAction,
        scope: Scope,
    ) -> NativeLifecycleRequest {
        NativeLifecycleRequest {
            harness,
            action,
            scope,
            name: NativeId::new("formatter@team").unwrap(),
            source: Some(SourceLocator::new("https://example.invalid/team.git").unwrap()),
        }
    }

    #[test]
    fn native_vectors_use_direct_arguments_and_scope_mapping() {
        let claude = native_arguments(&request(
            HarnessKind::Claude,
            NativeLifecycleAction::PluginInstall,
            Scope::Global,
        ))
        .unwrap();
        assert_eq!(
            claude,
            ["plugin", "install", "formatter@team", "--scope", "user"].map(OsString::from)
        );
        let codex = native_arguments(&request(
            HarnessKind::Codex,
            NativeLifecycleAction::MarketplaceAdd,
            Scope::Global,
        ))
        .unwrap();
        assert_eq!(
            codex,
            [
                "plugin",
                "marketplace",
                "add",
                "https://example.invalid/team.git"
            ]
            .map(OsString::from)
        );
        let claude_update = native_arguments(&request(
            HarnessKind::Claude,
            NativeLifecycleAction::PluginUpdate,
            Scope::Global,
        ))
        .unwrap();
        assert_eq!(
            claude_update,
            ["plugin", "update", "formatter@team", "--scope", "user"].map(OsString::from)
        );
        let codex_remove = native_arguments(&request(
            HarnessKind::Codex,
            NativeLifecycleAction::MarketplaceRemove,
            Scope::Global,
        ))
        .unwrap();
        assert_eq!(
            codex_remove,
            ["plugin", "marketplace", "remove", "formatter@team"].map(OsString::from)
        );
        assert!(matches!(
            native_arguments(&request(
                HarnessKind::Codex,
                NativeLifecycleAction::PluginInstall,
                Scope::Project(AbsolutePath::new("/tmp/project").unwrap())
            )),
            Err(NativeLifecycleError::UnsupportedProjectScope)
        ));
    }
}
