use std::{collections::BTreeMap, ffi::OsString};

use skilltap_core::{
    domain::{
        CapabilityId, CapabilityProfileId, CapabilityProfileSelection, CapabilitySet,
        CapabilitySupport, ConfiguredBinary, HarnessId, HarnessInstallation, HarnessReachability,
        NativeId, NativeVersion, ScopedCapabilitySets, UnreachableReason,
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
