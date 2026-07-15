//! Harness installation identity and compiled capability-profile contracts.

use std::{
    collections::BTreeMap,
    path::{Component, Path},
};

use serde::{Deserialize, Serialize};

use super::{
    AbsolutePath, CapabilityId, CapabilitySet, CapabilitySupport, HarnessId, NativeId, Scope,
    ValidationError, validate_identifier, validate_text,
    validated_newtype::validated_string_newtype,
};

validated_string_newtype!(
    NativeVersion,
    "native harness version",
    512,
    validate_text,
    try_from
);
validated_string_newtype!(
    CapabilityProfileId,
    "capability profile id",
    128,
    validate_identifier,
    try_from
);

/// The configured way to locate a harness executable, before any I/O occurs.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ConfiguredBinary {
    PathLookup(NativeId),
    Absolute(AbsolutePath),
}

impl ConfiguredBinary {
    pub fn path_lookup(name: NativeId) -> Result<Self, ValidationError> {
        let mut components = Path::new(name.as_str()).components();
        if !matches!(components.next(), Some(Component::Normal(_))) || components.next().is_some() {
            return Err(ValidationError::InvalidFormat {
                kind: "configured binary name",
                expected: "be one PATH executable name",
            });
        }
        Ok(Self::PathLookup(name))
    }

    pub const fn absolute(path: AbsolutePath) -> Self {
        Self::Absolute(path)
    }
}

/// Stable identity of one resolved Unix executable for a single observation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutableFileIdentity {
    device: u64,
    inode: u64,
}

impl ExecutableFileIdentity {
    pub const fn new(device: u64, inode: u64) -> Self {
        Self { device, inode }
    }

    pub const fn device(self) -> u64 {
        self.device
    }

    pub const fn inode(self) -> u64 {
        self.inode
    }
}

/// A canonical executable path bound to the exact file that was inspected.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutableIdentity {
    path: AbsolutePath,
    file: ExecutableFileIdentity,
}

impl ExecutableIdentity {
    pub const fn new(path: AbsolutePath, file: ExecutableFileIdentity) -> Self {
        Self { path, file }
    }

    pub const fn path(&self) -> &AbsolutePath {
        &self.path
    }

    pub const fn file(&self) -> ExecutableFileIdentity {
        self.file
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UnreachableReason {
    NotFound,
    NotExecutable,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum HarnessReachability {
    Reachable {
        executable: ExecutableIdentity,
        native_version: NativeVersion,
    },
    Unreachable {
        reason: UnreachableReason,
    },
}

/// Detection result for one configured harness without any profile assumption.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HarnessInstallation {
    harness: HarnessId,
    configured_binary: ConfiguredBinary,
    reachability: HarnessReachability,
}

impl HarnessInstallation {
    pub const fn new(
        harness: HarnessId,
        configured_binary: ConfiguredBinary,
        reachability: HarnessReachability,
    ) -> Self {
        Self {
            harness,
            configured_binary,
            reachability,
        }
    }

    pub const fn harness(&self) -> &HarnessId {
        &self.harness
    }

    pub const fn configured_binary(&self) -> &ConfiguredBinary {
        &self.configured_binary
    }

    pub const fn reachability(&self) -> &HarnessReachability {
        &self.reachability
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityScope {
    Global,
    Project,
}

impl From<&Scope> for CapabilityScope {
    fn from(scope: &Scope) -> Self {
        match scope {
            Scope::Global => Self::Global,
            Scope::Project(_) => Self::Project,
        }
    }
}

/// Compiled capabilities whose support may differ between global and project scope.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScopedCapabilitySets {
    global: CapabilitySet,
    project: CapabilitySet,
}

impl ScopedCapabilitySets {
    pub const fn new(global: CapabilitySet, project: CapabilitySet) -> Self {
        Self { global, project }
    }

    pub const fn for_scope_kind(&self, scope: CapabilityScope) -> &CapabilitySet {
        match scope {
            CapabilityScope::Global => &self.global,
            CapabilityScope::Project => &self.project,
        }
    }

    pub fn for_scope(&self, scope: &Scope) -> &CapabilitySet {
        self.for_scope_kind(scope.into())
    }

    pub fn narrow(&self, narrowing: &ScopedCapabilitySets) -> Result<Self, ProfileContractError> {
        Ok(Self {
            global: narrow_set(&self.global, &narrowing.global, CapabilityScope::Global)?,
            project: narrow_set(&self.project, &narrowing.project, CapabilityScope::Project)?,
        })
    }
}

fn narrow_set(
    baseline: &CapabilitySet,
    narrowing: &CapabilitySet,
    scope: CapabilityScope,
) -> Result<CapabilitySet, ProfileContractError> {
    let mut result = baseline
        .iter()
        .map(|(id, support)| (id.clone(), support))
        .collect::<BTreeMap<_, _>>();
    for (capability, narrowed) in narrowing.iter() {
        let Some(baseline_support) = baseline.support(capability) else {
            return Err(ProfileContractError::UnknownCapability {
                scope,
                capability: capability.clone(),
            });
        };
        if !is_narrowing(baseline_support, narrowed) {
            return Err(ProfileContractError::CapabilityWidened {
                scope,
                capability: capability.clone(),
                baseline: baseline_support,
                proposed: narrowed,
            });
        }
        result.insert(capability.clone(), narrowed);
    }
    Ok(CapabilitySet::new(result))
}

const fn is_narrowing(baseline: CapabilitySupport, proposed: CapabilitySupport) -> bool {
    matches!(
        (baseline, proposed),
        (CapabilitySupport::Supported, _)
            | (CapabilitySupport::Unverified, CapabilitySupport::Unverified)
            | (
                CapabilitySupport::Unverified,
                CapabilitySupport::Unsupported
            )
            | (
                CapabilitySupport::Unsupported,
                CapabilitySupport::Unsupported
            )
    )
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileAuthority {
    VerifiedCompiled,
    ObserveOnly,
}

/// A version/profile result. Only a verified compiled profile grants mutation authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "authority", rename_all = "snake_case", deny_unknown_fields)]
pub enum CapabilityProfileSelection {
    VerifiedCompiled {
        id: CapabilityProfileId,
        capabilities: ScopedCapabilitySets,
    },
    VerifiedObserveOnly {
        id: CapabilityProfileId,
        capabilities: ScopedCapabilitySets,
    },
    UnknownVersion {
        capabilities: ScopedCapabilitySets,
    },
}

impl CapabilityProfileSelection {
    pub const fn verified(id: CapabilityProfileId, capabilities: ScopedCapabilitySets) -> Self {
        Self::VerifiedCompiled { id, capabilities }
    }

    pub const fn verified_observe_only(
        id: CapabilityProfileId,
        capabilities: ScopedCapabilitySets,
    ) -> Self {
        Self::VerifiedObserveOnly { id, capabilities }
    }

    pub const fn unknown_version(capabilities: ScopedCapabilitySets) -> Self {
        Self::UnknownVersion { capabilities }
    }

    pub const fn authority(&self) -> ProfileAuthority {
        match self {
            Self::VerifiedCompiled { .. } => ProfileAuthority::VerifiedCompiled,
            Self::VerifiedObserveOnly { .. } | Self::UnknownVersion { .. } => {
                ProfileAuthority::ObserveOnly
            }
        }
    }

    pub const fn profile_id(&self) -> Option<&CapabilityProfileId> {
        match self {
            Self::VerifiedCompiled { id, .. } | Self::VerifiedObserveOnly { id, .. } => Some(id),
            Self::UnknownVersion { .. } => None,
        }
    }

    pub const fn observation_capabilities(&self) -> &ScopedCapabilitySets {
        match self {
            Self::VerifiedCompiled { capabilities, .. }
            | Self::VerifiedObserveOnly { capabilities, .. }
            | Self::UnknownVersion { capabilities } => capabilities,
        }
    }

    pub const fn mutation_capabilities(&self) -> Option<&ScopedCapabilitySets> {
        match self {
            Self::VerifiedCompiled { capabilities, .. } => Some(capabilities),
            Self::VerifiedObserveOnly { .. } | Self::UnknownVersion { .. } => None,
        }
    }

    /// Returns the exact scoped capability from a mutation-authorized profile.
    ///
    /// `None` is intentional: an unknown/observe-only profile and an absent
    /// capability are both unable to authorize mutation. Observation callers
    /// should use [`Self::observation_capabilities`] when they need to retain
    /// diagnostic evidence for those profiles.
    pub fn mutation_support(
        &self,
        scope: &Scope,
        capability: &CapabilityId,
    ) -> Option<CapabilitySupport> {
        self.mutation_capabilities()
            .and_then(|capabilities| capabilities.for_scope(scope).support(capability))
    }

    pub fn narrow(&self, narrowing: &ScopedCapabilitySets) -> Result<Self, ProfileContractError> {
        let capabilities = self.observation_capabilities().narrow(narrowing)?;
        Ok(match self {
            Self::VerifiedCompiled { id, .. } => Self::VerifiedCompiled {
                id: id.clone(),
                capabilities,
            },
            Self::VerifiedObserveOnly { id, .. } => Self::VerifiedObserveOnly {
                id: id.clone(),
                capabilities,
            },
            Self::UnknownVersion { .. } => Self::UnknownVersion { capabilities },
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProfileContractError {
    UnknownCapability {
        scope: CapabilityScope,
        capability: CapabilityId,
    },
    CapabilityWidened {
        scope: CapabilityScope,
        capability: CapabilityId,
        baseline: CapabilitySupport,
        proposed: CapabilitySupport,
    },
}

impl std::fmt::Display for ProfileContractError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownCapability { scope, capability } => write!(
                formatter,
                "probe references unknown {scope:?} capability `{capability}`"
            ),
            Self::CapabilityWidened {
                scope,
                capability,
                baseline,
                proposed,
            } => write!(
                formatter,
                "probe widens {scope:?} capability `{capability}` from {baseline:?} to {proposed:?}"
            ),
        }
    }
}

impl std::error::Error for ProfileContractError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn capability(value: &str) -> CapabilityId {
        CapabilityId::new(value).unwrap()
    }

    fn sets(support: CapabilitySupport) -> ScopedCapabilitySets {
        ScopedCapabilitySets::new(
            CapabilitySet::new([(capability("plugin.install"), support)]),
            CapabilitySet::new([(capability("plugin.install"), support)]),
        )
    }

    #[test]
    fn configured_binary_distinguishes_path_lookup_from_absolute_paths() {
        assert!(ConfiguredBinary::path_lookup(NativeId::new("codex").unwrap()).is_ok());
        assert!(ConfiguredBinary::path_lookup(NativeId::new("bin/codex").unwrap()).is_err());
        assert!(ConfiguredBinary::path_lookup(NativeId::new(".").unwrap()).is_err());
        assert!(ConfiguredBinary::path_lookup(NativeId::new("..").unwrap()).is_err());
        let absolute = ConfiguredBinary::absolute(AbsolutePath::new("/opt/codex").unwrap());
        assert!(matches!(absolute, ConfiguredBinary::Absolute(_)));
    }

    #[test]
    fn unreachable_installations_cannot_carry_executable_or_version_evidence() {
        let installation = HarnessInstallation::new(
            HarnessId::new("codex").unwrap(),
            ConfiguredBinary::path_lookup(NativeId::new("codex").unwrap()).unwrap(),
            HarnessReachability::Unreachable {
                reason: UnreachableReason::NotFound,
            },
        );
        assert!(matches!(
            installation.reachability(),
            HarnessReachability::Unreachable { .. }
        ));
    }

    #[test]
    fn reachable_installations_bind_version_to_one_exact_executable() {
        let identity = ExecutableIdentity::new(
            AbsolutePath::new("/opt/codex").unwrap(),
            ExecutableFileIdentity::new(41, 73),
        );
        let installation = HarnessInstallation::new(
            HarnessId::new("codex").unwrap(),
            ConfiguredBinary::absolute(AbsolutePath::new("/opt/codex").unwrap()),
            HarnessReachability::Reachable {
                executable: identity.clone(),
                native_version: NativeVersion::new("codex-cli 1.2.3+vendor").unwrap(),
            },
        );
        assert!(matches!(
            installation.reachability(),
            HarnessReachability::Reachable { executable, .. } if executable == &identity
        ));
        assert_eq!(identity.file().device(), 41);
        assert_eq!(identity.file().inode(), 73);
    }

    #[test]
    fn capabilities_vary_by_concrete_scope_kind() {
        let sets = ScopedCapabilitySets::new(
            CapabilitySet::new([(capability("plugin.install"), CapabilitySupport::Supported)]),
            CapabilitySet::new([(capability("plugin.install"), CapabilitySupport::Unsupported)]),
        );
        assert_eq!(
            sets.for_scope(&Scope::Global)
                .support(&capability("plugin.install")),
            Some(CapabilitySupport::Supported)
        );
        assert_eq!(
            sets.for_scope(&Scope::Project(AbsolutePath::new("/work/project").unwrap()))
                .support(&capability("plugin.install")),
            Some(CapabilitySupport::Unsupported)
        );
    }

    #[test]
    fn unknown_versions_never_expose_profile_or_mutation_authority() {
        let unknown =
            CapabilityProfileSelection::unknown_version(sets(CapabilitySupport::Supported));
        assert_eq!(unknown.authority(), ProfileAuthority::ObserveOnly);
        assert_eq!(unknown.profile_id(), None);
        assert_eq!(unknown.mutation_capabilities(), None);
        assert_eq!(
            unknown
                .observation_capabilities()
                .for_scope_kind(CapabilityScope::Global)
                .support(&capability("plugin.install")),
            Some(CapabilitySupport::Supported)
        );
        let json = serde_json::to_string(&unknown).unwrap();
        assert!(json.contains(r#""authority":"unknown_version""#));
        assert!(!json.contains("profile_id"));
        assert!(serde_json::from_str::<CapabilityProfileSelection>(
            r#"{"authority":"unknown_version","id":"codex-v3","capabilities":{"global":{"capabilities":{}},"project":{"capabilities":{}}}}"#
        )
        .is_err());
    }

    #[test]
    fn verified_observe_only_profiles_preserve_identity_and_wire_shape() {
        let profile = CapabilityProfileSelection::verified_observe_only(
            CapabilityProfileId::new("cursor-v1").unwrap(),
            sets(CapabilitySupport::Supported),
        );

        assert_eq!(profile.authority(), ProfileAuthority::ObserveOnly);
        assert_eq!(profile.profile_id().unwrap().as_str(), "cursor-v1");
        assert_eq!(
            profile
                .observation_capabilities()
                .for_scope_kind(CapabilityScope::Global)
                .support(&capability("plugin.install")),
            Some(CapabilitySupport::Supported)
        );
        assert!(profile.mutation_capabilities().is_none());

        let encoded = serde_json::to_string(&profile).unwrap();
        assert!(encoded.contains(r#""authority":"verified_observe_only""#));
        assert!(encoded.contains(r#""id":"cursor-v1""#));
        assert_eq!(
            serde_json::from_str::<CapabilityProfileSelection>(&encoded).unwrap(),
            profile
        );
    }

    #[test]
    fn verified_observe_only_narrowing_preserves_observe_only_authority() {
        let profile = CapabilityProfileSelection::verified_observe_only(
            CapabilityProfileId::new("cursor-v1").unwrap(),
            sets(CapabilitySupport::Supported),
        );
        let narrowed = profile
            .narrow(&sets(CapabilitySupport::Unsupported))
            .unwrap();

        assert_eq!(narrowed.authority(), ProfileAuthority::ObserveOnly);
        assert_eq!(narrowed.profile_id().unwrap().as_str(), "cursor-v1");
        assert!(narrowed.mutation_capabilities().is_none());
        assert_eq!(
            narrowed
                .observation_capabilities()
                .for_scope_kind(CapabilityScope::Global)
                .support(&capability("plugin.install")),
            Some(CapabilitySupport::Unsupported)
        );

        let unsupported = CapabilityProfileSelection::verified_observe_only(
            CapabilityProfileId::new("cursor-v2").unwrap(),
            sets(CapabilitySupport::Unsupported),
        );
        assert!(
            unsupported
                .narrow(&sets(CapabilitySupport::Supported))
                .is_err()
        );
    }

    #[test]
    fn probes_can_only_narrow_compiled_capabilities() {
        let profile = CapabilityProfileSelection::verified(
            CapabilityProfileId::new("codex-v3").unwrap(),
            sets(CapabilitySupport::Supported),
        );
        let narrowed = profile
            .narrow(&sets(CapabilitySupport::Unsupported))
            .unwrap();
        assert_eq!(
            narrowed
                .mutation_capabilities()
                .unwrap()
                .for_scope_kind(CapabilityScope::Global)
                .support(&capability("plugin.install")),
            Some(CapabilitySupport::Unsupported)
        );

        let unverified = CapabilityProfileSelection::verified(
            CapabilityProfileId::new("codex-v4").unwrap(),
            sets(CapabilitySupport::Unverified),
        );
        assert!(
            unverified
                .narrow(&sets(CapabilitySupport::Supported))
                .is_err()
        );
        let unsupported = CapabilityProfileSelection::verified(
            CapabilityProfileId::new("codex-v5").unwrap(),
            sets(CapabilitySupport::Unsupported),
        );
        assert!(
            unsupported
                .narrow(&sets(CapabilitySupport::Supported))
                .is_err()
        );
    }

    #[test]
    fn narrowing_rejects_capabilities_absent_from_the_compiled_profile() {
        let baseline =
            ScopedCapabilitySets::new(CapabilitySet::default(), CapabilitySet::default());
        let probe = ScopedCapabilitySets::new(
            CapabilitySet::new([(capability("plugin.install"), CapabilitySupport::Unsupported)]),
            CapabilitySet::default(),
        );
        assert!(matches!(
            baseline.narrow(&probe),
            Err(ProfileContractError::UnknownCapability {
                scope: CapabilityScope::Global,
                ..
            })
        ));
    }
}
