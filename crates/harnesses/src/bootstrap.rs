//! Read-first bootstrap setup for the canonical skilltap plugin.

use std::{collections::BTreeMap, ffi::OsString, fmt};

use skilltap_core::{
    domain::{
        CapabilityId, CapabilitySupport, ConfiguredBinary, HarnessInstallation,
        HarnessReachability, NativeId, NativeVersion, Scope, SourceLocator,
    },
    runtime::{JsonLimits, ProcessLimits},
};

use crate::{
    DetectionError, DistributionSurface, HarnessAdapter, NativeLifecycleAction,
    NativeLifecycleDispatch, NativeLifecycleRequest, NativeResourceObservation,
    detect_configured_installation, observe_native_resource, run_native_lifecycle_bound,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SetupReason {
    NotInstalled,
    InvalidVersion,
    UnknownNativeState,
    UnsupportedCapability,
    NativeCommandFailed,
    NativeCommandUnavailable,
}

impl fmt::Display for SetupReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NotInstalled => "the harness executable is not installed or not reachable",
            Self::InvalidVersion => "the harness version could not be validated",
            Self::UnknownNativeState => {
                "native plugin list output was missing or malformed; mutation was blocked"
            }
            Self::UnsupportedCapability => {
                "the verified harness profile does not grant non-interactive plugin installation"
            }
            Self::NativeCommandFailed => "the native plugin lifecycle command failed",
            Self::NativeCommandUnavailable => {
                "the configured harness lifecycle command is unavailable"
            }
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HarnessSetupResult {
    Installed {
        harness: skilltap_core::domain::HarnessId,
        version: NativeVersion,
    },
    AlreadyPresent {
        harness: skilltap_core::domain::HarnessId,
        version: NativeVersion,
    },
    Unavailable {
        harness: skilltap_core::domain::HarnessId,
        reason: SetupReason,
    },
    Unsupported {
        harness: skilltap_core::domain::HarnessId,
        next_action: String,
    },
    Failed {
        harness: skilltap_core::domain::HarnessId,
        reason: SetupReason,
    },
}

impl HarnessSetupResult {
    pub const fn harness(&self) -> &skilltap_core::domain::HarnessId {
        match self {
            Self::Installed { harness, .. }
            | Self::AlreadyPresent { harness, .. }
            | Self::Unavailable { harness, .. }
            | Self::Unsupported { harness, .. }
            | Self::Failed { harness, .. } => harness,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HarnessBootstrapPolicy {
    pub configured: ConfiguredBinary,
    pub search_path: Option<OsString>,
    pub process_limits: ProcessLimits,
    pub json_limits: JsonLimits,
    pub plugin_name: NativeId,
    pub canonical_source: Option<SourceLocator>,
    pub environment: BTreeMap<OsString, OsString>,
}

impl HarnessBootstrapPolicy {
    pub fn skilltap(configured: ConfiguredBinary, search_path: Option<OsString>) -> Self {
        Self {
            configured,
            search_path,
            process_limits: ProcessLimits::new(30_000, 64 * 1024, 64 * 1024, 128 * 1024)
                .expect("static bootstrap process limits are valid"),
            json_limits: JsonLimits::new(128 * 1024, 32)
                .expect("static bootstrap JSON limits are valid"),
            plugin_name: NativeId::new("skilltap").expect("canonical plugin id is valid"),
            canonical_source: Some(
                SourceLocator::new("https://github.com/nklisch/skilltap/tree/main/plugin")
                    .expect("canonical source is valid"),
            ),
            environment: BTreeMap::new(),
        }
    }

    pub fn with_environment(mut self, environment: BTreeMap<OsString, OsString>) -> Self {
        self.environment = environment;
        self
    }
}

pub fn setup_first_party_plugin(
    target: &'static dyn HarnessAdapter,
    policy: &HarnessBootstrapPolicy,
) -> HarnessSetupResult {
    let harness = target.identity().id;
    let installation = match detect_configured_installation(
        target,
        policy.configured.clone(),
        policy.search_path.clone(),
        &policy.environment,
        policy.process_limits,
        policy.json_limits,
    ) {
        Ok(installation) => installation,
        Err(DetectionError::InvalidVersion | DetectionError::NonZeroExit) => {
            return HarnessSetupResult::Unavailable {
                harness: harness.clone(),
                reason: SetupReason::InvalidVersion,
            };
        }
        Err(_) => {
            return HarnessSetupResult::Unavailable {
                harness: harness.clone(),
                reason: SetupReason::NotInstalled,
            };
        }
    };
    setup_detected_plugin(target, &installation, policy)
}

pub fn setup_detected_plugin(
    target: &'static dyn HarnessAdapter,
    installation: &HarnessInstallation,
    policy: &HarnessBootstrapPolicy,
) -> HarnessSetupResult {
    let harness = target.identity().id;
    let HarnessReachability::Reachable { native_version, .. } = installation.reachability() else {
        return HarnessSetupResult::Unavailable {
            harness,
            reason: SetupReason::NotInstalled,
        };
    };
    if target.identity().distribution_surface != DistributionSurface::FirstPartyPlugin {
        return HarnessSetupResult::Unsupported {
            harness,
            next_action: "Use the target's managed distribution workflow; this target has no first-party skilltap plugin.".to_owned(),
        };
    }
    if let Some(next_action) = target.bootstrap_next_action() {
        return HarnessSetupResult::Unsupported {
            harness,
            next_action: next_action.to_owned(),
        };
    }
    let profile = target.select_profile(native_version);
    let capabilities = profile.mutation_capabilities();
    let supports = |name: &str| {
        let capability = CapabilityId::new(name).expect("compiled capability id is valid");
        capabilities
            .and_then(|sets| sets.for_scope(&Scope::Global).support(&capability))
            .is_some_and(|support| matches!(support, CapabilitySupport::Supported))
    };
    // Each native mutation has its own capability gate.  Keep these checks
    // explicit so a profile cannot accidentally authorize marketplace
    // registration by proving only plugin installation (or vice versa).
    if !supports("marketplace.register") || !supports("plugin.install") {
        return HarnessSetupResult::Unsupported {
            harness,
            next_action: target.bootstrap_capability_next_action().to_owned(),
        };
    }
    let observed_executable = match installation.reachability() {
        HarnessReachability::Reachable { executable, .. } => executable,
        HarnessReachability::Unreachable { .. } => unreachable!("reachable checked above"),
    };
    let observed_configured = ConfiguredBinary::absolute(observed_executable.path().clone());
    let marketplace_request = NativeLifecycleRequest {
        action: NativeLifecycleAction::MarketplaceAdd,
        scope: Scope::Global,
        name: NativeId::new("skilltap").expect("canonical marketplace id is valid"),
        source: policy.canonical_source.clone(),
    };
    let Some(lifecycle) = target.native_lifecycle() else {
        return HarnessSetupResult::Unsupported {
            harness,
            next_action: target.bootstrap_capability_next_action().to_owned(),
        };
    };
    let marketplace_dispatch =
        NativeLifecycleDispatch::new(harness.clone(), lifecycle, marketplace_request);
    match observe_native_resource(
        observed_configured.clone(),
        None,
        &policy.environment,
        &marketplace_dispatch,
        policy.process_limits,
        policy.json_limits,
    ) {
        Ok(NativeResourceObservation::Present { .. }) => {}
        Ok(NativeResourceObservation::Missing) => {
            match run_native_lifecycle_bound(
                observed_executable,
                &policy.environment,
                &marketplace_dispatch,
                policy.process_limits,
            ) {
                Ok(output) if output.status().success() => {}
                Ok(_) => {
                    return HarnessSetupResult::Failed {
                        harness: harness.clone(),
                        reason: SetupReason::NativeCommandFailed,
                    };
                }
                Err(_) => {
                    return HarnessSetupResult::Failed {
                        harness: harness.clone(),
                        reason: SetupReason::NativeCommandUnavailable,
                    };
                }
            }
        }
        Ok(NativeResourceObservation::Indeterminate(_)) | Err(_) => {
            return HarnessSetupResult::Failed {
                harness: harness.clone(),
                reason: SetupReason::UnknownNativeState,
            };
        }
    }
    let request = NativeLifecycleRequest {
        action: NativeLifecycleAction::PluginInstall,
        scope: Scope::Global,
        name: NativeId::new(format!("{}@skilltap", policy.plugin_name.as_str()))
            .expect("canonical qualified plugin id is valid"),
        source: None,
    };
    let dispatch = NativeLifecycleDispatch::new(harness.clone(), lifecycle, request);
    let presence = observe_native_resource(
        observed_configured.clone(),
        None,
        &policy.environment,
        &dispatch,
        policy.process_limits,
        policy.json_limits,
    );
    match presence {
        Ok(NativeResourceObservation::Present { .. }) => {
            return HarnessSetupResult::AlreadyPresent {
                harness,
                version: native_version.clone(),
            };
        }
        Ok(NativeResourceObservation::Indeterminate(_)) | Err(_) => {
            return HarnessSetupResult::Failed {
                harness,
                reason: SetupReason::UnknownNativeState,
            };
        }
        Ok(NativeResourceObservation::Missing) => {}
    }
    match run_native_lifecycle_bound(
        observed_executable,
        &policy.environment,
        &dispatch,
        policy.process_limits,
    ) {
        Ok(output) if output.status().success() => HarnessSetupResult::Installed {
            harness: harness.clone(),
            version: native_version.clone(),
        },
        Ok(_) => HarnessSetupResult::Failed {
            harness: harness.clone(),
            reason: SetupReason::NativeCommandFailed,
        },
        Err(_) => HarnessSetupResult::Failed {
            harness,
            reason: SetupReason::NativeCommandUnavailable,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use skilltap_core::domain::{AbsolutePath, HarnessId, NativeVersion, UnreachableReason};
    use skilltap_core::domain::{ExecutableFileIdentity, ExecutableIdentity};

    fn installation(target: &'static dyn HarnessAdapter, version: &str) -> HarnessInstallation {
        HarnessInstallation::new(
            target.identity().id,
            ConfiguredBinary::absolute(AbsolutePath::new("/tmp/fake-harness").unwrap()),
            HarnessReachability::Reachable {
                executable: ExecutableIdentity::new(
                    AbsolutePath::new("/tmp/fake-harness").unwrap(),
                    ExecutableFileIdentity::new(1, 2),
                ),
                native_version: NativeVersion::new(version).unwrap(),
            },
        )
    }

    #[test]
    fn unknown_versions_are_unsupported_with_agent_next_actions() {
        let policy = HarnessBootstrapPolicy::skilltap(
            ConfiguredBinary::absolute(AbsolutePath::new("/tmp/fake-harness").unwrap()),
            None,
        );
        let result = setup_detected_plugin(
            crate::CodexAdapter::static_ref(),
            &installation(crate::CodexAdapter::static_ref(), "99.0.0"),
            &policy,
        );
        assert!(matches!(result, HarnessSetupResult::Unsupported { .. }));
        assert!(format!("{result:?}").contains("standalone skill"));
    }

    #[test]
    fn unreachable_installations_are_bounded() {
        let installation = HarnessInstallation::new(
            HarnessId::new("claude").unwrap(),
            ConfiguredBinary::path_lookup(NativeId::new("claude").unwrap()).unwrap(),
            HarnessReachability::Unreachable {
                reason: UnreachableReason::NotFound,
            },
        );
        let policy = HarnessBootstrapPolicy::skilltap(
            ConfiguredBinary::path_lookup(NativeId::new("claude").unwrap()).unwrap(),
            None,
        );
        let result =
            setup_detected_plugin(crate::ClaudeAdapter::static_ref(), &installation, &policy);
        assert_eq!(
            result,
            HarnessSetupResult::Unavailable {
                harness: HarnessId::new("claude").unwrap(),
                reason: SetupReason::NotInstalled
            }
        );
    }
}
