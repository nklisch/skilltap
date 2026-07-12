//! Native lifecycle command vectors for verified Codex and Claude contracts.

use std::{collections::BTreeMap, ffi::OsString, fmt};

use skilltap_core::{
    domain::{
        ConfiguredBinary, EvidenceCode, EvidenceDetail, ExecutableIdentity, HarnessId, NativeId,
        Operation, OperationId, OperationOutcome, Plan, Scope, SourceLocator,
    },
    executor::{ExecutionError, ExecutionPort},
    runtime::{
        ExecutableResolutionRequest, ExecutableResolver, NativeProcessOutput, NativeProcessRequest,
        NativeProcessRunner, ObservationRuntimeError, ProcessLimits, StrictJson, StrictJsonDecoder,
        SystemExecutableResolver, SystemNativeProcessRunner,
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

/// Fresh native evidence for a managed lifecycle resource.  Unknown is
/// intentionally conservative: an unreadable or shape-changing native list
/// must not cause a previously successful operation to run again.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeResourcePresence {
    Present,
    Missing,
    Unknown,
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
    OptionLikeArgument(&'static str),
    UnsupportedProjectScope,
    UnsupportedAction,
    Runtime(ObservationRuntimeError),
}

impl fmt::Display for NativeLifecycleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::MissingSource => "marketplace add requires an explicit source",
            Self::OptionLikeArgument(field) => {
                return write!(
                    formatter,
                    "native lifecycle {field} must not begin with `-`"
                );
            }
            Self::UnsupportedProjectScope => {
                "the native harness has no verified project-scoped lifecycle command"
            }
            Self::UnsupportedAction => "the native harness has no verified lifecycle command",
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
    if request.name.as_str().starts_with('-') {
        return Err(NativeLifecycleError::OptionLikeArgument("name"));
    }
    if request
        .source
        .as_ref()
        .is_some_and(|source| source.as_str().starts_with('-'))
    {
        return Err(NativeLifecycleError::OptionLikeArgument("source"));
    }
    let project = matches!(request.scope, Scope::Project(_));
    if request.harness == HarnessKind::Codex
        && request.action == NativeLifecycleAction::PluginUpdate
    {
        return Err(NativeLifecycleError::UnsupportedAction);
    }
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
    environment: &BTreeMap<OsString, OsString>,
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
        environment.clone(),
        working_directory,
        limits,
    ))?)
}

/// Execute a lifecycle vector against the exact executable identity that was
/// observed during detection.  Revalidation immediately before spawn closes
/// the PATH replacement window between read-first detection and mutation.
pub fn run_native_lifecycle_bound(
    executable: &ExecutableIdentity,
    environment: &BTreeMap<OsString, OsString>,
    request: &NativeLifecycleRequest,
    limits: ProcessLimits,
) -> Result<NativeProcessOutput, NativeLifecycleError> {
    SystemExecutableResolver.revalidate(executable)?;
    let working_directory = match &request.scope {
        Scope::Global => None,
        Scope::Project(path) => Some(path.clone()),
    };
    Ok(SystemNativeProcessRunner.run(&NativeProcessRequest::new(
        executable.clone(),
        native_arguments(request)?,
        environment.clone(),
        working_directory,
        limits,
    ))?)
}

/// Observe one exact native resource through the harness's documented JSON
/// list command.  This is deliberately separate from lifecycle execution so
/// callers can invalidate stale journal entries without treating caches as a
/// write API.  Unknown output remains non-authoritative and preserves the
/// existing idempotent journal behavior.
pub fn observe_native_resource(
    configured: ConfiguredBinary,
    search_path: Option<OsString>,
    environment: &BTreeMap<OsString, OsString>,
    request: &NativeLifecycleRequest,
    process_limits: ProcessLimits,
    json_limits: skilltap_core::runtime::JsonLimits,
) -> Result<NativeResourcePresence, NativeLifecycleError> {
    let executable = SystemExecutableResolver
        .resolve(&ExecutableResolutionRequest::new(configured, search_path))?;
    let output = SystemNativeProcessRunner.run(&NativeProcessRequest::new(
        executable,
        native_list_arguments(request),
        environment.clone(),
        match &request.scope {
            Scope::Global => None,
            Scope::Project(path) => Some(path.clone()),
        },
        process_limits,
    ))?;
    if !output.status().success() {
        return Ok(NativeResourcePresence::Unknown);
    }
    let decoded = match StrictJson.decode(output.stdout(), json_limits) {
        Ok(decoded) => decoded,
        Err(_) => return Ok(NativeResourcePresence::Unknown),
    };
    Ok(resource_presence(decoded.value(), request.name.as_str()))
}

fn native_list_arguments(request: &NativeLifecycleRequest) -> Vec<OsString> {
    let mut args = vec![OsString::from("plugin")];
    match request.action {
        NativeLifecycleAction::MarketplaceAdd
        | NativeLifecycleAction::MarketplaceRemove
        | NativeLifecycleAction::MarketplaceUpdate => {
            args.extend(
                ["marketplace", "list", "--json"]
                    .into_iter()
                    .map(OsString::from),
            );
        }
        NativeLifecycleAction::PluginInstall
        | NativeLifecycleAction::PluginRemove
        | NativeLifecycleAction::PluginUpdate => {
            args.extend(["list", "--json"].into_iter().map(OsString::from));
        }
    }
    args
}

fn resource_presence(value: &serde_json::Value, name: &str) -> NativeResourcePresence {
    const LIST_FIELDS: &[&str] = &["plugins", "marketplaces", "installed", "resources", "items"];
    const IDENTITY_FIELDS: &[&str] = &["name", "id", "plugin", "marketplace", "qualifiedName"];

    // Return Ok only when the value has a documented list shape. A list entry
    // must carry a string identity (or itself be a string); malformed entries
    // are not evidence that the resource is missing.
    fn parse_list(
        value: &serde_json::Value,
        name: &str,
        list_fields: &[&str],
        identity_fields: &[&str],
    ) -> Result<bool, ()> {
        match value {
            serde_json::Value::Array(values) => values.iter().try_fold(false, |found, value| {
                let matches = match value {
                    serde_json::Value::String(value) => value == name,
                    serde_json::Value::Object(fields) => {
                        let identities = fields
                            .iter()
                            .filter(|(field, _)| identity_fields.contains(&field.as_str()))
                            .map(|(_, value)| value.as_str().ok_or(()))
                            .collect::<Result<Vec<_>, _>>()?;
                        if identities.is_empty() {
                            return Err(());
                        }
                        identities.into_iter().any(|value| value == name)
                    }
                    _ => return Err(()),
                };
                Ok(found || matches)
            }),
            serde_json::Value::Object(fields) => {
                let list_values = fields
                    .iter()
                    .filter(|(field, _)| list_fields.contains(&field.as_str()))
                    .map(|(_, value)| value)
                    .collect::<Vec<_>>();
                if !list_values.is_empty() {
                    return list_values.into_iter().try_fold(false, |found, value| {
                        if !value.is_array() {
                            return Err(());
                        }
                        Ok(found || parse_list(value, name, list_fields, identity_fields)?)
                    });
                }
                let identities = fields
                    .iter()
                    .filter(|(field, _)| identity_fields.contains(&field.as_str()))
                    .map(|(_, value)| value.as_str().ok_or(()))
                    .collect::<Result<Vec<_>, _>>()?;
                if identities.is_empty() {
                    return Err(());
                }
                Ok(identities.into_iter().any(|value| value == name))
            }
            _ => Err(()),
        }
    }

    match parse_list(value, name, LIST_FIELDS, IDENTITY_FIELDS) {
        Ok(true) => NativeResourcePresence::Present,
        Ok(false) => NativeResourcePresence::Missing,
        Err(()) => NativeResourcePresence::Unknown,
    }
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
    environment: BTreeMap<OsString, OsString>,
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
            environment: BTreeMap::new(),
        }
    }

    pub fn new_per_operation_with_environment(
        entries: impl IntoIterator<
            Item = (
                OperationId,
                ConfiguredBinary,
                Option<OsString>,
                ProcessLimits,
                NativeLifecycleRequest,
            ),
        >,
        environment: BTreeMap<OsString, OsString>,
    ) -> Self {
        let mut port = Self::new_per_operation(entries);
        port.environment = environment;
        port
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
            &self.environment,
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
    if request.action != NativeLifecycleAction::MarketplaceUpdate {
        args.extend(["--scope", scope].into_iter().map(OsString::from));
    }
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
        let claude_marketplace_update = native_arguments(&request(
            HarnessKind::Claude,
            NativeLifecycleAction::MarketplaceUpdate,
            Scope::Project(AbsolutePath::new("/tmp/project").unwrap()),
        ))
        .unwrap();
        assert_eq!(
            claude_marketplace_update,
            ["plugin", "marketplace", "update", "formatter@team"].map(OsString::from)
        );
        assert_eq!(
            native_list_arguments(&request(
                HarnessKind::Claude,
                NativeLifecycleAction::MarketplaceUpdate,
                Scope::Project(AbsolutePath::new("/tmp/project").unwrap()),
            )),
            ["plugin", "marketplace", "list", "--json"].map(OsString::from)
        );
        assert_eq!(
            native_list_arguments(&request(
                HarnessKind::Claude,
                NativeLifecycleAction::PluginInstall,
                Scope::Global,
            )),
            ["plugin", "list", "--json"].map(OsString::from)
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
        assert_eq!(
            native_arguments(&request(
                HarnessKind::Codex,
                NativeLifecycleAction::PluginUpdate,
                Scope::Global,
            )),
            Err(NativeLifecycleError::UnsupportedAction)
        );
    }

    #[test]
    fn native_vectors_reject_option_like_untrusted_values() {
        let mut name = request(
            HarnessKind::Claude,
            NativeLifecycleAction::PluginInstall,
            Scope::Global,
        );
        name.name = NativeId::new("--help").unwrap();
        assert_eq!(
            native_arguments(&name),
            Err(NativeLifecycleError::OptionLikeArgument("name"))
        );

        let mut source = request(
            HarnessKind::Codex,
            NativeLifecycleAction::MarketplaceAdd,
            Scope::Global,
        );
        source.source = Some(SourceLocator::new("--upload-pack=evil").unwrap());
        assert_eq!(
            native_arguments(&source),
            Err(NativeLifecycleError::OptionLikeArgument("source"))
        );
    }

    #[test]
    fn native_resource_presence_is_conservative_and_identity_bound() {
        assert_eq!(
            resource_presence(
                &serde_json::json!({
                    "plugins": [{"name": "formatter@team"}]
                }),
                "formatter@team"
            ),
            NativeResourcePresence::Present
        );
        assert_eq!(
            resource_presence(&serde_json::json!({"plugins": []}), "formatter@team"),
            NativeResourcePresence::Missing
        );
        assert_eq!(
            resource_presence(&serde_json::json!({"version": "3.0.0"}), "formatter@team"),
            NativeResourcePresence::Unknown
        );
        for malformed in [
            serde_json::json!([1]),
            serde_json::json!([{}]),
            serde_json::json!({"plugins": "garbage"}),
            serde_json::json!({"plugins": [{}]}),
        ] {
            assert_eq!(
                resource_presence(&malformed, "formatter@team"),
                NativeResourcePresence::Unknown,
                "malformed list payload: {malformed}"
            );
        }
    }
}
