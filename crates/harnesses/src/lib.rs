use std::{collections::BTreeMap, ffi::OsString};

use skilltap_core::{
    domain::{
        CapabilityId, CapabilityProfileSelection, CapabilitySet, CapabilitySupport,
        ConfiguredBinary, HarnessInstallation, HarnessObservationOutcome, HarnessReachability,
        NativeId, ObservationBatch, ObservedEnvironment, ProfileContractError, Scope,
        ScopedCapabilitySets, UnreachableReason,
    },
    runtime::{
        ExecutableResolutionRequest, ExecutableResolver, ExternalTreeObserver, JsonLimits,
        NativeProcessRequest, NativeProcessRunner, ObservationRuntimeError, ProcessLimits,
        StrictJson, StrictJsonDecoder, SystemExecutableResolver, SystemExternalTreeObserver,
        SystemNativeProcessRunner,
    },
};

mod adapter_helpers;
pub mod adapters;
pub mod conditional_profile;
pub use adapters::{
    ClaudeAdapter, ClaudeInstructionBridge, ClaudeLifecycle, ClaudeSkillProjection, CodexAdapter,
    CodexInstructionBridge, CodexLifecycle, CodexManagedProjection, CodexSkillProjection,
    GeminiAdapter, GeminiEffectiveStateProbe, GeminiManagedProjection, GeminiSkillProjection,
    OpenCodeAdapter, OpenCodeEffectiveStateProbe, OpenCodeManagedProjection,
    OpenCodeSkillProjection, PiAdapter, PiConditionalProfile, PiSkillProjection,
};

mod plugin_graph;
pub use conditional_profile::{ConditionalProfileContext, ConditionalProfilePort};
pub use plugin_graph::{ClaudePluginGraphReader, CodexPluginGraphReader};
mod managed_codex_project;
pub use managed_codex_project::{ManagedCodexCatalog, ManagedCodexCatalogError};
mod effective_state;
pub use effective_state::{
    EffectiveMcpStatus, EffectiveProbeError, EffectiveProbeSpec, EffectiveServerHealth,
    EffectiveStateProbePort, ProjectTrustHealth, ReloadSemantics, decode_json_status,
};
mod managed_projection;
pub use managed_projection::{
    ManagedLifecycleKind, ManagedProjectionContext, ManagedProjectionInput, ManagedProjectionPort,
};
mod materialization;
pub use materialization::JsonMcpProjectionMapper;
mod load_verification;
pub use load_verification::EffectiveObservationVerifier;
mod update_resolution;
pub use update_resolution::{GitSourceRevisionResolver, ObservedNativeRevisionResolver};

mod lifecycle;
pub use lifecycle::{
    LifecyclePostconditionError, NativeLifecycleAction, NativeLifecycleBinding,
    NativeLifecycleDispatch, NativeLifecycleError, NativeLifecyclePort, NativeLifecycleRequest,
    NativeObservationFailure, NativeResourceObservation, native_arguments, observe_native_resource,
    run_native_lifecycle, run_native_lifecycle_bound, verify_lifecycle_postcondition,
};
pub mod registry;
pub use registry::{
    AdapterObservationPaths, DistributionSurface, HarnessAdapter, InstructionBridgePort,
    NativeLifecycleVector, ObservationPathError, SkillProjectionPort, TargetIdentity,
    TargetRegistry,
};
mod bootstrap;
pub use bootstrap::{
    HarnessBootstrapPolicy, HarnessSetupResult, SetupReason, setup_detected_plugin,
    setup_first_party_plugin,
};

pub use skilltap_core::VERSION;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DetectionError {
    Runtime(ObservationRuntimeError),
    NonZeroExit,
    InvalidVersion,
}

impl std::fmt::Display for DetectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Runtime(error) => return error.fmt(formatter),
            Self::NonZeroExit => "the harness version command failed",
            Self::InvalidVersion => "the harness returned an invalid version document",
        })
    }
}

impl std::error::Error for DetectionError {}

/// Detects one configured harness without observing resources or mutating state.
pub fn detect_installation(
    adapter: &dyn HarnessAdapter,
    search_path: OsString,
    environment: &BTreeMap<OsString, OsString>,
    process_limits: ProcessLimits,
    json_limits: JsonLimits,
) -> Result<HarnessInstallation, DetectionError> {
    let configured = ConfiguredBinary::path_lookup(
        NativeId::new(adapter.identity().default_binary)
            .map_err(|_| DetectionError::InvalidVersion)?,
    )
    .map_err(|_| DetectionError::InvalidVersion)?;
    detect_configured_installation(
        adapter,
        configured,
        Some(search_path),
        environment,
        process_limits,
        json_limits,
    )
}

/// Detects one harness using the configured binary policy. This is deliberately
/// read-only: it resolves and invokes the executable's version command but does
/// not create or update any native or skilltap-owned files.
pub fn detect_configured_installation(
    adapter: &dyn HarnessAdapter,
    configured: ConfiguredBinary,
    search_path: Option<OsString>,
    environment: &BTreeMap<OsString, OsString>,
    process_limits: ProcessLimits,
    json_limits: JsonLimits,
) -> Result<HarnessInstallation, DetectionError> {
    let resolved = SystemExecutableResolver
        .resolve(&ExecutableResolutionRequest::new(
            configured.clone(),
            search_path,
        ))
        .map_err(DetectionError::Runtime)?;
    let output = SystemNativeProcessRunner
        .run(&NativeProcessRequest::new(
            resolved.clone(),
            adapter.version_arguments(),
            environment.clone(),
            None,
            process_limits,
        ))
        .map_err(DetectionError::Runtime)?;
    if !output.status().success() {
        return Err(DetectionError::NonZeroExit);
    }
    let native_version = adapter.decode_version_with_limits(output.stdout(), json_limits)?;
    Ok(HarnessInstallation::new(
        adapter.identity().id,
        configured,
        HarnessReachability::Reachable {
            executable: resolved,
            native_version,
        },
    ))
}

/// Represents an absent or unusable binary without probing resources.
pub fn unreachable_installation(
    adapter: &dyn HarnessAdapter,
    reason: UnreachableReason,
) -> HarnessInstallation {
    let id = adapter.identity().id;
    let configured = ConfiguredBinary::path_lookup(
        NativeId::new(adapter.identity().default_binary)
            .expect("registered harness default binary is valid"),
    )
    .expect("registered harness default binary is a path name");
    HarnessInstallation::new(id, configured, HarnessReachability::Unreachable { reason })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexObservationPaths {
    pub home: skilltap_core::domain::AbsolutePath,
    pub codex_home: skilltap_core::domain::AbsolutePath,
    pub global_agents: skilltap_core::domain::AbsolutePath,
    pub project_root: Option<skilltap_core::domain::AbsolutePath>,
    pub project_agents: Option<skilltap_core::domain::AbsolutePath>,
    pub project_override: Option<skilltap_core::domain::AbsolutePath>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaudeObservationPaths {
    pub claude_home: skilltap_core::domain::AbsolutePath,
    pub global_settings: skilltap_core::domain::AbsolutePath,
    pub global_plugins: skilltap_core::domain::AbsolutePath,
    pub global_skills: skilltap_core::domain::AbsolutePath,
    pub project_root: Option<skilltap_core::domain::AbsolutePath>,
    pub project_settings: Option<skilltap_core::domain::AbsolutePath>,
}

/// One bounded snapshot rooted at a documented native location.  The root
/// label is stable and intentionally does not expose arbitrary filesystem
/// paths as resource identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalObservation {
    pub root: String,
    pub snapshot: skilltap_core::runtime::ExternalTreeSnapshot,
}

/// Derives only documented Claude user/global and one personal project inputs.
pub fn claude_observation_paths(
    paths: &skilltap_core::runtime::PlatformPaths,
    scope: &Scope,
) -> Result<ClaudeObservationPaths, skilltap_core::domain::ValidationError> {
    let claude_home = paths.claude_home().clone();
    let global_settings = absolute_child(&claude_home, "settings.json").ok_or(
        skilltap_core::domain::ValidationError::InvalidFormat {
            kind: "Claude settings path",
            expected: "a valid absolute path",
        },
    )?;
    let global_plugins = absolute_child(&claude_home, "plugins").ok_or(
        skilltap_core::domain::ValidationError::InvalidFormat {
            kind: "Claude plugins path",
            expected: "a valid absolute path",
        },
    )?;
    let global_skills = absolute_child(&claude_home, "skills").ok_or(
        skilltap_core::domain::ValidationError::InvalidFormat {
            kind: "Claude skills path",
            expected: "a valid absolute path",
        },
    )?;
    let project_root = match scope {
        Scope::Global => None,
        Scope::Project(root) => Some(root.clone()),
    };
    let project_settings = project_root
        .as_ref()
        .and_then(|root| absolute_child(root, ".claude/settings.json"));
    Ok(ClaudeObservationPaths {
        claude_home,
        global_settings,
        global_plugins,
        global_skills,
        project_root,
        project_settings,
    })
}

/// Derives only the documented Codex observation inputs for one exact scope.
pub fn codex_observation_paths(
    paths: &skilltap_core::runtime::PlatformPaths,
    scope: &Scope,
) -> Result<CodexObservationPaths, skilltap_core::domain::ValidationError> {
    let project_root = match scope {
        Scope::Global => None,
        Scope::Project(root) => Some(root.clone()),
    };
    let (project_agents, project_override) = project_root.as_ref().map_or((None, None), |root| {
        (
            absolute_child(root, "AGENTS.md"),
            absolute_child(root, "AGENTS.override.md"),
        )
    });
    Ok(CodexObservationPaths {
        home: paths.home().clone(),
        codex_home: paths.codex_home().clone(),
        global_agents: paths.global_agents().clone(),
        project_root,
        project_agents,
        project_override,
    })
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct CodexConfigObservation {
    pub marketplace_count: usize,
    pub plugin_count: usize,
    pub trust_policy_present: bool,
}

/// Observes the bounded Codex native tree without materializing or interpreting
/// resource payloads in the harness boundary.
pub fn observe_codex_resources(
    paths: &CodexObservationPaths,
    limits: skilltap_core::runtime::ExternalTreeLimits,
) -> Result<skilltap_core::runtime::ExternalTreeSnapshot, ObservationRuntimeError> {
    SystemExternalTreeObserver.observe(&skilltap_core::runtime::ExternalTreeRequest::new(
        paths.codex_home.clone(),
        limits,
    ))
}

/// Observes the bounded Claude native tree; cache contents remain evidence only.
pub fn observe_claude_resources(
    paths: &ClaudeObservationPaths,
    limits: skilltap_core::runtime::ExternalTreeLimits,
) -> Result<skilltap_core::runtime::ExternalTreeSnapshot, ObservationRuntimeError> {
    SystemExternalTreeObserver.observe(&skilltap_core::runtime::ExternalTreeRequest::new(
        paths.claude_home.clone(),
        limits,
    ))
}

/// Observes only documented project-owned native roots, never the arbitrary
/// project root. Missing optional roots are tolerated; an entirely absent
/// project native surface remains an explicit unavailable observation.
pub fn observe_codex_project_resources(
    paths: &CodexObservationPaths,
    limits: skilltap_core::runtime::ExternalTreeLimits,
) -> Result<usize, ObservationRuntimeError> {
    let project = paths
        .project_root
        .as_ref()
        .ok_or(ObservationRuntimeError::TreeRootUnavailable)?;
    observe_project_roots(
        [
            absolute_child(project, ".agents"),
            absolute_child(project, ".codex"),
        ],
        limits,
    )
}

/// Observes only Claude's documented project directory, never arbitrary
/// project content outside `.claude`.
pub fn observe_claude_project_resources(
    paths: &ClaudeObservationPaths,
    limits: skilltap_core::runtime::ExternalTreeLimits,
) -> Result<usize, ObservationRuntimeError> {
    let project = paths
        .project_root
        .as_ref()
        .ok_or(ObservationRuntimeError::TreeRootUnavailable)?;
    observe_project_roots([absolute_child(project, ".claude")], limits)
}

/// Observes only documented Codex roots. Missing optional roots are omitted;
/// callers receive an unavailable error only when none of the roots exists.
pub fn observe_codex_canonical_resources(
    paths: &CodexObservationPaths,
    scope: &Scope,
    limits: skilltap_core::runtime::ExternalTreeLimits,
) -> Result<Vec<CanonicalObservation>, ObservationRuntimeError> {
    let roots = match scope {
        Scope::Global => vec![
            (
                "agents.skills",
                absolute_child(&paths.home, ".agents/skills"),
            ),
            ("codex.skills", absolute_child(&paths.codex_home, "skills")),
            (
                "codex.plugins",
                absolute_child(&paths.codex_home, "plugins"),
            ),
        ],
        Scope::Project(project) => vec![
            ("project.agents", absolute_child(project, ".agents")),
            ("project.codex", absolute_child(project, ".codex")),
        ],
    };
    observe_named_roots(roots, limits, matches!(scope, Scope::Project(_)))
}

/// Observes only documented Claude roots. Settings are parsed separately by
/// the settings adapter; this function is limited to plugin/skill trees.
pub fn observe_claude_canonical_resources(
    paths: &ClaudeObservationPaths,
    scope: &Scope,
    limits: skilltap_core::runtime::ExternalTreeLimits,
) -> Result<Vec<CanonicalObservation>, ObservationRuntimeError> {
    let roots = match scope {
        Scope::Global => vec![
            ("claude.plugins", Some(paths.global_plugins.clone())),
            ("claude.skills", Some(paths.global_skills.clone())),
        ],
        Scope::Project(project) => vec![("project.claude", absolute_child(project, ".claude"))],
    };
    observe_named_roots(roots, limits, matches!(scope, Scope::Project(_)))
}

fn observe_named_roots(
    roots: impl IntoIterator<Item = (&'static str, Option<skilltap_core::domain::AbsolutePath>)>,
    limits: skilltap_core::runtime::ExternalTreeLimits,
    allow_empty: bool,
) -> Result<Vec<CanonicalObservation>, ObservationRuntimeError> {
    let mut observed = Vec::new();
    let mut aggregate_entries = 0_u64;
    for (name, root) in roots
        .into_iter()
        .filter_map(|(name, root)| root.map(|root| (name, root)))
    {
        match SystemExternalTreeObserver.observe(&skilltap_core::runtime::ExternalTreeRequest::new(
            root, limits,
        )) {
            Ok(snapshot) => {
                aggregate_entries = aggregate_entries
                    .checked_add(snapshot.entries().len() as u64)
                    .ok_or(ObservationRuntimeError::TreeEntryLimitExceeded)?;
                if aggregate_entries > limits.entries() {
                    return Err(ObservationRuntimeError::TreeEntryLimitExceeded);
                }
                observed.push(CanonicalObservation {
                    root: name.to_owned(),
                    snapshot,
                });
            }
            Err(ObservationRuntimeError::TreeRootUnavailable) => {}
            Err(error) => return Err(error),
        }
    }
    if observed.is_empty() && !allow_empty {
        Err(ObservationRuntimeError::TreeRootUnavailable)
    } else {
        Ok(observed)
    }
}

fn observe_project_roots(
    roots: impl IntoIterator<Item = Option<skilltap_core::domain::AbsolutePath>>,
    limits: skilltap_core::runtime::ExternalTreeLimits,
) -> Result<usize, ObservationRuntimeError> {
    let mut observed = false;
    let mut entries = 0_usize;
    for root in roots.into_iter().flatten() {
        match SystemExternalTreeObserver.observe(&skilltap_core::runtime::ExternalTreeRequest::new(
            root, limits,
        )) {
            Ok(snapshot) => {
                observed = true;
                entries = entries.saturating_add(snapshot.entries().len());
            }
            Err(ObservationRuntimeError::TreeRootUnavailable) => {}
            Err(error) => return Err(error),
        }
    }
    if observed {
        Ok(entries)
    } else {
        Err(ObservationRuntimeError::TreeRootUnavailable)
    }
}

/// Composes successful and failed harness siblings without dropping any target.
pub fn normalize_observations(
    batch: ObservationBatch,
    outcomes: impl IntoIterator<Item = HarnessObservationOutcome>,
) -> Result<ObservedEnvironment, skilltap_core::domain::ObservationContractError> {
    ObservedEnvironment::new(batch, outcomes)
}

/// Returns true only when two observations share a declared source and the
/// same resource semantics; names, URLs, and fingerprints alone never match.
pub fn conservatively_equivalent(
    left: &skilltap_core::domain::ObservedResource,
    right: &skilltap_core::domain::ObservedResource,
) -> bool {
    left.source().is_some()
        && left.source() == right.source()
        && left.kind() == right.kind()
        && left.components() == right.components()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NormalizationHealth {
    pub observed_targets: usize,
    pub failed_targets: usize,
}

/// Summarizes partial normalization without dropping failed sibling evidence.
pub fn normalization_health(environment: &ObservedEnvironment) -> NormalizationHealth {
    let mut health = NormalizationHealth {
        observed_targets: 0,
        failed_targets: 0,
    };
    for (_, outcome) in environment.iter() {
        match outcome {
            HarnessObservationOutcome::Observed { .. } => health.observed_targets += 1,
            HarnessObservationOutcome::Failed { .. } => health.failed_targets += 1,
        }
    }
    health
}

impl std::fmt::Debug for CodexConfigObservation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CodexConfigObservation")
            .field("marketplace_count", &self.marketplace_count)
            .field("plugin_count", &self.plugin_count)
            .field("trust_policy_present", &self.trust_policy_present)
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodexConfigError {
    Malformed,
    UnsupportedShape,
}

/// Parses documented Codex config evidence while preserving unknown fields by
/// counting only known tables/arrays and never returning raw native values.
pub fn observe_codex_config(input: &[u8]) -> Result<CodexConfigObservation, CodexConfigError> {
    let source = std::str::from_utf8(input).map_err(|_| CodexConfigError::Malformed)?;
    let value = toml::from_str::<toml::Value>(source).map_err(|_| CodexConfigError::Malformed)?;
    let table = value.as_table().ok_or(CodexConfigError::UnsupportedShape)?;
    let marketplace_count = table.get("marketplaces").map_or(0, |value| {
        value
            .as_table()
            .map_or_else(|| value.as_array().map_or(0, Vec::len), toml::map::Map::len)
    });
    let plugin_count = table
        .get("plugins")
        .and_then(toml::Value::as_array)
        .map_or(0, Vec::len);
    let trust_policy_present = table.get("trust").is_some();
    Ok(CodexConfigObservation {
        marketplace_count,
        plugin_count,
        trust_policy_present,
    })
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct ClaudeSettingsObservation {
    pub enabled_plugin_count: usize,
    pub qualified_plugin_count: usize,
    pub trust_policy_present: bool,
    pub shared_project: bool,
}

impl std::fmt::Debug for ClaudeSettingsObservation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ClaudeSettingsObservation")
            .field("enabled_plugin_count", &self.enabled_plugin_count)
            .field("qualified_plugin_count", &self.qualified_plugin_count)
            .field("trust_policy_present", &self.trust_policy_present)
            .field("shared_project", &self.shared_project)
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClaudeSettingsError {
    Runtime(ObservationRuntimeError),
    InvalidShape,
}

/// Parses bounded Claude settings without returning native names or values.
pub fn observe_claude_settings(
    input: &[u8],
    limits: JsonLimits,
) -> Result<ClaudeSettingsObservation, ClaudeSettingsError> {
    let decoded = StrictJson
        .decode(input, limits)
        .map_err(ClaudeSettingsError::Runtime)?;
    let object = decoded
        .value()
        .as_object()
        .ok_or(ClaudeSettingsError::InvalidShape)?;
    let enabled = object
        .get("enabledPlugins")
        .and_then(serde_json::Value::as_array)
        .ok_or(ClaudeSettingsError::InvalidShape)?;
    let qualified_plugin_count = enabled
        .iter()
        .filter(|value| value.as_str().is_some_and(|name| name.contains('@')))
        .count();
    Ok(ClaudeSettingsObservation {
        enabled_plugin_count: enabled.len(),
        qualified_plugin_count,
        trust_policy_present: object.contains_key("trust"),
        shared_project: object
            .get("sharedProject")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
    })
}

fn absolute_child(
    root: &skilltap_core::domain::AbsolutePath,
    child: &str,
) -> Option<skilltap_core::domain::AbsolutePath> {
    skilltap_core::domain::AbsolutePath::new(format!("{}/{}", root.as_str(), child)).ok()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProbeError {
    Runtime(ObservationRuntimeError),
    NonZeroExit,
    InvalidPayload,
    Contract(ProfileContractError),
}

impl std::fmt::Display for ProbeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Runtime(error) => error.fmt(formatter),
            Self::NonZeroExit => formatter.write_str("the harness probe failed"),
            Self::InvalidPayload => formatter.write_str("the harness probe payload is invalid"),
            Self::Contract(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for ProbeError {}

/// Runs a bounded JSON probe and monotonically narrows a compiled profile.
pub fn probe_profile(
    profile: &CapabilityProfileSelection,
    request: &NativeProcessRequest,
    json_limits: JsonLimits,
) -> Result<CapabilityProfileSelection, ProbeError> {
    let output = SystemNativeProcessRunner
        .run(request)
        .map_err(ProbeError::Runtime)?;
    if !output.status().success() {
        return Err(ProbeError::NonZeroExit);
    }
    let decoded = StrictJson
        .decode(output.stdout(), json_limits)
        .map_err(ProbeError::Runtime)?;
    narrow_profile(profile, decoded.value())
}

/// Applies one strict probe payload to a profile without granting new authority.
pub fn narrow_profile(
    profile: &CapabilityProfileSelection,
    payload: &serde_json::Value,
) -> Result<CapabilityProfileSelection, ProbeError> {
    let scope = payload
        .get("scope")
        .and_then(serde_json::Value::as_str)
        .ok_or(ProbeError::InvalidPayload)?;
    let capabilities = payload
        .get("capabilities")
        .and_then(serde_json::Value::as_object)
        .ok_or(ProbeError::InvalidPayload)?;
    let parsed = capabilities
        .iter()
        .map(|(id, support)| {
            let id = CapabilityId::new(id).map_err(|_| ProbeError::InvalidPayload)?;
            let support = match support.as_str() {
                Some("supported") => CapabilitySupport::Supported,
                Some("unsupported") => CapabilitySupport::Unsupported,
                Some("unverified") => CapabilitySupport::Unverified,
                _ => return Err(ProbeError::InvalidPayload),
            };
            Ok((id, support))
        })
        .collect::<Result<Vec<_>, ProbeError>>()?;
    let (global, project) = match scope {
        "global" => (CapabilitySet::new(parsed), CapabilitySet::default()),
        "project" => (CapabilitySet::default(), CapabilitySet::new(parsed)),
        _ => return Err(ProbeError::InvalidPayload),
    };
    profile
        .narrow(&ScopedCapabilitySets::new(global, project))
        .map_err(ProbeError::Contract)
}
