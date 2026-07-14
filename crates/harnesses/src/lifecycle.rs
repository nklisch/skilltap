//! Native lifecycle command vectors for verified Codex and Claude contracts.

use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    fmt,
};

use skilltap_core::{
    domain::{
        CapabilityScope, ConfiguredBinary, EvidenceCode, EvidenceDetail, ExecutableIdentity,
        HarnessId, NativeId, Operation, OperationId, OperationOutcome, Plan, ResolvedRevision,
        Scope, SourceLocator,
    },
    executor::{ExecutionError, ExecutionPort},
    runtime::{
        ExecutableResolutionRequest, ExecutableResolver, NativeProcessOutput, NativeProcessRequest,
        NativeProcessRunner, ObservationRuntimeError, ProcessLimits, StrictJson, StrictJsonDecoder,
        SystemExecutableResolver, SystemNativeProcessRunner,
    },
};

use crate::NativeLifecycleVector;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeLifecycleAction {
    MarketplaceAdd,
    MarketplaceRemove,
    MarketplaceUpdate,
    PluginInstall,
    PluginRemove,
    PluginUpdate,
}

/// Fresh native evidence for one exact managed lifecycle resource.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NativeResourceObservation {
    Present {
        scope: Option<CapabilityScope>,
        revision: Option<ResolvedRevision>,
    },
    Missing,
    Indeterminate(NativeObservationFailure),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeObservationFailure {
    CommandFailed,
    InvalidJson,
    UnsupportedShape,
    AmbiguousScope,
}

impl NativeObservationFailure {
    pub const fn diagnostic_code(self) -> &'static str {
        match self {
            Self::CommandFailed => "native_observation_command_failed",
            Self::InvalidJson => "native_observation_invalid_json",
            Self::UnsupportedShape => "native_observation_unsupported_shape",
            Self::AmbiguousScope => "native_observation_ambiguous_scope",
        }
    }

    pub const fn summary(self) -> &'static str {
        match self {
            Self::CommandFailed => "The native list command returned a nonzero status.",
            Self::InvalidJson => "The native list command returned invalid JSON.",
            Self::UnsupportedShape => "The native list command returned an unsupported JSON shape.",
            Self::AmbiguousScope => {
                "The native list command did not identify one unambiguous requested scope."
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LifecyclePostconditionError {
    ObservationFailed(NativeObservationFailure),
    ExpectedPresent,
    ExpectedMissing,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeLifecycleRequest {
    pub action: NativeLifecycleAction,
    pub scope: Scope,
    pub name: NativeId,
    pub source: Option<SourceLocator>,
}

/// Binds one semantic lifecycle request to the exact registered adapter that
/// owns its target-specific argv and scope-observation contract.
#[derive(Clone)]
pub struct NativeLifecycleDispatch {
    target: HarnessId,
    lifecycle: &'static dyn NativeLifecycleVector,
    request: NativeLifecycleRequest,
}

impl NativeLifecycleDispatch {
    pub fn new(
        target: HarnessId,
        lifecycle: &'static dyn NativeLifecycleVector,
        request: NativeLifecycleRequest,
    ) -> Self {
        Self {
            target,
            lifecycle,
            request,
        }
    }

    pub fn target(&self) -> &HarnessId {
        &self.target
    }

    pub fn request(&self) -> &NativeLifecycleRequest {
        &self.request
    }
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

/// Build a direct native argument vector through the owning adapter. The
/// caller still owns executable resolution, profile authority, bounded
/// execution, and post-mutation observation; this function never shells out.
pub fn native_arguments(
    dispatch: &NativeLifecycleDispatch,
) -> Result<Vec<OsString>, NativeLifecycleError> {
    dispatch.lifecycle.arguments(&dispatch.request)
}

pub(crate) fn validate_native_request(
    request: &NativeLifecycleRequest,
) -> Result<(), NativeLifecycleError> {
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
    Ok(())
}

/// Execute one already-authorized lifecycle vector through the bounded native
/// process boundary. Profile selection and post-mutation observation remain
/// caller responsibilities.
pub fn run_native_lifecycle(
    configured: ConfiguredBinary,
    search_path: Option<OsString>,
    environment: &BTreeMap<OsString, OsString>,
    dispatch: &NativeLifecycleDispatch,
    limits: ProcessLimits,
) -> Result<NativeProcessOutput, NativeLifecycleError> {
    let executable = SystemExecutableResolver
        .resolve(&ExecutableResolutionRequest::new(configured, search_path))?;
    let working_directory = match &dispatch.request.scope {
        Scope::Global => None,
        Scope::Project(path) => Some(path.clone()),
    };
    Ok(SystemNativeProcessRunner.run(&NativeProcessRequest::new(
        executable,
        native_arguments(dispatch)?,
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
    dispatch: &NativeLifecycleDispatch,
    limits: ProcessLimits,
) -> Result<NativeProcessOutput, NativeLifecycleError> {
    SystemExecutableResolver.revalidate(executable)?;
    let working_directory = match &dispatch.request.scope {
        Scope::Global => None,
        Scope::Project(path) => Some(path.clone()),
    };
    Ok(SystemNativeProcessRunner.run(&NativeProcessRequest::new(
        executable.clone(),
        native_arguments(dispatch)?,
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
    dispatch: &NativeLifecycleDispatch,
    process_limits: ProcessLimits,
    json_limits: skilltap_core::runtime::JsonLimits,
) -> Result<NativeResourceObservation, NativeLifecycleError> {
    let request = dispatch.request();
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
        return Ok(NativeResourceObservation::Indeterminate(
            NativeObservationFailure::CommandFailed,
        ));
    }
    let decoded = match StrictJson.decode(output.stdout(), json_limits) {
        Ok(decoded) => decoded,
        Err(_) => {
            return Ok(NativeResourceObservation::Indeterminate(
                NativeObservationFailure::InvalidJson,
            ));
        }
    };
    Ok(resource_observation(decoded.value(), dispatch))
}

pub fn verify_lifecycle_postcondition(
    action: NativeLifecycleAction,
    observation: NativeResourceObservation,
) -> Result<(), LifecyclePostconditionError> {
    match observation {
        NativeResourceObservation::Indeterminate(failure) => {
            Err(LifecyclePostconditionError::ObservationFailed(failure))
        }
        NativeResourceObservation::Present { .. }
            if matches!(
                action,
                NativeLifecycleAction::MarketplaceAdd
                    | NativeLifecycleAction::MarketplaceUpdate
                    | NativeLifecycleAction::PluginInstall
                    | NativeLifecycleAction::PluginUpdate
            ) =>
        {
            Ok(())
        }
        NativeResourceObservation::Missing
            if matches!(
                action,
                NativeLifecycleAction::MarketplaceRemove | NativeLifecycleAction::PluginRemove
            ) =>
        {
            Ok(())
        }
        NativeResourceObservation::Missing => Err(LifecyclePostconditionError::ExpectedPresent),
        NativeResourceObservation::Present { .. } => {
            Err(LifecyclePostconditionError::ExpectedMissing)
        }
    }
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

fn resource_observation(
    value: &serde_json::Value,
    dispatch: &NativeLifecycleDispatch,
) -> NativeResourceObservation {
    let request = dispatch.request();
    const LIST_FIELDS: &[&str] = &["plugins", "marketplaces", "installed", "resources", "items"];
    const IDENTITY_FIELDS: &[&str] = &["name", "id", "plugin", "marketplace", "qualifiedName"];

    #[derive(Clone)]
    struct Entry<'a> {
        identity: &'a str,
        scope: Option<&'a str>,
        revision: Option<ResolvedRevision>,
    }

    fn parse_entry<'a>(
        value: &'a serde_json::Value,
        identity_fields: &[&str],
    ) -> Result<Entry<'a>, ()> {
        match value {
            serde_json::Value::String(identity) => Ok(Entry {
                identity,
                scope: None,
                revision: None,
            }),
            serde_json::Value::Object(fields) => {
                let identities = fields
                    .iter()
                    .filter(|(field, _)| identity_fields.contains(&field.as_str()))
                    .map(|(_, value)| value.as_str().ok_or(()))
                    .collect::<Result<Vec<_>, _>>()?;
                let Some(identity) = identities.first().copied() else {
                    return Err(());
                };
                if identities.iter().any(|candidate| *candidate != identity) {
                    return Err(());
                }
                let scope = fields
                    .get("scope")
                    .map(|value| value.as_str().ok_or(()))
                    .transpose()?;
                let version = fields
                    .get("version")
                    .map(|value| value.as_str().ok_or(()))
                    .transpose()?;
                let revision = fields
                    .get("revision")
                    .map(|value| value.as_str().ok_or(()))
                    .transpose()?;
                if version.is_some() && revision.is_some() && version != revision {
                    return Err(());
                }
                let revision = match version.or(revision) {
                    Some(value) => Some(ResolvedRevision::Native(
                        NativeId::new(value).map_err(|_| ())?,
                    )),
                    None => None,
                };
                Ok(Entry {
                    identity,
                    scope,
                    revision,
                })
            }
            _ => Err(()),
        }
    }

    fn parse_list<'a>(
        value: &'a serde_json::Value,
        list_fields: &[&str],
        identity_fields: &[&str],
    ) -> Result<Vec<Entry<'a>>, ()> {
        match value {
            serde_json::Value::Array(values) => values
                .iter()
                .map(|value| parse_entry(value, identity_fields))
                .collect(),
            serde_json::Value::Object(fields) => {
                let list_values = fields
                    .iter()
                    .filter(|(field, _)| list_fields.contains(&field.as_str()))
                    .map(|(_, value)| value)
                    .collect::<Vec<_>>();
                if !list_values.is_empty() {
                    return list_values
                        .into_iter()
                        .try_fold(Vec::new(), |mut entries, value| {
                            if !value.is_array() {
                                return Err(());
                            }
                            entries.extend(parse_list(value, list_fields, identity_fields)?);
                            Ok(entries)
                        });
                }
                Ok(vec![parse_entry(value, identity_fields)?])
            }
            _ => Err(()),
        }
    }

    let entries = match parse_list(value, LIST_FIELDS, IDENTITY_FIELDS) {
        Ok(entries) => entries,
        Err(()) => {
            return NativeResourceObservation::Indeterminate(
                NativeObservationFailure::UnsupportedShape,
            );
        }
    };
    let Some(expected_scope) = dispatch.lifecycle.observation_scope(&request.scope) else {
        return if entries
            .iter()
            .any(|entry| entry.identity == request.name.as_str())
        {
            NativeResourceObservation::Present {
                scope: None,
                revision: entries
                    .iter()
                    .find(|entry| entry.identity == request.name.as_str())
                    .and_then(|entry| entry.revision.clone()),
            }
        } else {
            NativeResourceObservation::Missing
        };
    };

    let expected_scope_name = match expected_scope {
        CapabilityScope::Global => "user",
        CapabilityScope::Project => "local",
    };
    if entries
        .iter()
        .any(|entry| !matches!(entry.scope, Some("user" | "local")))
    {
        return NativeResourceObservation::Indeterminate(NativeObservationFailure::AmbiguousScope);
    }
    match entries
        .iter()
        .filter(|entry| {
            entry.identity == request.name.as_str() && entry.scope == Some(expected_scope_name)
        })
        .count()
    {
        0 => NativeResourceObservation::Missing,
        1 => NativeResourceObservation::Present {
            scope: Some(expected_scope),
            revision: entries
                .iter()
                .find(|entry| {
                    entry.identity == request.name.as_str()
                        && entry.scope == Some(expected_scope_name)
                })
                .and_then(|entry| entry.revision.clone()),
        },
        _ => NativeResourceObservation::Indeterminate(NativeObservationFailure::AmbiguousScope),
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
    foreign_operations: BTreeSet<OperationId>,
}

/// A native lifecycle request bound to its fresh pre-mutation observation.
/// Foreground callers may leave `before` empty; daemon update batches provide
/// it so conservative revision evidence can distinguish a verified no-change.
#[derive(Clone)]
pub struct NativeLifecycleBinding {
    pub operation_id: OperationId,
    pub configured: ConfiguredBinary,
    pub search_path: Option<OsString>,
    pub limits: ProcessLimits,
    pub dispatch: NativeLifecycleDispatch,
    pub before: Option<NativeResourceObservation>,
}

struct NativeLifecycleEntry {
    configured: ConfiguredBinary,
    search_path: Option<OsString>,
    limits: ProcessLimits,
    json_limits: skilltap_core::runtime::JsonLimits,
    dispatch: NativeLifecycleDispatch,
    before: Option<NativeResourceObservation>,
}

impl NativeLifecyclePort {
    pub fn new(
        configured: ConfiguredBinary,
        search_path: Option<OsString>,
        limits: ProcessLimits,
        requests: impl IntoIterator<Item = (OperationId, NativeLifecycleDispatch)>,
    ) -> Self {
        Self::new_per_operation(requests.into_iter().map(|(id, dispatch)| {
            (
                id,
                configured.clone(),
                search_path.clone(),
                limits,
                dispatch,
            )
        }))
    }

    pub fn new_per_operation(
        entries: impl IntoIterator<
            Item = (
                OperationId,
                ConfiguredBinary,
                Option<OsString>,
                ProcessLimits,
                NativeLifecycleDispatch,
            ),
        >,
    ) -> Self {
        Self {
            entries: entries
                .into_iter()
                .map(|(id, configured, search_path, limits, dispatch)| {
                    (
                        id,
                        NativeLifecycleEntry {
                            configured,
                            search_path,
                            limits,
                            json_limits: skilltap_core::runtime::JsonLimits::new(256 * 1024, 64)
                                .expect("static lifecycle JSON limits are valid"),
                            dispatch,
                            before: None,
                        },
                    )
                })
                .collect(),
            environment: BTreeMap::new(),
            foreign_operations: BTreeSet::new(),
        }
    }

    pub fn new_per_operation_with_environment(
        entries: impl IntoIterator<
            Item = (
                OperationId,
                ConfiguredBinary,
                Option<OsString>,
                ProcessLimits,
                NativeLifecycleDispatch,
            ),
        >,
        environment: BTreeMap<OsString, OsString>,
    ) -> Self {
        let mut port = Self::new_per_operation(entries);
        port.environment = environment;
        port
    }

    /// Build a port from bindings that carry the daemon's fresh pre-observation.
    pub fn new_bound_with_environment(
        bindings: impl IntoIterator<Item = NativeLifecycleBinding>,
        environment: BTreeMap<OsString, OsString>,
    ) -> Self {
        Self {
            entries: bindings
                .into_iter()
                .map(|binding| {
                    let id = binding.operation_id.clone();
                    (
                        id,
                        NativeLifecycleEntry {
                            configured: binding.configured,
                            search_path: binding.search_path,
                            limits: binding.limits,
                            json_limits: skilltap_core::runtime::JsonLimits::new(256 * 1024, 64)
                                .expect("static lifecycle JSON limits are valid"),
                            dispatch: binding.dispatch,
                            before: binding.before,
                        },
                    )
                })
                .collect(),
            environment,
            foreign_operations: BTreeSet::new(),
        }
    }

    /// Declare operation ids executed by a sibling adapter in one mixed plan.
    /// Native operations not present in either set still fail revalidation.
    pub fn with_foreign_operations(
        mut self,
        operations: impl IntoIterator<Item = OperationId>,
    ) -> Self {
        self.foreign_operations = operations.into_iter().collect();
        self
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
                if self.foreign_operations.contains(operation.id()) {
                    continue;
                }
                return Err(ExecutionError::revalidation(
                    EvidenceCode::new("native.request_missing")
                        .expect("static evidence code is valid"),
                    EvidenceDetail::new(
                        "The native lifecycle adapter did not receive a request for a planned operation.",
                    )
                    .expect("static evidence detail is valid"),
                ));
            };
            if entry.dispatch.request.scope != *operation.scope()
                || !action_matches(operation.action(), entry.dispatch.request.action)
                || entry.dispatch.target != *operation.target()
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
            if let Some(before) = &entry.before {
                let observation = observe_native_resource(
                    entry.configured.clone(),
                    entry.search_path.clone(),
                    &self.environment,
                    &entry.dispatch,
                    entry.limits,
                    entry.json_limits,
                )
                .map_err(|_| {
                    native_noop_revalidation_failure(
                        "native.precondition_observation_unavailable",
                        "Fresh native precondition evidence could not be re-observed under the configuration lock.",
                    )
                })?;
                if &observation != before {
                    return Err(native_noop_revalidation_failure(
                        "native.stale_evidence",
                        "Native lifecycle evidence changed after daemon planning.",
                    ));
                }
            }
            if operation.class() == skilltap_core::domain::OperationClass::NoOp {
                let observation = observe_native_resource(
                    entry.configured.clone(),
                    entry.search_path.clone(),
                    &self.environment,
                    &entry.dispatch,
                    entry.limits,
                    entry.json_limits,
                )
                .map_err(|_| {
                    native_noop_revalidation_failure(
                        "native.noop_observation_unavailable",
                        "Fresh native no-change evidence could not be re-observed under the configuration lock.",
                    )
                })?;
                verify_lifecycle_postcondition(entry.dispatch.request.action, observation).map_err(
                    |error| match error {
                        LifecyclePostconditionError::ObservationFailed(failure) => {
                            native_noop_revalidation_failure(
                                failure.diagnostic_code(),
                                failure.summary(),
                            )
                        }
                        LifecyclePostconditionError::ExpectedPresent => {
                            native_noop_revalidation_failure(
                                "native.noop_expected_present",
                                "The resource was no longer present when no-change evidence was revalidated.",
                            )
                        }
                        LifecyclePostconditionError::ExpectedMissing => {
                            native_noop_revalidation_failure(
                                "native.noop_expected_missing",
                                "The resource was present when removal no-change evidence was revalidated.",
                            )
                        }
                    },
                )?;
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
            &entry.dispatch,
            entry.limits,
        )
        .map_err(|_| native_apply_failure("The native lifecycle command could not be run."))?;
        if output.status().success() {
            let observation = observe_native_resource(
                entry.configured.clone(),
                entry.search_path.clone(),
                &self.environment,
                &entry.dispatch,
                entry.limits,
                entry.json_limits,
            )
            .map_err(|_| native_observation_failure(NativeObservationFailure::CommandFailed))?;
            verify_lifecycle_postcondition(entry.dispatch.request.action, observation.clone())
                .map_err(lifecycle_postcondition_failure)?;
            let no_change = matches!(
                (&entry.before, &observation),
                (
                    Some(NativeResourceObservation::Present {
                        revision: Some(before),
                        ..
                    }),
                    NativeResourceObservation::Present {
                        revision: Some(after),
                        ..
                    }
                ) if before == after
            );
            Ok(if no_change {
                OperationOutcome::NoChange
            } else {
                OperationOutcome::Applied
            })
        } else {
            Err(native_apply_failure(
                "The native lifecycle command returned a nonzero status.",
            ))
        }
    }
}

fn native_noop_revalidation_failure(code: &'static str, detail: &'static str) -> ExecutionError {
    ExecutionError::revalidation(
        EvidenceCode::new(code).expect("static evidence code is valid"),
        EvidenceDetail::new(detail).expect("static evidence detail is valid"),
    )
}

fn lifecycle_postcondition_failure(error: LifecyclePostconditionError) -> ExecutionError {
    match error {
        LifecyclePostconditionError::ObservationFailed(failure) => {
            native_observation_failure(failure)
        }
        LifecyclePostconditionError::ExpectedPresent => native_postcondition_failure(
            "native.postcondition.expected_present",
            "The native command succeeded, but the resource was not present in the requested scope.",
        ),
        LifecyclePostconditionError::ExpectedMissing => native_postcondition_failure(
            "native.postcondition.expected_missing",
            "The native command succeeded, but the resource remained present in the requested scope.",
        ),
    }
}

fn native_observation_failure(failure: NativeObservationFailure) -> ExecutionError {
    native_postcondition_failure(failure.diagnostic_code(), failure.summary())
}

fn native_postcondition_failure(code: &'static str, detail: &'static str) -> ExecutionError {
    ExecutionError::apply_failure(skilltap_core::domain::AttentionReason::operation_failed(
        EvidenceCode::new(code).expect("static evidence code is valid"),
        EvidenceDetail::new(detail).expect("static evidence detail is valid"),
    ))
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

#[cfg(test)]
mod tests {
    use super::*;
    use skilltap_core::domain::AbsolutePath;

    fn dispatch(
        adapter: &'static dyn crate::HarnessAdapter,
        action: NativeLifecycleAction,
        scope: Scope,
    ) -> NativeLifecycleDispatch {
        NativeLifecycleDispatch::new(
            adapter.identity().id,
            adapter.native_lifecycle().expect("test adapter lifecycle"),
            NativeLifecycleRequest {
                action,
                scope,
                name: NativeId::new("formatter@team").unwrap(),
                source: Some(SourceLocator::new("https://example.invalid/team.git").unwrap()),
            },
        )
    }

    fn codex(action: NativeLifecycleAction, scope: Scope) -> NativeLifecycleDispatch {
        dispatch(crate::CodexAdapter::static_ref(), action, scope)
    }

    fn claude(action: NativeLifecycleAction, scope: Scope) -> NativeLifecycleDispatch {
        dispatch(crate::ClaudeAdapter::static_ref(), action, scope)
    }

    #[test]
    fn native_vectors_use_direct_arguments_and_scope_mapping() {
        assert_eq!(
            native_arguments(&claude(NativeLifecycleAction::PluginInstall, Scope::Global)).unwrap(),
            ["plugin", "install", "formatter@team", "--scope", "user"].map(OsString::from)
        );
        assert_eq!(
            native_arguments(&codex(NativeLifecycleAction::MarketplaceAdd, Scope::Global)).unwrap(),
            [
                "plugin",
                "marketplace",
                "add",
                "https://example.invalid/team.git"
            ]
            .map(OsString::from)
        );
        assert_eq!(
            native_arguments(&claude(NativeLifecycleAction::PluginUpdate, Scope::Global)).unwrap(),
            ["plugin", "update", "formatter@team", "--scope", "user"].map(OsString::from)
        );
        let project = Scope::Project(AbsolutePath::new("/tmp/project").unwrap());
        assert_eq!(
            native_arguments(&claude(
                NativeLifecycleAction::MarketplaceUpdate,
                project.clone()
            ))
            .unwrap(),
            ["plugin", "marketplace", "update", "formatter@team"].map(OsString::from)
        );
        assert_eq!(
            native_list_arguments(
                claude(NativeLifecycleAction::MarketplaceUpdate, project).request()
            ),
            ["plugin", "marketplace", "list", "--json"].map(OsString::from)
        );
        assert_eq!(
            native_list_arguments(
                claude(NativeLifecycleAction::PluginInstall, Scope::Global).request()
            ),
            ["plugin", "list", "--json"].map(OsString::from)
        );
        assert_eq!(
            native_arguments(&codex(
                NativeLifecycleAction::MarketplaceRemove,
                Scope::Global
            ))
            .unwrap(),
            ["plugin", "marketplace", "remove", "formatter@team"].map(OsString::from)
        );
        assert_eq!(
            native_arguments(&codex(
                NativeLifecycleAction::PluginInstall,
                Scope::Project(AbsolutePath::new("/tmp/project").unwrap()),
            )),
            Err(NativeLifecycleError::UnsupportedProjectScope)
        );
        assert_eq!(
            native_arguments(&codex(NativeLifecycleAction::PluginUpdate, Scope::Global)),
            Err(NativeLifecycleError::UnsupportedAction)
        );
    }

    #[test]
    fn native_vectors_reject_option_like_untrusted_values() {
        let mut name = claude(NativeLifecycleAction::PluginInstall, Scope::Global);
        name.request.name = NativeId::new("--help").unwrap();
        assert_eq!(
            native_arguments(&name),
            Err(NativeLifecycleError::OptionLikeArgument("name"))
        );

        let mut source = codex(NativeLifecycleAction::MarketplaceAdd, Scope::Global);
        source.request.source = Some(SourceLocator::new("--upload-pack=evil").unwrap());
        assert_eq!(
            native_arguments(&source),
            Err(NativeLifecycleError::OptionLikeArgument("source"))
        );
    }

    #[test]
    fn native_resource_presence_is_conservative_and_identity_bound() {
        let request = codex(NativeLifecycleAction::PluginInstall, Scope::Global);
        assert_eq!(
            resource_observation(
                &serde_json::json!({"plugins": [{"name": "formatter@team"}]}),
                &request,
            ),
            NativeResourceObservation::Present {
                scope: None,
                revision: None,
            }
        );
        assert_eq!(
            resource_observation(&serde_json::json!({"plugins": []}), &request),
            NativeResourceObservation::Missing
        );
        assert_eq!(
            resource_observation(&serde_json::json!({"version": "3.0.0"}), &request),
            NativeResourceObservation::Indeterminate(NativeObservationFailure::UnsupportedShape)
        );
        for malformed in [
            serde_json::json!([1]),
            serde_json::json!([{}]),
            serde_json::json!({"plugins": "garbage"}),
            serde_json::json!({"plugins": [{}]}),
        ] {
            assert_eq!(
                resource_observation(&malformed, &request),
                NativeResourceObservation::Indeterminate(
                    NativeObservationFailure::UnsupportedShape
                ),
                "malformed list payload: {malformed}"
            );
        }
    }

    #[test]
    fn native_revision_evidence_is_strict_and_opaque() {
        let request = claude(NativeLifecycleAction::PluginUpdate, Scope::Global);
        assert_eq!(
            resource_observation(
                &serde_json::json!({
                    "plugins": [{"name": "formatter@team", "scope": "user", "version": "1.2.3"}]
                }),
                &request,
            ),
            NativeResourceObservation::Present {
                scope: Some(CapabilityScope::Global),
                revision: Some(ResolvedRevision::Native(NativeId::new("1.2.3").unwrap())),
            }
        );
        assert_eq!(
            resource_observation(
                &serde_json::json!({
                    "plugins": [{"name": "formatter@team", "scope": "user", "version": "1.2.3", "revision": "1.2.4"}]
                }),
                &request,
            ),
            NativeResourceObservation::Indeterminate(NativeObservationFailure::UnsupportedShape)
        );
        assert_eq!(
            resource_observation(
                &serde_json::json!({
                    "plugins": [{"name": "formatter@team", "scope": "user", "version": 3}]
                }),
                &request,
            ),
            NativeResourceObservation::Indeterminate(NativeObservationFailure::UnsupportedShape)
        );
    }

    #[test]
    fn claude_presence_matches_identity_and_concrete_scope() {
        let global = claude(NativeLifecycleAction::PluginInstall, Scope::Global);
        let project = claude(
            NativeLifecycleAction::PluginInstall,
            Scope::Project(AbsolutePath::new("/tmp/project").unwrap()),
        );
        let global_only = serde_json::json!({
            "plugins": [{"name": "formatter@team", "scope": "user"}]
        });
        assert_eq!(
            resource_observation(&global_only, &global),
            NativeResourceObservation::Present {
                scope: Some(CapabilityScope::Global),
                revision: None,
            }
        );
        assert_eq!(
            resource_observation(&global_only, &project),
            NativeResourceObservation::Missing
        );
        let siblings = serde_json::json!({
            "plugins": [
                {"name": "formatter@team", "scope": "user"},
                {"name": "formatter@team", "scope": "local"}
            ]
        });
        assert_eq!(
            resource_observation(&siblings, &global),
            NativeResourceObservation::Present {
                scope: Some(CapabilityScope::Global),
                revision: None,
            }
        );
        assert_eq!(
            resource_observation(&siblings, &project),
            NativeResourceObservation::Present {
                scope: Some(CapabilityScope::Project),
                revision: None,
            }
        );
    }

    #[test]
    fn claude_scope_evidence_fails_closed_when_missing_malformed_or_duplicate() {
        let project = claude(
            NativeLifecycleAction::PluginInstall,
            Scope::Project(AbsolutePath::new("/tmp/project").unwrap()),
        );
        for uncertain in [
            serde_json::json!({"plugins": [{"name": "formatter@team"}]}),
            serde_json::json!({"plugins": [{"name": "formatter@team", "scope": "project"}]}),
            serde_json::json!({"plugins": [
                {"name": "formatter@team", "scope": "local"},
                {"name": "formatter@team", "scope": "local"}
            ]}),
        ] {
            assert_eq!(
                resource_observation(&uncertain, &project),
                NativeResourceObservation::Indeterminate(NativeObservationFailure::AmbiguousScope),
                "uncertain scoped payload: {uncertain}"
            );
        }
        for unsupported in [
            serde_json::json!({"plugins": [{"name": "formatter@team", "scope": 3}]}),
            serde_json::json!({"plugins": [{
                "name": "formatter@team",
                "id": "different@team",
                "scope": "local"
            }]}),
        ] {
            assert_eq!(
                resource_observation(&unsupported, &project),
                NativeResourceObservation::Indeterminate(
                    NativeObservationFailure::UnsupportedShape
                ),
                "unsupported scoped payload: {unsupported}"
            );
        }
    }

    #[test]
    fn lifecycle_postconditions_require_fresh_expected_presence() {
        assert_eq!(
            verify_lifecycle_postcondition(
                NativeLifecycleAction::PluginInstall,
                NativeResourceObservation::Present {
                    scope: None,
                    revision: None,
                },
            ),
            Ok(())
        );
        assert_eq!(
            verify_lifecycle_postcondition(
                NativeLifecycleAction::PluginInstall,
                NativeResourceObservation::Missing,
            ),
            Err(LifecyclePostconditionError::ExpectedPresent)
        );
        assert_eq!(
            verify_lifecycle_postcondition(
                NativeLifecycleAction::PluginRemove,
                NativeResourceObservation::Present {
                    scope: None,
                    revision: None,
                },
            ),
            Err(LifecyclePostconditionError::ExpectedMissing)
        );
        assert_eq!(
            verify_lifecycle_postcondition(
                NativeLifecycleAction::PluginRemove,
                NativeResourceObservation::Indeterminate(NativeObservationFailure::InvalidJson),
            ),
            Err(LifecyclePostconditionError::ObservationFailed(
                NativeObservationFailure::InvalidJson,
            ))
        );
    }
}
