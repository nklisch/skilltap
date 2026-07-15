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
        EffectiveStateProbePort, ReloadSemantics,
    },
    managed_projection::ManagedProjectionPort,
    registry::{
        AdapterObservationPaths, DistributionSurface, HarnessAdapter, ObservationPathError,
        SkillProjectionPort, TargetIdentity,
    },
};

use super::opencode_managed::OpenCodeManagedProjection;

const VERIFIED_VERSION: &str = "1.18.1";
const PROFILE_ID: &str = "opencode-1-18-1";

pub struct OpenCodeAdapter;
pub struct OpenCodeSkillProjection;
pub struct OpenCodeEffectiveStateProbe;

static ADAPTER: OpenCodeAdapter = OpenCodeAdapter;
static SKILLS: OpenCodeSkillProjection = OpenCodeSkillProjection;
static PROBE: OpenCodeEffectiveStateProbe = OpenCodeEffectiveStateProbe;

impl OpenCodeAdapter {
    pub fn static_ref() -> &'static dyn HarnessAdapter {
        &ADAPTER
    }
}

impl HarnessAdapter for OpenCodeAdapter {
    fn identity(&self) -> TargetIdentity {
        TargetIdentity {
            id: HarnessId::new("opencode").expect("static harness id is valid"),
            display_name: "OpenCode",
            default_binary: Some("opencode"),
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
            || text.contains(char::is_whitespace)
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
            // OpenCode has no complete native marketplace/plugin lifecycle.
            // These capabilities authorize only skilltap-owned, file-managed
            // projection and control-plane source registration.
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
                    "opencode.agents.skills",
                    adapter_helpers::absolute_child(paths.home(), ".agents/skills"),
                ),
                (
                    "opencode.skills",
                    adapter_helpers::absolute_child(paths.config_home(), "opencode/skills"),
                ),
                (
                    "opencode.claude.skills",
                    adapter_helpers::absolute_child(paths.home(), ".claude/skills"),
                ),
            ],
            Scope::Project(project) => vec![
                (
                    "project.opencode.agents.skills",
                    adapter_helpers::absolute_child(project, ".agents/skills"),
                ),
                (
                    "project.opencode.skills",
                    adapter_helpers::absolute_child(project, ".opencode/skills"),
                ),
                (
                    "project.opencode.claude.skills",
                    adapter_helpers::absolute_child(project, ".claude/skills"),
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
                let config =
                    adapter_helpers::absolute_child(paths.config_home(), "opencode/opencode.json");
                let mut labels = Vec::new();
                if config.as_ref().is_some_and(path_exists) {
                    labels.push("opencode.config");
                }
                if adapter_helpers::absolute_child(paths.config_home(), "opencode/plugins")
                    .as_ref()
                    .is_some_and(path_exists)
                {
                    labels.push("opencode.plugins");
                }
                labels
            }
            Scope::Project(project) => {
                let config = adapter_helpers::absolute_child(project, "opencode.json");
                let mut labels = Vec::new();
                if config.as_ref().is_some_and(path_exists) {
                    labels.push("project.opencode.config");
                }
                if adapter_helpers::absolute_child(project, ".opencode/plugins")
                    .as_ref()
                    .is_some_and(path_exists)
                {
                    labels.push("project.opencode.plugins");
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
        Some(OpenCodeManagedProjection::static_ref())
    }

    fn effective_state_probe(&self) -> Option<&dyn EffectiveStateProbePort> {
        Some(&PROBE)
    }

    fn native_root(&self, paths: &PlatformPaths) -> Option<AbsolutePath> {
        adapter_helpers::absolute_child(paths.config_home(), "opencode")
    }
}

impl SkillProjectionPort for OpenCodeSkillProjection {
    fn destination(&self, paths: &PlatformPaths, scope: &Scope) -> Option<AbsolutePath> {
        match scope {
            Scope::Global => adapter_helpers::absolute_child(paths.home(), ".agents/skills"),
            Scope::Project(project) => adapter_helpers::absolute_child(project, ".agents/skills"),
        }
    }
}

impl EffectiveStateProbePort for OpenCodeEffectiveStateProbe {
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
        decode_opencode_mcp_status(output, limits)
    }

    fn reload_semantics(&self) -> ReloadSemantics {
        ReloadSemantics::StatusRefresh
    }
}

fn path_exists(path: &AbsolutePath) -> bool {
    std::fs::symlink_metadata(path.as_str()).is_ok()
}

/// Decode the exact human-readable `opencode mcp list` grammar attested for
/// OpenCode 1.18.1. The process boundary supplies bounded stdout; ANSI
/// decoration is presentation noise, while the table structure and count are
/// part of the version-pinned contract.
fn decode_opencode_mcp_status(
    output: &[u8],
    limits: JsonLimits,
) -> Result<EffectiveMcpStatus, EffectiveProbeError> {
    if (output.len() as u64) > limits.bytes() {
        return Err(EffectiveProbeError::InvalidPayload);
    }
    let text = std::str::from_utf8(output).map_err(|_| EffectiveProbeError::InvalidPayload)?;
    let text = strip_ansi(text);
    if !text.contains("MCP Servers") {
        return Err(EffectiveProbeError::InvalidPayload);
    }
    if text.contains("No MCP servers configured") {
        return Ok(EffectiveMcpStatus {
            servers: BTreeMap::new(),
            project_trust: None,
        });
    }

    let mut servers = BTreeMap::new();
    for line in text.lines().map(str::trim) {
        let Some(mut remainder) = line.strip_prefix("●  ") else {
            continue;
        };
        let marker = remainder
            .chars()
            .next()
            .ok_or(EffectiveProbeError::InvalidPayload)?;
        if !matches!(marker, '○' | '✓' | '✗') {
            return Err(EffectiveProbeError::InvalidPayload);
        }
        remainder = remainder[marker.len_utf8()..].trim_start();
        let mut words = remainder.split_whitespace();
        let name = words.next().ok_or(EffectiveProbeError::InvalidPayload)?;
        let status = words.next().ok_or(EffectiveProbeError::InvalidPayload)?;
        if words.next().is_some() {
            return Err(EffectiveProbeError::InvalidPayload);
        }
        let name = NativeId::new(name).map_err(|_| EffectiveProbeError::InvalidPayload)?;
        let health = match (marker, status) {
            ('○', "disabled") => EffectiveServerHealth::Disabled,
            ('✓', "connected") => EffectiveServerHealth::Healthy,
            ('✗', "failed" | "error") => EffectiveServerHealth::Unhealthy,
            ('✗', "needs-auth" | "unauthorized") => EffectiveServerHealth::Unknown,
            _ => return Err(EffectiveProbeError::InvalidPayload),
        };
        if servers.insert(name, health).is_some() {
            return Err(EffectiveProbeError::InvalidPayload);
        }
    }

    let count = text
        .lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix("└  "))
        .and_then(|line| line.strip_suffix(" server(s)"))
        .and_then(|value| value.parse::<usize>().ok())
        .ok_or(EffectiveProbeError::InvalidPayload)?;
    if count != servers.len() {
        return Err(EffectiveProbeError::InvalidPayload);
    }

    Ok(EffectiveMcpStatus {
        servers,
        project_trust: None,
    })
}

fn strip_ansi(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();
    while let Some(character) = chars.next() {
        if character != '\u{1b}' {
            output.push(character);
            continue;
        }
        if chars.next() != Some('[') {
            continue;
        }
        for character in chars.by_ref() {
            if character.is_ascii_alphabetic() {
                break;
            }
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> JsonLimits {
        JsonLimits::new(16 * 1024, 32).unwrap()
    }

    #[test]
    fn exact_profile_accepts_only_the_validated_version() {
        let adapter = OpenCodeAdapter;
        assert_eq!(
            adapter.decode_version(b"1.18.1\n").unwrap().as_str(),
            "1.18.1"
        );
        assert_eq!(
            adapter
                .select_profile(&NativeVersion::new("1.18.1").unwrap())
                .profile_id()
                .unwrap()
                .as_str(),
            PROFILE_ID
        );
        assert!(
            adapter
                .select_profile(&NativeVersion::new("1.18.1").unwrap())
                .mutation_capabilities()
                .is_some()
        );
        for version in ["1.18.0", "1.18.2", "2.0.0", "99.0.0"] {
            assert!(
                adapter
                    .select_profile(&NativeVersion::new(version).unwrap())
                    .mutation_capabilities()
                    .is_none(),
                "{version} must remain observe-only"
            );
        }
    }

    #[test]
    fn version_decoder_rejects_extra_output() {
        let adapter = OpenCodeAdapter;
        for output in [b"1.18.1\nextra\n".as_slice(), b"1.18.1  \n".as_slice()] {
            assert_eq!(
                adapter.decode_version(output),
                Err(crate::DetectionError::InvalidVersion)
            );
        }
    }

    #[test]
    fn probe_uses_direct_argv_and_project_working_directory() {
        let project = AbsolutePath::new("/tmp/opencode-project").unwrap();
        let spec = PROBE.mcp_status_spec(&Scope::Project(project.clone()));
        assert_eq!(spec.arguments, ["mcp", "list"].map(OsString::from));
        assert_eq!(spec.working_directory, Some(project));
        assert_eq!(
            PROBE.mcp_status_spec(&Scope::Global).working_directory,
            None
        );
    }

    #[test]
    fn exact_list_fixture_decodes_ansi_status_and_count() {
        let status = decode_opencode_mcp_status(
            b"\x1b[0m\n\xE2\x94\x8C  MCP Servers\n\xE2\x94\x82\n\xE2\x97\x8F  \xE2\x97\x8B docs \x1b[90mdisabled\n\xE2\x94\x82      /bin/true --global\n\xE2\x97\x8F  \xE2\x9C\x93 ready \x1b[90mconnected\n\xE2\x94\x82      https://example.invalid/mcp\n\xE2\x94\x94  2 server(s)\n",
            limits(),
        )
        .unwrap();
        assert_eq!(status.servers.len(), 2);
        assert_eq!(
            status.servers[&NativeId::new("docs").unwrap()],
            EffectiveServerHealth::Disabled
        );
        assert_eq!(
            status.servers[&NativeId::new("ready").unwrap()],
            EffectiveServerHealth::Healthy
        );
    }

    #[test]
    fn empty_list_is_a_valid_empty_effective_state() {
        let status = decode_opencode_mcp_status(
            b"\x1b[0m\n\xE2\x94\x8C  MCP Servers\n\xE2\x94\x82\n\xE2\x96\xb2  No MCP servers configured\n\xE2\x94\x82\n\xE2\x94\x94  Add servers with: opencode mcp add\n",
            limits(),
        )
        .unwrap();
        assert!(status.servers.is_empty());
    }

    #[test]
    fn malformed_list_never_becomes_healthy_empty_state() {
        for output in [
            b"not an OpenCode status".as_slice(),
            b"MCP Servers\n\xE2\x94\x94  2 server(s)\n".as_slice(),
        ] {
            assert!(matches!(
                decode_opencode_mcp_status(output, limits()),
                Err(EffectiveProbeError::InvalidPayload)
            ));
        }
    }
}
