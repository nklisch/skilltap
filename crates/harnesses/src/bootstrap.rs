//! Read-first bootstrap setup for the canonical skilltap plugin.

use std::{ffi::OsString, fmt};

use skilltap_core::{
    domain::{
        CapabilityId, CapabilitySupport, ConfiguredBinary, HarnessInstallation,
        HarnessReachability, NativeId, NativeVersion, Scope, SourceLocator,
    },
    runtime::{JsonLimits, ProcessLimits},
};

use crate::{
    HarnessKind, NativeLifecycleAction, NativeLifecycleRequest, NativeResourcePresence,
    detect_configured_installation, observe_native_resource, run_native_lifecycle, select_profile,
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
        harness: HarnessKind,
        version: NativeVersion,
    },
    AlreadyPresent {
        harness: HarnessKind,
        version: NativeVersion,
    },
    Unavailable {
        harness: HarnessKind,
        reason: SetupReason,
    },
    Unsupported {
        harness: HarnessKind,
        next_action: String,
    },
    Failed {
        harness: HarnessKind,
        reason: SetupReason,
    },
}

impl HarnessSetupResult {
    pub const fn harness(&self) -> HarnessKind {
        match self {
            Self::Installed { harness, .. }
            | Self::AlreadyPresent { harness, .. }
            | Self::Unavailable { harness, .. }
            | Self::Unsupported { harness, .. }
            | Self::Failed { harness, .. } => *harness,
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
                SourceLocator::new("https://github.com/nklisch/skilltap")
                    .expect("canonical source is valid"),
            ),
        }
    }
}

pub fn setup_first_party_plugin(
    target: HarnessKind,
    policy: &HarnessBootstrapPolicy,
) -> HarnessSetupResult {
    let installation = match detect_configured_installation(
        target,
        policy.configured.clone(),
        policy.search_path.clone(),
        policy.process_limits,
        policy.json_limits,
    ) {
        Ok(installation) => installation,
        Err(_) => {
            return HarnessSetupResult::Unavailable {
                harness: target,
                reason: SetupReason::NotInstalled,
            };
        }
    };
    setup_detected_plugin(target, &installation, policy)
}

pub fn setup_detected_plugin(
    target: HarnessKind,
    installation: &HarnessInstallation,
    policy: &HarnessBootstrapPolicy,
) -> HarnessSetupResult {
    let HarnessReachability::Reachable { native_version, .. } = installation.reachability() else {
        return HarnessSetupResult::Unavailable {
            harness: target,
            reason: SetupReason::NotInstalled,
        };
    };
    let profile = select_profile(target, native_version);
    let capability = CapabilityId::new("plugin.install").expect("compiled capability id is valid");
    let support = profile
        .mutation_capabilities()
        .and_then(|capabilities| capabilities.for_scope(&Scope::Global).support(&capability));
    if !matches!(support, Some(CapabilitySupport::Supported)) {
        return HarnessSetupResult::Unsupported {
            harness: target,
            next_action: unsupported_next_action(target),
        };
    }
    let request = NativeLifecycleRequest {
        harness: target,
        action: NativeLifecycleAction::PluginInstall,
        scope: Scope::Global,
        name: policy.plugin_name.clone(),
        source: policy.canonical_source.clone(),
    };
    let presence = observe_native_resource(
        policy.configured.clone(),
        policy.search_path.clone(),
        &request,
        policy.process_limits,
        policy.json_limits,
    );
    match presence {
        Ok(NativeResourcePresence::Present) => {
            return HarnessSetupResult::AlreadyPresent {
                harness: target,
                version: native_version.clone(),
            };
        }
        Ok(NativeResourcePresence::Unknown) | Err(_) => {
            return HarnessSetupResult::Failed {
                harness: target,
                reason: SetupReason::UnknownNativeState,
            };
        }
        Ok(NativeResourcePresence::Missing) => {}
    }
    match run_native_lifecycle(
        policy.configured.clone(),
        policy.search_path.clone(),
        &request,
        policy.process_limits,
    ) {
        Ok(output) if output.status().success() => HarnessSetupResult::Installed {
            harness: target,
            version: native_version.clone(),
        },
        Ok(_) => HarnessSetupResult::Failed {
            harness: target,
            reason: SetupReason::NativeCommandFailed,
        },
        Err(_) => HarnessSetupResult::Failed {
            harness: target,
            reason: SetupReason::NativeCommandUnavailable,
        },
    }
}

fn unsupported_next_action(target: HarnessKind) -> String {
    match target {
        HarnessKind::Claude => "Run `claude plugin install skilltap --scope user` through Claude's native consent flow.".to_owned(),
        HarnessKind::Codex => "Run the documented Codex plugin flow, or use the standalone skill under `.agents/skills/skilltap/`; skilltap will not write Codex caches.".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use skilltap_core::domain::{AbsolutePath, HarnessId, NativeVersion, UnreachableReason};
    use skilltap_core::domain::{ExecutableFileIdentity, ExecutableIdentity};

    fn installation(target: HarnessKind, version: &str) -> HarnessInstallation {
        HarnessInstallation::new(
            HarnessId::new(target.id()).unwrap(),
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
            HarnessKind::Codex,
            &installation(HarnessKind::Codex, "99.0.0"),
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
        let result = setup_detected_plugin(HarnessKind::Claude, &installation, &policy);
        assert_eq!(
            result,
            HarnessSetupResult::Unavailable {
                harness: HarnessKind::Claude,
                reason: SetupReason::NotInstalled
            }
        );
    }
}
