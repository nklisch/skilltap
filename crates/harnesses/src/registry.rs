use std::{ffi::OsString, fmt};

use skilltap_core::{
    domain::{
        AbsolutePath, CapabilityProfileSelection, CapabilityScope, HarnessId, NativeVersion, Scope,
    },
    runtime::{ExternalTreeLimits, JsonLimits, ObservationRuntimeError, PlatformPaths},
    skill::ValidatedSkillTree,
    skill_compatibility::{AgentSkillValidation, SkillCompatibility},
};

use crate::{
    CanonicalObservation, DetectionError,
    adapters::{
        ClaudeAdapter, CodexAdapter, FactoryAdapter, GeminiAdapter, OpenCodeAdapter, PiAdapter,
        QwenAdapter,
    },
    conditional_profile::ConditionalProfilePort,
    lifecycle::{
        NativeLifecycleDispatch, NativeLifecycleError, NativeLifecycleRequest,
        NativeResourceObservation,
    },
    managed_projection::ManagedProjectionPort,
    native_distribution::NativeDistributionPort,
};

/// Whether a target participates in skilltap's self-hosted first-party plugin
/// bootstrap.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DistributionSurface {
    FirstPartyPlugin,
    Managed,
}

/// Stable identity and display metadata for one registered target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TargetIdentity {
    pub id: HarnessId,
    pub display_name: &'static str,
    /// The executable name used when policy does not provide an explicit path.
    /// It is adapter metadata rather than an assumption that id and binary are
    /// interchangeable (for example, Kiro uses `kiro-cli`).
    pub default_binary: &'static str,
    pub distribution_surface: DistributionSurface,
}

/// Documented native observation roots for one concrete scope.
#[derive(Clone, Debug)]
pub struct AdapterObservationPaths {
    pub canonical: Vec<CanonicalObservation>,
    pub project_entry_count: Option<usize>,
    /// Existing native surfaces that status renders in addition to bounded
    /// tree snapshots. Labels are adapter-authored so composition never
    /// reinterprets target-specific paths.
    pub surface_labels: Vec<&'static str>,
}

#[derive(Clone, Debug)]
pub enum ObservationPathError {
    Validation(skilltap_core::domain::ValidationError),
    Runtime(ObservationRuntimeError),
}

impl fmt::Display for ObservationPathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(error) => error.fmt(formatter),
            Self::Runtime(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for ObservationPathError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Validation(error) => Some(error),
            Self::Runtime(error) => Some(error),
        }
    }
}

impl From<skilltap_core::domain::ValidationError> for ObservationPathError {
    fn from(error: skilltap_core::domain::ValidationError) -> Self {
        Self::Validation(error)
    }
}

impl From<ObservationRuntimeError> for ObservationPathError {
    fn from(error: ObservationRuntimeError) -> Self {
        Self::Runtime(error)
    }
}

/// One registered target adapter.
///
/// Required methods provide detection, capability selection, and bounded
/// observation. Optional ports expose only the native behavior a target
/// actually supports.
pub trait HarnessAdapter: Sync {
    fn identity(&self) -> TargetIdentity;

    fn version_arguments(&self) -> Vec<OsString>;
    fn decode_version(&self, stdout: &[u8]) -> Result<NativeVersion, DetectionError>;

    /// Detection callers with an explicit JSON boundary use this method. The
    /// default preserves the compact public contract for text-only adapters.
    fn decode_version_with_limits(
        &self,
        stdout: &[u8],
        _limits: JsonLimits,
    ) -> Result<NativeVersion, DetectionError> {
        self.decode_version(stdout)
    }

    fn select_profile(&self, version: &NativeVersion) -> CapabilityProfileSelection;

    fn observe(
        &self,
        paths: &PlatformPaths,
        scope: &Scope,
        limits: ExternalTreeLimits,
    ) -> Result<AdapterObservationPaths, ObservationPathError>;

    fn native_lifecycle(&self) -> Option<&dyn NativeLifecycleVector> {
        None
    }

    fn instruction_bridge(&self) -> Option<&dyn InstructionBridgePort> {
        None
    }

    fn skill_projection(&self) -> Option<&dyn SkillProjectionPort> {
        None
    }

    /// Target-specific acquisition and projection for managed fallback.
    fn managed_projection(&self) -> Option<&dyn ManagedProjectionPort> {
        None
    }

    /// Target-native source distribution assessment. This is intentionally
    /// separate from lifecycle mutation authority and receives one resolved
    /// checkout supplied by shared orchestration.
    fn native_distribution(&self) -> Option<&dyn NativeDistributionPort> {
        None
    }

    /// Bounded effective MCP status probe, separate from declared-file
    /// observation. The CLI owns executable resolution and process limits.
    fn effective_state_probe(&self) -> Option<&dyn crate::EffectiveStateProbePort> {
        None
    }

    /// Optional conditional companion/profile inspection. Existing adapters
    /// remain unchanged and have no compound-profile port.
    fn conditional_profile(&self) -> Option<&dyn ConditionalProfilePort> {
        None
    }

    /// Root shown by `harness list` for this target's native state.
    fn native_root(&self, _paths: &PlatformPaths) -> Option<AbsolutePath> {
        None
    }

    /// Whether the adapter's managed projection is mutation-authorized for the
    /// concrete scope. Adapters must opt in explicitly so an available
    /// projection port cannot accidentally grant mutation authority.
    fn supports_managed_projection(&self, _scope: CapabilityScope) -> bool {
        false
    }

    /// An agent action returned instead of unattended first-party bootstrap.
    fn bootstrap_next_action(&self) -> Option<&'static str> {
        None
    }

    /// Agent action when a first-party target is eligible for distribution but
    /// its detected profile does not authorize unattended bootstrap.
    fn bootstrap_capability_next_action(&self) -> &'static str {
        "Use the harness's documented first-party plugin flow after installing a mutation-authorized version."
    }
}

/// Native marketplace/plugin lifecycle argument vector for one request.
pub trait NativeLifecycleVector: Sync {
    fn arguments(
        &self,
        request: &NativeLifecycleRequest,
    ) -> Result<Vec<OsString>, NativeLifecycleError>;

    /// Returns the native scope evidence required in list output. `None`
    /// means this lifecycle has no independently encoded scope dimension.
    fn observation_scope(&self, scope: &Scope) -> Option<CapabilityScope>;

    /// Return the exact read-only list command for a postcondition. Native
    /// targets with human-only output override this rather than pretending a
    /// structured schema exists.
    fn observation_arguments(
        &self,
        request: &NativeLifecycleRequest,
    ) -> Result<Vec<OsString>, NativeLifecycleError> {
        Ok(crate::lifecycle::native_observation_arguments(request))
    }

    /// Decode one bounded postcondition response. The default is the existing
    /// strict JSON grammar; target adapters may supply a version-pinned human
    /// parser without weakening the shared lifecycle executor.
    fn decode_observation(
        &self,
        stdout: &[u8],
        dispatch: &NativeLifecycleDispatch,
        limits: JsonLimits,
    ) -> NativeResourceObservation {
        crate::lifecycle::decode_native_observation(stdout, dispatch, limits)
    }
}

/// Harness-native instruction bridge location for one scope.
pub trait InstructionBridgePort: Sync {
    fn global_bridge(&self, paths: &PlatformPaths) -> Option<AbsolutePath>;
    fn project_bridge(&self, project: &AbsolutePath) -> Option<AbsolutePath>;

    /// Supported legacy/project alternatives that must be observed and
    /// preserved or consolidated without hard-coding adapter paths in CLI.
    fn alternate_project_bridges(&self, _project: &AbsolutePath) -> Vec<AbsolutePath> {
        Vec::new()
    }
}

/// Where skilltap projects a standalone skill for this target.
pub trait SkillProjectionPort: Sync {
    fn destination(&self, paths: &PlatformPaths, scope: &Scope) -> Option<AbsolutePath>;

    /// Conservative default evidence for a portable skill. Adapters override
    /// this only when their attested loader semantics are stronger or narrower
    /// than the shared Agent Skills contract.
    fn compatibility(
        &self,
        target: &HarnessId,
        skill: &ValidatedSkillTree,
        validation: &AgentSkillValidation,
    ) -> SkillCompatibility {
        let _ = skill;
        SkillCompatibility::portable(target.clone(), validation)
    }
}

#[derive(Clone)]
struct RegistryEntry {
    identity: TargetIdentity,
    adapter: &'static dyn HarnessAdapter,
}

/// The authoritative typed target registry.
#[derive(Clone)]
pub struct TargetRegistry {
    entries: Vec<RegistryEntry>,
}

impl TargetRegistry {
    /// Builds a registry in stable insertion order.
    ///
    /// This constructor also provides the composition seam for adapter contract
    /// tests without requiring production adapters in the canonical registry.
    pub fn new(adapters: impl IntoIterator<Item = &'static dyn HarnessAdapter>) -> Self {
        Self {
            entries: adapters
                .into_iter()
                .map(|adapter| RegistryEntry {
                    identity: adapter.identity(),
                    adapter,
                })
                .collect(),
        }
    }

    pub fn canonical() -> Self {
        Self::new([
            CodexAdapter::static_ref(),
            ClaudeAdapter::static_ref(),
            FactoryAdapter::static_ref(),
            GeminiAdapter::static_ref(),
            QwenAdapter::static_ref(),
            OpenCodeAdapter::static_ref(),
            PiAdapter::static_ref(),
        ])
    }

    pub fn contains(&self, id: &HarnessId) -> bool {
        self.entries.iter().any(|entry| &entry.identity.id == id)
    }

    pub fn adapter(&self, id: &HarnessId) -> Option<&'static dyn HarnessAdapter> {
        self.entries
            .iter()
            .find(|entry| &entry.identity.id == id)
            .map(|entry| entry.adapter)
    }

    pub fn ids(&self) -> impl Iterator<Item = &HarnessId> {
        self.entries.iter().map(|entry| &entry.identity.id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &'static dyn HarnessAdapter> + '_ {
        self.entries.iter().map(|entry| entry.adapter)
    }

    pub fn first_party_targets(&self) -> impl Iterator<Item = &'static dyn HarnessAdapter> + '_ {
        self.entries
            .iter()
            .filter(|entry| {
                entry.identity.distribution_surface == DistributionSurface::FirstPartyPlugin
            })
            .map(|entry| entry.adapter)
    }
}

impl fmt::Debug for TargetRegistry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TargetRegistry")
            .field(
                "targets",
                &self
                    .entries
                    .iter()
                    .map(|entry| &entry.identity)
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use skilltap_core::domain::{CapabilitySet, ScopedCapabilitySets};

    struct TestAdapter {
        id: &'static str,
        display_name: &'static str,
        distribution_surface: DistributionSurface,
    }

    impl HarnessAdapter for TestAdapter {
        fn identity(&self) -> TargetIdentity {
            TargetIdentity {
                id: HarnessId::new(self.id).expect("test harness id is valid"),
                display_name: self.display_name,
                default_binary: self.id,
                distribution_surface: self.distribution_surface,
            }
        }

        fn version_arguments(&self) -> Vec<OsString> {
            vec![OsString::from("--version")]
        }

        fn decode_version(&self, stdout: &[u8]) -> Result<NativeVersion, DetectionError> {
            let version =
                std::str::from_utf8(stdout).map_err(|_| DetectionError::InvalidVersion)?;
            NativeVersion::new(version).map_err(|_| DetectionError::InvalidVersion)
        }

        fn select_profile(&self, _version: &NativeVersion) -> CapabilityProfileSelection {
            CapabilityProfileSelection::unknown_version(ScopedCapabilitySets::new(
                CapabilitySet::default(),
                CapabilitySet::default(),
            ))
        }

        fn observe(
            &self,
            _paths: &PlatformPaths,
            _scope: &Scope,
            _limits: ExternalTreeLimits,
        ) -> Result<AdapterObservationPaths, ObservationPathError> {
            Ok(AdapterObservationPaths {
                canonical: Vec::new(),
                project_entry_count: None,
                surface_labels: Vec::new(),
            })
        }
    }

    static FIRST_PARTY: TestAdapter = TestAdapter {
        id: "test-first-party",
        display_name: "Test First Party",
        distribution_surface: DistributionSurface::FirstPartyPlugin,
    };
    static MANAGED: TestAdapter = TestAdapter {
        id: "test-managed",
        display_name: "Test Managed",
        distribution_surface: DistributionSurface::Managed,
    };

    #[test]
    fn registry_dispatches_and_filters_adapters_in_insertion_order() {
        let registry = TargetRegistry::new([
            &FIRST_PARTY as &'static dyn HarnessAdapter,
            &MANAGED as &'static dyn HarnessAdapter,
        ]);
        let first_party_id = HarnessId::new("test-first-party").unwrap();
        let managed_id = HarnessId::new("test-managed").unwrap();
        let absent_id = HarnessId::new("absent").unwrap();

        assert_eq!(
            registry.ids().map(HarnessId::as_str).collect::<Vec<_>>(),
            ["test-first-party", "test-managed"]
        );
        assert!(registry.contains(&first_party_id));
        assert!(registry.contains(&managed_id));
        assert!(!registry.contains(&absent_id));
        assert_eq!(
            registry
                .adapter(&managed_id)
                .map(HarnessAdapter::identity)
                .map(|identity| identity.id),
            Some(managed_id)
        );
        assert!(registry.adapter(&absent_id).is_none());
        assert_eq!(registry.iter().count(), 2);
        assert_eq!(
            registry
                .first_party_targets()
                .map(HarnessAdapter::identity)
                .map(|identity| identity.id)
                .collect::<Vec<_>>(),
            [first_party_id]
        );
    }

    #[test]
    fn managed_projection_port_defaults_to_absent() {
        assert!(HarnessAdapter::managed_projection(&FIRST_PARTY).is_none());
    }

    #[test]
    fn canonical_registry_contains_only_current_concrete_adapters() {
        let registry = TargetRegistry::canonical();

        assert_eq!(
            registry.ids().map(HarnessId::as_str).collect::<Vec<_>>(),
            [
                "codex", "claude", "droid", "gemini", "qwen", "opencode", "pi"
            ]
        );
        assert_eq!(registry.iter().count(), 7);
        assert_eq!(registry.first_party_targets().count(), 2);
        assert!(
            registry
                .adapter(&HarnessId::new("gemini").unwrap())
                .is_some()
        );
    }

    #[test]
    fn optional_ports_default_to_absent() {
        let adapter = &FIRST_PARTY as &dyn HarnessAdapter;

        assert!(adapter.native_lifecycle().is_none());
        assert!(adapter.instruction_bridge().is_none());
        assert!(adapter.skill_projection().is_none());
        assert!(adapter.conditional_profile().is_none());
    }
}
