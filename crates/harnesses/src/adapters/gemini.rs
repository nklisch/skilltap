use std::{collections::BTreeMap, ffi::OsString};

use skilltap_core::{
    domain::{AbsolutePath, CapabilityProfileSelection, HarnessId, NativeId, NativeVersion, Scope},
    runtime::{
        ExternalTreeLimits, ExternalTreeObserver, ExternalTreeRequest, JsonLimits,
        ObservationRuntimeError, PlatformPaths, SystemExternalTreeObserver,
    },
};

use crate::{
    adapter_helpers,
    effective_state::{
        EffectiveMcpStatus, EffectiveProbeError, EffectiveProbeSpec, EffectiveServerHealth,
        EffectiveStateProbePort, ProjectTrustHealth, ReloadSemantics,
    },
    managed_projection::ManagedProjectionPort,
    registry::{
        AdapterObservationPaths, DistributionSurface, HarnessAdapter, ObservationPathError,
        SkillProjectionPort, TargetIdentity,
    },
};

use super::gemini_managed::GeminiManagedProjection;

const VERIFIED_VERSION: &str = "0.50.0";
const PROFILE_ID: &str = "gemini-0-50-0";

pub struct GeminiAdapter;
pub struct GeminiSkillProjection;
pub struct GeminiEffectiveStateProbe;

static ADAPTER: GeminiAdapter = GeminiAdapter;
static SKILLS: GeminiSkillProjection = GeminiSkillProjection;
static PROBE: GeminiEffectiveStateProbe = GeminiEffectiveStateProbe;

impl GeminiAdapter {
    pub fn static_ref() -> &'static dyn HarnessAdapter {
        &ADAPTER
    }
}

impl HarnessAdapter for GeminiAdapter {
    fn identity(&self) -> TargetIdentity {
        TargetIdentity {
            id: HarnessId::new("gemini").expect("static harness id is valid"),
            display_name: "Gemini CLI",
            default_binary: Some("gemini"),
            distribution_surface: DistributionSurface::Managed,
            identity_boundary: crate::TargetIdentityBoundary::Executable,
        }
    }

    fn version_arguments(&self) -> Option<Vec<OsString>> {
        Some(vec![OsString::from("--version")])
    }

    fn decode_version(&self, stdout: &[u8]) -> Result<NativeVersion, crate::DetectionError> {
        let text =
            std::str::from_utf8(stdout).map_err(|_| crate::DetectionError::InvalidVersion)?;
        let text = text.strip_suffix('\n').unwrap_or(text);
        let text = text.strip_suffix('\r').unwrap_or(text);
        if text.is_empty()
            || text.chars().any(char::is_control)
            || text.chars().any(char::is_whitespace)
        {
            return Err(crate::DetectionError::InvalidVersion);
        }
        NativeVersion::new(text).map_err(|_| crate::DetectionError::InvalidVersion)
    }

    fn select_profile(&self, version: &NativeVersion) -> CapabilityProfileSelection {
        adapter_helpers::select_profile(
            version,
            VERIFIED_VERSION,
            PROFILE_ID,
            // Gemini has no native marketplace/plugin lifecycle in this
            // adapter. These capabilities authorize the managed projection
            // and control-plane source registration, not Gemini extensions.
            adapter_helpers::compiled_capabilities(true, true, true),
        )
    }

    fn observe(
        &self,
        paths: &PlatformPaths,
        scope: &Scope,
        limits: ExternalTreeLimits,
    ) -> Result<AdapterObservationPaths, ObservationPathError> {
        let roots = match scope {
            Scope::Global => vec![
                (
                    "gemini.agents.skills",
                    adapter_helpers::absolute_child(paths.home(), ".agents/skills"),
                ),
                (
                    "gemini.skills",
                    adapter_helpers::absolute_child(paths.home(), ".gemini/skills"),
                ),
            ],
            Scope::Project(project) => vec![
                (
                    "project.agents.skills",
                    adapter_helpers::absolute_child(project, ".agents/skills"),
                ),
                (
                    "project.gemini.skills",
                    adapter_helpers::absolute_child(project, ".gemini/skills"),
                ),
            ],
        };

        let mut canonical = Vec::new();
        let mut project_entry_count = 0usize;
        for (label, root) in roots
            .into_iter()
            .filter_map(|(label, root)| root.map(|root| (label, root)))
        {
            match SystemExternalTreeObserver.observe(&ExternalTreeRequest::new(root, limits)) {
                Ok(snapshot) => {
                    project_entry_count =
                        project_entry_count.saturating_add(snapshot.entries().len());
                    canonical.push(crate::CanonicalObservation {
                        root: label.to_owned(),
                        snapshot,
                    });
                }
                Err(ObservationRuntimeError::TreeRootUnavailable) => {}
                Err(error) => return Err(ObservationPathError::Runtime(error)),
            }
        }

        let surface_labels = match scope {
            Scope::Global => {
                let settings =
                    adapter_helpers::absolute_child(paths.home(), ".gemini/settings.json");
                let native_skills = adapter_helpers::absolute_child(paths.home(), ".gemini/skills");
                let mut labels = Vec::new();
                if settings.as_ref().is_some_and(path_exists) {
                    labels.push("gemini.settings");
                }
                if native_skills.as_ref().is_some_and(path_exists) {
                    labels.push("gemini.native.skills");
                }
                labels
            }
            Scope::Project(project) => {
                let settings = adapter_helpers::absolute_child(project, ".gemini/settings.json");
                let native_skills = adapter_helpers::absolute_child(project, ".gemini/skills");
                let mut labels = Vec::new();
                if settings.as_ref().is_some_and(path_exists) {
                    labels.push("project.gemini.settings");
                }
                if native_skills.as_ref().is_some_and(path_exists) {
                    labels.push("project.gemini.skills");
                }
                labels
            }
        };

        if canonical.is_empty() && surface_labels.is_empty() {
            return Err(ObservationPathError::Runtime(
                ObservationRuntimeError::TreeRootUnavailable,
            ));
        }

        Ok(AdapterObservationPaths {
            canonical,
            project_entry_count: matches!(scope, Scope::Project(_)).then_some(project_entry_count),
            surface_labels,
        })
    }

    fn skill_projection(&self) -> Option<&dyn SkillProjectionPort> {
        Some(&SKILLS)
    }

    fn managed_projection(&self) -> Option<&dyn ManagedProjectionPort> {
        Some(GeminiManagedProjection::static_ref())
    }

    fn effective_state_probe(&self) -> Option<&dyn EffectiveStateProbePort> {
        Some(&PROBE)
    }

    fn native_root(&self, paths: &PlatformPaths) -> Option<AbsolutePath> {
        adapter_helpers::absolute_child(paths.home(), ".gemini")
    }
}

impl SkillProjectionPort for GeminiSkillProjection {
    fn destination(&self, paths: &PlatformPaths, scope: &Scope) -> Option<AbsolutePath> {
        match scope {
            Scope::Global => adapter_helpers::absolute_child(paths.home(), ".agents/skills"),
            Scope::Project(project) => adapter_helpers::absolute_child(project, ".agents/skills"),
        }
    }
}

impl EffectiveStateProbePort for GeminiEffectiveStateProbe {
    fn mcp_status_spec(&self, scope: &Scope) -> EffectiveProbeSpec {
        EffectiveProbeSpec {
            arguments: vec![OsString::from("mcp"), OsString::from("list")],
            working_directory: match scope {
                Scope::Global => None,
                Scope::Project(project) => Some(project.clone()),
            },
        }
    }

    fn decode_mcp_status(
        &self,
        output: &[u8],
        limits: JsonLimits,
    ) -> Result<EffectiveMcpStatus, EffectiveProbeError> {
        decode_gemini_mcp_status(output, limits)
    }

    fn reload_semantics(&self) -> ReloadSemantics {
        ReloadSemantics::InteractiveRequired {
            next_action: "Run `/mcp reload` in Gemini CLI, then re-run status.",
        }
    }
}

fn path_exists(path: &AbsolutePath) -> bool {
    std::fs::symlink_metadata(path.as_str()).is_ok()
}

/// Decode the exact human-readable status emitted by Gemini CLI 0.50.0.
///
/// `gemini mcp list` writes this report to stderr, despite being a successful
/// read-only command. The probe boundary calls the input `output` so the
/// adapter remains usable by a runner that intentionally combines stdout and
/// stderr while keeping all parsing target-specific and bounded.
fn decode_gemini_mcp_status(
    output: &[u8],
    limits: JsonLimits,
) -> Result<EffectiveMcpStatus, EffectiveProbeError> {
    if (output.len() as u64) > limits.bytes() {
        return Err(EffectiveProbeError::InvalidPayload);
    }
    let text = std::str::from_utf8(output).map_err(|_| EffectiveProbeError::InvalidPayload)?;
    if text.contains("No MCP servers configured.") {
        return Ok(EffectiveMcpStatus {
            servers: BTreeMap::new(),
            project_trust: None,
        });
    }
    if !text.contains("Configured MCP servers:") {
        return Err(EffectiveProbeError::InvalidPayload);
    }

    let mut servers = BTreeMap::new();
    for line in text.lines().map(str::trim) {
        let (marker, remainder) = if let Some(value) = line.strip_prefix('✓') {
            (EffectiveServerHealth::Healthy, value.trim_start())
        } else if let Some(value) = line.strip_prefix('✗') {
            (EffectiveServerHealth::Unhealthy, value.trim_start())
        } else if let Some(value) = line.strip_prefix('○').or_else(|| line.strip_prefix('◯')) {
            (EffectiveServerHealth::Disabled, value.trim_start())
        } else {
            continue;
        };
        let (name, details) = remainder
            .split_once(": ")
            .ok_or(EffectiveProbeError::InvalidPayload)?;
        let name = NativeId::new(name).map_err(|_| EffectiveProbeError::InvalidPayload)?;
        let health = match details.rsplit_once(" - ").map(|(_, value)| value) {
            Some("Connected") => marker,
            Some("Disconnected") => EffectiveServerHealth::Unhealthy,
            Some("Disabled") => EffectiveServerHealth::Disabled,
            Some(_) | None => EffectiveServerHealth::Unknown,
        };
        servers.insert(name, health);
    }
    if servers.is_empty() {
        return Err(EffectiveProbeError::InvalidPayload);
    }

    let project_trust = if text.contains("this folder is untrusted") {
        Some(ProjectTrustHealth::Untrusted)
    } else if servers
        .values()
        .any(|health| *health == EffectiveServerHealth::Healthy)
    {
        Some(ProjectTrustHealth::Trusted)
    } else {
        Some(ProjectTrustHealth::Unknown)
    };

    Ok(EffectiveMcpStatus {
        servers,
        project_trust,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> JsonLimits {
        JsonLimits::new(16 * 1024, 32).unwrap()
    }

    #[test]
    fn exact_version_profile_is_authorized_and_adjacent_version_is_unknown() {
        let adapter = GeminiAdapter;
        assert_eq!(
            adapter.decode_version(b"0.50.0\n").unwrap().as_str(),
            "0.50.0"
        );
        let known = adapter.select_profile(&NativeVersion::new("0.50.0").unwrap());
        assert_eq!(known.profile_id().unwrap().as_str(), PROFILE_ID);
        assert!(known.mutation_capabilities().is_some());

        let adjacent = adapter.select_profile(&NativeVersion::new("0.50.1").unwrap());
        assert!(adjacent.profile_id().is_none());
        assert!(adjacent.mutation_capabilities().is_none());
    }

    #[test]
    fn mcp_probe_uses_direct_argv_and_project_cwd() {
        let project = AbsolutePath::new("/tmp/gemini-project").unwrap();
        let spec = PROBE.mcp_status_spec(&Scope::Project(project.clone()));
        assert_eq!(spec.arguments, ["mcp", "list"].map(OsString::from));
        assert_eq!(spec.working_directory, Some(project));
        assert_eq!(
            PROBE.mcp_status_spec(&Scope::Global).working_directory,
            None
        );
    }

    #[test]
    fn exact_status_fixture_distinguishes_connected_disconnected_disabled_and_trust() {
        let status = decode_gemini_mcp_status(
            "Warning: MCP servers are configured but disabled because this folder is untrusted.\n\nConfigured MCP servers:\n\n✓ docs: node server.mjs (stdio) - Connected\n✗ broken: /usr/bin/true (stdio) - Disconnected\n○ disabled: /usr/bin/true (stdio) - Disabled\n"
                .as_bytes(),
            limits(),
        )
        .unwrap();
        assert_eq!(status.project_trust, Some(ProjectTrustHealth::Untrusted));
        assert_eq!(
            status.servers[&NativeId::new("docs").unwrap()],
            EffectiveServerHealth::Healthy
        );
        assert_eq!(
            status.servers[&NativeId::new("broken").unwrap()],
            EffectiveServerHealth::Unhealthy
        );
        assert_eq!(
            status.servers[&NativeId::new("disabled").unwrap()],
            EffectiveServerHealth::Disabled
        );
    }

    #[test]
    fn connected_status_is_positive_effective_evidence() {
        let status = decode_gemini_mcp_status(
            b"Configured MCP servers:\n\n\xE2\x9C\x93 docs: node server.mjs (stdio) - Connected\n",
            limits(),
        )
        .unwrap();
        assert_eq!(status.project_trust, Some(ProjectTrustHealth::Trusted));
        assert_eq!(status.servers.len(), 1);
    }

    #[test]
    fn malformed_status_never_becomes_an_empty_healthy_result() {
        assert!(matches!(
            decode_gemini_mcp_status(b"not a Gemini status", limits()),
            Err(EffectiveProbeError::InvalidPayload)
        ));
    }
}
