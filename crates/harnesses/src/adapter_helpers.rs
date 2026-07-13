use skilltap_core::{
    domain::{
        CapabilityId, CapabilityProfileId, CapabilityProfileSelection, CapabilitySet,
        CapabilitySupport, NativeVersion, ScopedCapabilitySets,
    },
    runtime::{JsonLimits, StrictJson, StrictJsonDecoder},
};

use crate::DetectionError;

pub(crate) fn decode_codex_version(
    stdout: &[u8],
    json_limits: JsonLimits,
) -> Result<NativeVersion, DetectionError> {
    decode_native_version(stdout, json_limits, |text| text.strip_prefix("codex-cli "))
}

pub(crate) fn decode_claude_version(
    stdout: &[u8],
    json_limits: JsonLimits,
) -> Result<NativeVersion, DetectionError> {
    decode_native_version(stdout, json_limits, |text| {
        text.strip_suffix(" (Claude Code)")
    })
}

fn decode_native_version<'a>(
    stdout: &'a [u8],
    json_limits: JsonLimits,
    text_version: impl FnOnce(&'a str) -> Option<&'a str>,
) -> Result<NativeVersion, DetectionError> {
    let text = std::str::from_utf8(stdout).map_err(|_| DetectionError::InvalidVersion)?;
    let text = text.strip_suffix('\n').unwrap_or(text);
    let text = text.strip_suffix('\r').unwrap_or(text);
    if text.is_empty() || text.chars().any(char::is_control) {
        return Err(DetectionError::InvalidVersion);
    }

    let version = if text.starts_with('{') {
        let decoded = StrictJson
            .decode(stdout, json_limits)
            .map_err(|_| DetectionError::InvalidVersion)?;
        decoded
            .value()
            .as_object()
            .and_then(|object| object.get("version"))
            .and_then(serde_json::Value::as_str)
            .ok_or(DetectionError::InvalidVersion)?
            .to_owned()
    } else {
        text_version(text)
            .filter(|version| is_single_version_token(version))
            .ok_or(DetectionError::InvalidVersion)?
            .to_owned()
    };

    NativeVersion::new(&version).map_err(|_| DetectionError::InvalidVersion)
}

fn is_single_version_token(version: &str) -> bool {
    !version.is_empty() && !version.chars().any(char::is_whitespace)
}

pub(crate) fn select_profile(
    version: &NativeVersion,
    verified_version: &str,
    profile_id: &str,
    capabilities: ScopedCapabilitySets,
) -> CapabilityProfileSelection {
    if version.as_str() == verified_version {
        CapabilityProfileSelection::verified(
            CapabilityProfileId::new(profile_id).expect("compiled profile identifier is valid"),
            capabilities,
        )
    } else {
        CapabilityProfileSelection::unknown_version(unknown_capabilities(&capabilities))
    }
}

pub(crate) fn compiled_capabilities(
    plugin_update: bool,
    project_lifecycle: bool,
) -> ScopedCapabilitySets {
    let support = |capability: &str, supported: bool| {
        (
            CapabilityId::new(capability).expect("compiled capability is valid"),
            if supported {
                CapabilitySupport::Supported
            } else {
                CapabilitySupport::Unverified
            },
        )
    };
    let global = CapabilitySet::new([
        support("harness.observe", true),
        support("plugin.install", true),
        support("plugin.remove", true),
        support("plugin.update", plugin_update),
        support("marketplace.register", true),
        support("marketplace.remove", true),
        support("marketplace.update", true),
    ]);
    let project = CapabilitySet::new([
        support("harness.observe", true),
        support("plugin.install", project_lifecycle),
        support("plugin.remove", project_lifecycle),
        support("plugin.update", project_lifecycle),
        support("marketplace.register", project_lifecycle),
        support("marketplace.remove", project_lifecycle),
        support("marketplace.update", project_lifecycle),
    ]);
    ScopedCapabilitySets::new(global, project)
}

fn unknown_capabilities(baseline: &ScopedCapabilitySets) -> ScopedCapabilitySets {
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

pub(crate) fn observe_codex(
    paths: &skilltap_core::runtime::PlatformPaths,
    scope: &skilltap_core::domain::Scope,
    limits: skilltap_core::runtime::ExternalTreeLimits,
) -> Result<crate::registry::AdapterObservationPaths, crate::registry::ObservationPathError> {
    let inputs = crate::codex_observation_paths(paths, scope)?;
    let canonical = crate::observe_codex_canonical_resources(&inputs, scope, limits)?;
    let project_entry_count = if matches!(scope, skilltap_core::domain::Scope::Project(_)) {
        Some(crate::observe_codex_project_resources(&inputs, limits)?)
    } else {
        None
    };
    Ok(crate::registry::AdapterObservationPaths {
        canonical,
        project_entry_count,
    })
}

pub(crate) fn observe_claude(
    paths: &skilltap_core::runtime::PlatformPaths,
    scope: &skilltap_core::domain::Scope,
    limits: skilltap_core::runtime::ExternalTreeLimits,
) -> Result<crate::registry::AdapterObservationPaths, crate::registry::ObservationPathError> {
    let inputs = crate::claude_observation_paths(paths, scope)?;
    let canonical = crate::observe_claude_canonical_resources(&inputs, scope, limits)?;
    let project_entry_count = if matches!(scope, skilltap_core::domain::Scope::Project(_)) {
        Some(crate::observe_claude_project_resources(&inputs, limits)?)
    } else {
        None
    };
    Ok(crate::registry::AdapterObservationPaths {
        canonical,
        project_entry_count,
    })
}

pub(crate) fn absolute_child(
    root: &skilltap_core::domain::AbsolutePath,
    child: &str,
) -> Option<skilltap_core::domain::AbsolutePath> {
    skilltap_core::domain::AbsolutePath::new(format!("{}/{child}", root.as_str())).ok()
}
