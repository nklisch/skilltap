use std::{ffi::OsString, fmt};

use skilltap_core::{
    domain::{AbsolutePath, NativeId, Scope},
    runtime::{JsonLimits, ObservationRuntimeError, StrictJson, StrictJsonDecoder},
};

/// Whether a native status probe can establish effective MCP state after a
/// projection. Interactive reload actions are deliberately not automated.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReloadSemantics {
    HotReload,
    StatusRefresh,
    InteractiveRequired { next_action: &'static str },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectiveProbeSpec {
    pub arguments: Vec<OsString>,
    pub working_directory: Option<AbsolutePath>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EffectiveServerHealth {
    Healthy,
    Unhealthy,
    Disabled,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectTrustHealth {
    Trusted,
    Untrusted,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectiveMcpStatus {
    pub servers: std::collections::BTreeMap<NativeId, EffectiveServerHealth>,
    pub project_trust: Option<ProjectTrustHealth>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EffectiveProbeError {
    Runtime(ObservationRuntimeError),
    NonZeroExit,
    InvalidPayload,
    UnsupportedVersion,
}

impl fmt::Display for EffectiveProbeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Runtime(error) => return error.fmt(formatter),
            Self::NonZeroExit => "the effective MCP status probe failed",
            Self::InvalidPayload => "the effective MCP status payload is invalid",
            Self::UnsupportedVersion => {
                "the effective MCP status format is not verified for this version"
            }
        })
    }
}

impl std::error::Error for EffectiveProbeError {}

/// A bounded, version-owned effective-state boundary. The adapter supplies
/// only a direct argument vector and a typed decoder; process execution stays
/// at the CLI/runtime composition boundary.
pub trait EffectiveStateProbePort: Sync {
    fn mcp_status_spec(&self, scope: &Scope) -> EffectiveProbeSpec;

    fn decode_mcp_status(
        &self,
        stdout: &[u8],
        limits: JsonLimits,
    ) -> Result<EffectiveMcpStatus, EffectiveProbeError>;

    fn reload_semantics(&self) -> ReloadSemantics;
}

/// Decode the small JSON status shape shared by fixture probes. Native
/// adapters may wrap this with their exact-version parser; this helper never
/// returns raw payload values or treats a malformed payload as an empty list.
pub fn decode_json_status(
    stdout: &[u8],
    limits: JsonLimits,
) -> Result<EffectiveMcpStatus, EffectiveProbeError> {
    let decoded = StrictJson
        .decode(stdout, limits)
        .map_err(EffectiveProbeError::Runtime)?;
    let object = decoded
        .value()
        .as_object()
        .ok_or(EffectiveProbeError::InvalidPayload)?;
    let server_object = object
        .get("servers")
        .or_else(|| object.get("mcpServers"))
        .and_then(serde_json::Value::as_object)
        .ok_or(EffectiveProbeError::InvalidPayload)?;
    let mut servers = std::collections::BTreeMap::new();
    for (name, value) in server_object {
        let id = NativeId::new(name).map_err(|_| EffectiveProbeError::InvalidPayload)?;
        let health = if value.get("enabled").and_then(serde_json::Value::as_bool) == Some(false) {
            EffectiveServerHealth::Disabled
        } else {
            match value.get("status").and_then(serde_json::Value::as_str) {
                Some("healthy" | "connected" | "ready") => EffectiveServerHealth::Healthy,
                Some("disabled") => EffectiveServerHealth::Disabled,
                Some("unhealthy" | "failed" | "error") => EffectiveServerHealth::Unhealthy,
                Some(_) | None => EffectiveServerHealth::Unknown,
            }
        };
        servers.insert(id, health);
    }
    let project_trust = object
        .get("trusted")
        .and_then(serde_json::Value::as_bool)
        .map(|trusted| {
            if trusted {
                ProjectTrustHealth::Trusted
            } else {
                ProjectTrustHealth::Untrusted
            }
        })
        .or_else(|| {
            object
                .get("projectTrust")
                .and_then(serde_json::Value::as_str)
                .map(|value| match value {
                    "trusted" => ProjectTrustHealth::Trusted,
                    "untrusted" => ProjectTrustHealth::Untrusted,
                    _ => ProjectTrustHealth::Unknown,
                })
        });
    Ok(EffectiveMcpStatus {
        servers,
        project_trust,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> JsonLimits {
        JsonLimits::new(4096, 16).unwrap()
    }

    #[test]
    fn malformed_or_missing_server_object_is_unverified_not_empty() {
        for payload in [
            br"{}".as_slice(),
            br"[]".as_slice(),
            br"not-json".as_slice(),
        ] {
            assert!(matches!(
                decode_json_status(payload, limits()),
                Err(EffectiveProbeError::InvalidPayload | EffectiveProbeError::Runtime(_))
            ));
        }
    }

    #[test]
    fn bounded_status_decoder_returns_typed_health_and_trust() {
        let status = decode_json_status(
            br#"{"servers":{"docs":{"status":"connected"},"off":{"enabled":false}},"trusted":false}"#,
            limits(),
        )
        .unwrap();
        assert_eq!(status.servers.len(), 2);
        assert_eq!(
            status.servers[&NativeId::new("docs").unwrap()],
            EffectiveServerHealth::Healthy
        );
        assert_eq!(status.project_trust, Some(ProjectTrustHealth::Untrusted));
    }
}
