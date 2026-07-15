//! Shared conditional-profile resolution and mutation authorization.
//!
//! Pi is the first compound target. Its package files are user-owned evidence,
//! so this module owns the only path that turns a detected executable and
//! companion observations into a scoped capability decision. Callers must use
//! the resulting decision before constructing operations or state seeds.

use super::*;

use skilltap_core::{
    domain::{
        CapabilityId, CapabilitySupport, ConditionalProfileError, ConditionalProfileObservation,
        ConfiguredBinary, HarnessId, HarnessReachability, HarnessSet, NativeVersion, Scope,
    },
    runtime::{ConfinedFileSystem, ProcessLimits},
    storage::ConfigDocument,
};
use skilltap_harnesses::{
    ConditionalProfileContext, ConditionalProfilePort, DetectionError, HarnessAdapter,
    TargetRegistry, detect_configured_installation,
};

/// A composed conditional profile bound to one exact runtime version and scope.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ResolvedConditionalProfile {
    pub(super) core_version: NativeVersion,
    pub(super) observation: ConditionalProfileObservation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum ConditionalProfileResolutionError {
    Detection(DetectionError),
    InvalidConfiguredBinary,
    Unreachable,
    EnvironmentUnavailable,
    Contract(ConditionalProfileError),
}

/// Resolve the configured executable, inspect its conditional companions, and
/// compose the compiled profile with narrowing evidence. Adapters without a
/// conditional port deliberately return `None`, preserving ordinary targets.
#[allow(clippy::too_many_arguments)]
pub(super) fn resolve_conditional_profile(
    registry: &TargetRegistry,
    config: &ConfigDocument,
    target: &HarnessId,
    scope: &Scope,
    paths: &PlatformPaths,
    process_limits: ProcessLimits,
    json_limits: JsonLimits,
    filesystem: &dyn ConfinedFileSystem,
) -> Result<Option<ResolvedConditionalProfile>, ConditionalProfileResolutionError> {
    let Some(adapter) = registry.adapter(target) else {
        return Ok(None);
    };
    let Some(profile_port) = adapter.conditional_profile() else {
        return Ok(None);
    };
    let Some(policy) = config.harnesses().get(target) else {
        return Ok(None);
    };
    let configured = configured_binary(policy.binary.as_str())
        .map_err(|_| ConditionalProfileResolutionError::InvalidConfiguredBinary)?;
    resolve_conditional_profile_for_installation(
        adapter,
        profile_port,
        target,
        scope,
        paths,
        configured,
        config,
        process_limits,
        json_limits,
        filesystem,
    )
}

#[allow(clippy::too_many_arguments)]
fn resolve_conditional_profile_for_installation(
    adapter: &'static dyn HarnessAdapter,
    profile_port: &dyn ConditionalProfilePort,
    _target: &HarnessId,
    scope: &Scope,
    paths: &PlatformPaths,
    configured: ConfiguredBinary,
    _config: &ConfigDocument,
    process_limits: ProcessLimits,
    json_limits: JsonLimits,
    filesystem: &dyn ConfinedFileSystem,
) -> Result<Option<ResolvedConditionalProfile>, ConditionalProfileResolutionError> {
    let search_path = std::env::var_os("PATH");
    let environment = paths
        .native_process_environment(search_path.clone())
        .map_err(|_| ConditionalProfileResolutionError::EnvironmentUnavailable)?;
    let installation = detect_configured_installation(
        adapter,
        configured,
        search_path,
        &environment,
        process_limits,
        json_limits,
    )
    .map_err(ConditionalProfileResolutionError::Detection)?;
    let HarnessReachability::Reachable { native_version, .. } = installation.reachability() else {
        return Err(ConditionalProfileResolutionError::Unreachable);
    };
    let report = profile_port
        .inspect_components(&ConditionalProfileContext {
            scope,
            paths,
            filesystem,
            json_limits,
            maximum_manifest_bytes: json_limits.bytes(),
        })
        .map_err(ConditionalProfileResolutionError::Contract)?;
    let compiled = profile_port.select_compiled_profile(native_version, report.components());
    let observation = ConditionalProfileObservation::compose(compiled, report)
        .map_err(ConditionalProfileResolutionError::Contract)?;
    Ok(Some(ResolvedConditionalProfile {
        core_version: native_version.clone(),
        observation,
    }))
}

/// Require exact scoped mutation authorization. Observe-only and unknown
/// profiles fail closed even when their files happen to be present.
pub(super) fn require_target_mutation_capability(
    resolved: Option<&ResolvedConditionalProfile>,
    capability: &CapabilityId,
    scope: &Scope,
) -> Result<(), ErrorDetail> {
    let Some(resolved) = resolved else {
        return Ok(());
    };
    if resolved.observation.mutation_support(scope, capability) == CapabilitySupport::Supported {
        return Ok(());
    }
    let profile = resolved
        .observation
        .profile()
        .profile_id()
        .map_or_else(|| "unknown".to_owned(), |id| id.as_str().to_owned());
    Err(ErrorDetail::new(
        "conditional_profile_mutation_unauthorized",
        "The selected conditional harness profile is observe-only for this mutation.",
    )
    .with_context("capability", capability.as_str())
    .with_context("scope", scope_label(scope))
    .with_context("profile", profile)
    .with_next_action(NextAction::new(
        "inspect_conditional_profile",
        "Review the conditional harness status; no mutation-authorized profile is currently compiled.",
    )))
}

/// Filter a target set before an operation, canonical write, or state seed is
/// constructed. Authorized siblings remain eligible when one compound target
/// is blocked. Resolution failures are rendered as bounded attention output.
#[allow(clippy::too_many_arguments)]
pub(super) fn filter_targets_for_capability(
    registry: &TargetRegistry,
    config: &ConfigDocument,
    targets: &HarnessSet,
    scope: &Scope,
    paths: &PlatformPaths,
    process_limits: ProcessLimits,
    json_limits: JsonLimits,
    filesystem: &dyn ConfinedFileSystem,
    capability: &CapabilityId,
    mut outcome: Outcome,
) -> (Option<HarnessSet>, Outcome) {
    let mut authorized = Vec::new();
    for target in targets.iter() {
        let resolved = match resolve_conditional_profile(
            registry,
            config,
            target,
            scope,
            paths,
            process_limits,
            json_limits,
            filesystem,
        ) {
            Ok(resolved) => resolved,
            Err(error) => {
                outcome.result = merge_result(outcome.result, ResultClass::AttentionRequired);
                outcome = outcome
                    .with_warning(conditional_profile_warning(target, scope, &error))
                    .with_next_action(conditional_profile_next_action());
                continue;
            }
        };
        match require_target_mutation_capability(resolved.as_ref(), capability, scope) {
            Ok(()) => authorized.push(target.clone()),
            Err(error) => {
                outcome.result = merge_result(outcome.result, ResultClass::AttentionRequired);
                outcome = outcome
                    .with_error(error.with_context("target", target.as_str()))
                    .with_next_action(conditional_profile_next_action());
            }
        }
    }
    let authorized = HarnessSet::new(authorized).ok();
    (authorized, outcome)
}

pub(super) fn conditional_profile_warning(
    target: &HarnessId,
    scope: &Scope,
    error: &ConditionalProfileResolutionError,
) -> Warning {
    let reason = match error {
        ConditionalProfileResolutionError::Detection(_) => "detection",
        ConditionalProfileResolutionError::InvalidConfiguredBinary => "invalid_binary",
        ConditionalProfileResolutionError::Unreachable => "unreachable",
        ConditionalProfileResolutionError::EnvironmentUnavailable => "environment",
        ConditionalProfileResolutionError::Contract(_) => "contract",
    };
    Warning::new(
        "conditional_profile_unavailable",
        "The conditional harness profile could not be resolved safely; its mutation remains blocked.",
    )
    .with_context("target", target.as_str())
    .with_context("scope", scope_label(scope))
    .with_context("reason", reason)
}

pub(super) fn conditional_profile_next_action() -> NextAction {
    NextAction::new(
        "inspect_conditional_profile",
        "Review the conditional harness status; the current hook companion remains partial and cannot grant mutation authority.",
    )
}

// Keep this local to the resolver so all configured executable parsing follows
// the same bounded policy as the existing native lifecycle path.
fn configured_binary(binary: &str) -> Result<ConfiguredBinary, ()> {
    super::configured_binary(binary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use skilltap_core::domain::{
        CapabilityProfileId, CapabilitySet, ConditionalComponentReport, ScopedCapabilitySets,
    };

    #[test]
    fn mutation_guard_fails_closed_for_observe_only_profile() {
        let profile = ResolvedConditionalProfile {
            core_version: NativeVersion::new("0.80.6").unwrap(),
            observation: ConditionalProfileObservation::compose(
                skilltap_core::domain::CapabilityProfileSelection::verified_observe_only(
                    CapabilityProfileId::new("pi-test").unwrap(),
                    ScopedCapabilitySets::new(
                        CapabilitySet::new([(
                            CapabilityId::new("skill.install").unwrap(),
                            CapabilitySupport::Unsupported,
                        )]),
                        CapabilitySet::new([(
                            CapabilityId::new("skill.install").unwrap(),
                            CapabilitySupport::Unsupported,
                        )]),
                    ),
                ),
                ConditionalComponentReport::from_components(
                    [],
                    ScopedCapabilitySets::new(
                        CapabilitySet::new([(
                            CapabilityId::new("skill.install").unwrap(),
                            CapabilitySupport::Unsupported,
                        )]),
                        CapabilitySet::new([(
                            CapabilityId::new("skill.install").unwrap(),
                            CapabilitySupport::Unsupported,
                        )]),
                    ),
                    [],
                )
                .unwrap(),
            )
            .unwrap(),
        };
        for capability in [
            "skill.install",
            "skill.update",
            "skill.remove",
            "marketplace.register",
            "marketplace.update",
            "marketplace.remove",
            "plugin.install",
            "plugin.update",
            "plugin.remove",
            "managed.projection",
        ] {
            let capability = CapabilityId::new(capability).unwrap();
            assert!(
                require_target_mutation_capability(Some(&profile), &capability, &Scope::Global)
                    .is_err()
            );
        }
        let capability = CapabilityId::new("skill.install").unwrap();
        assert!(require_target_mutation_capability(None, &capability, &Scope::Global).is_ok());
    }
}
