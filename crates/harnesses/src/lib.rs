use std::{collections::BTreeMap, ffi::OsString};

use skilltap_core::{
    domain::{
        CapabilityId, CapabilityProfileId, CapabilityProfileSelection, CapabilitySet,
        CapabilitySupport, ConfiguredBinary, HarnessId, HarnessInstallation, HarnessReachability,
        NativeId, NativeVersion, ProfileContractError, Scope, ScopedCapabilitySets,
        UnreachableReason,
    },
    runtime::{
        ExecutableResolutionRequest, ExecutableResolver, JsonLimits, NativeProcessRequest,
        NativeProcessRunner, ObservationRuntimeError, ProcessLimits, StrictJson, StrictJsonDecoder,
        SystemExecutableResolver, SystemNativeProcessRunner,
    },
};

pub use skilltap_core::VERSION;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HarnessKind {
    Codex,
    Claude,
}

impl HarnessKind {
    pub const fn id(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Claude => "claude",
        }
    }
}

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
    harness: HarnessKind,
    search_path: OsString,
    process_limits: ProcessLimits,
    json_limits: JsonLimits,
) -> Result<HarnessInstallation, DetectionError> {
    let configured = ConfiguredBinary::path_lookup(
        NativeId::new(harness.id()).map_err(|_| DetectionError::InvalidVersion)?,
    )
    .map_err(|_| DetectionError::InvalidVersion)?;
    let resolved = SystemExecutableResolver
        .resolve(&ExecutableResolutionRequest::new(
            configured.clone(),
            Some(search_path),
        ))
        .map_err(DetectionError::Runtime)?;
    let output = SystemNativeProcessRunner
        .run(&NativeProcessRequest::new(
            resolved.clone(),
            [OsString::from("--version"), OsString::from("--json")],
            BTreeMap::new(),
            None,
            process_limits,
        ))
        .map_err(DetectionError::Runtime)?;
    if !output.status().success() {
        return Err(DetectionError::NonZeroExit);
    }
    let decoded = StrictJson
        .decode(output.stdout(), json_limits)
        .map_err(DetectionError::Runtime)?;
    let version = decoded
        .value()
        .get("version")
        .and_then(serde_json::Value::as_str)
        .ok_or(DetectionError::InvalidVersion)?;
    let native_version = NativeVersion::new(version).map_err(|_| DetectionError::InvalidVersion)?;
    Ok(HarnessInstallation::new(
        HarnessId::new(harness.id()).map_err(|_| DetectionError::InvalidVersion)?,
        configured,
        HarnessReachability::Reachable {
            executable: resolved,
            native_version,
        },
    ))
}

/// Represents an absent or unusable binary without probing resources.
pub fn unreachable_installation(
    harness: HarnessKind,
    reason: UnreachableReason,
) -> HarnessInstallation {
    let configured =
        ConfiguredBinary::path_lookup(NativeId::new(harness.id()).expect("static harness id"))
            .expect("static harness id is a path name");
    HarnessInstallation::new(
        HarnessId::new(harness.id()).expect("static harness id"),
        configured,
        HarnessReachability::Unreachable { reason },
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexObservationPaths {
    pub codex_home: skilltap_core::domain::AbsolutePath,
    pub global_agents: skilltap_core::domain::AbsolutePath,
    pub project_root: Option<skilltap_core::domain::AbsolutePath>,
    pub project_agents: Option<skilltap_core::domain::AbsolutePath>,
    pub project_override: Option<skilltap_core::domain::AbsolutePath>,
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

fn absolute_child(
    root: &skilltap_core::domain::AbsolutePath,
    child: &str,
) -> Option<skilltap_core::domain::AbsolutePath> {
    skilltap_core::domain::AbsolutePath::new(format!("{}/{}", root.as_str(), child)).ok()
}

/// Selects one immutable compiled profile, or an observe-only unknown-version profile.
pub fn select_profile(harness: HarnessKind, version: &NativeVersion) -> CapabilityProfileSelection {
    let capabilities = compiled_capabilities(harness);
    let known = matches!(
        (harness, version.as_str()),
        (HarnessKind::Codex, "3.0.0") | (HarnessKind::Claude, "3.0.0")
    );
    if known {
        CapabilityProfileSelection::verified(
            CapabilityProfileId::new(match harness {
                HarnessKind::Codex => "codex-v3",
                HarnessKind::Claude => "claude-v3",
            })
            .expect("compiled profile identifiers are valid"),
            capabilities,
        )
    } else {
        CapabilityProfileSelection::unknown_version(unknown_capabilities(harness))
    }
}

fn compiled_capabilities(harness: HarnessKind) -> ScopedCapabilitySets {
    let global = CapabilitySet::new([
        (
            CapabilityId::new("harness.observe").expect("compiled capability is valid"),
            CapabilitySupport::Supported,
        ),
        (
            CapabilityId::new("plugin.install").expect("compiled capability is valid"),
            CapabilitySupport::Supported,
        ),
    ]);
    let project = CapabilitySet::new([
        (
            CapabilityId::new("harness.observe").expect("compiled capability is valid"),
            CapabilitySupport::Supported,
        ),
        (
            CapabilityId::new("plugin.install").expect("compiled capability is valid"),
            if matches!(harness, HarnessKind::Codex) {
                CapabilitySupport::Unverified
            } else {
                CapabilitySupport::Supported
            },
        ),
    ]);
    ScopedCapabilitySets::new(global, project)
}

fn unknown_capabilities(harness: HarnessKind) -> ScopedCapabilitySets {
    let baseline = compiled_capabilities(harness);
    let unverified = |set: &CapabilitySet| {
        CapabilitySet::new(
            set.iter()
                .map(|(id, _)| (id.clone(), CapabilitySupport::Unverified)),
        )
    };
    ScopedCapabilitySets::new(
        unverified(baseline.for_scope_kind(skilltap_core::domain::CapabilityScope::Global)),
        unverified(baseline.for_scope_kind(skilltap_core::domain::CapabilityScope::Project)),
    )
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
