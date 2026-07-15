use std::ffi::OsString;

use skilltap_core::{
    domain::{
        AbsolutePath, CapabilityProfileSelection, CapabilityScope, HarnessId, NativeVersion, Scope,
    },
    runtime::{
        ExternalTreeLimits, ExternalTreeObserver, ExternalTreeRequest, JsonLimits,
        ObservationRuntimeError, PlatformPaths, SystemExternalTreeObserver,
    },
};

use crate::{
    adapter_helpers,
    effective_state::{
        EffectiveMcpStatus, EffectiveProbeError, EffectiveProbeSpec, EffectiveStateProbePort,
        ReloadSemantics, decode_json_status,
    },
    managed_projection::ManagedProjectionPort,
    registry::{
        AdapterObservationPaths, DistributionSurface, HarnessAdapter, ObservationPathError,
        SkillProjectionPort, TargetIdentity,
    },
};

use super::kiro_managed::KiroManagedProjection;

const VERIFIED_VERSION: &str = "2.12.2";
const PROFILE_ID: &str = "kiro-2-12-2";

pub struct KiroAdapter;
pub struct KiroSkillProjection;
pub struct KiroEffectiveStateProbe;

// The adapter remains private while the official effective-load contract is
// blocked; this singleton is ready for the guarded registry export.
#[allow(dead_code)]
static ADAPTER: KiroAdapter = KiroAdapter;
static SKILLS: KiroSkillProjection = KiroSkillProjection;
static PROBE: KiroEffectiveStateProbe = KiroEffectiveStateProbe;

impl KiroAdapter {
    #[allow(dead_code)]
    pub fn static_ref() -> &'static dyn HarnessAdapter {
        &ADAPTER
    }
}

impl HarnessAdapter for KiroAdapter {
    fn identity(&self) -> TargetIdentity {
        TargetIdentity {
            id: HarnessId::new("kiro").expect("static harness id is valid"),
            display_name: "Kiro CLI",
            default_binary: "kiro-cli",
            distribution_surface: DistributionSurface::Managed,
        }
    }

    fn version_arguments(&self) -> Vec<OsString> {
        vec![OsString::from("--version")]
    }

    fn decode_version(&self, stdout: &[u8]) -> Result<NativeVersion, crate::DetectionError> {
        let text =
            std::str::from_utf8(stdout).map_err(|_| crate::DetectionError::InvalidVersion)?;
        let text = text.strip_suffix('\n').unwrap_or(text);
        let text = text.strip_suffix('\r').unwrap_or(text);
        let version = text
            .strip_prefix("kiro-cli ")
            .filter(|version| !version.is_empty() && !version.chars().any(char::is_whitespace))
            .ok_or(crate::DetectionError::InvalidVersion)?;
        NativeVersion::new(version).map_err(|_| crate::DetectionError::InvalidVersion)
    }

    fn select_profile(&self, version: &NativeVersion) -> CapabilityProfileSelection {
        adapter_helpers::select_profile(
            version,
            VERIFIED_VERSION,
            PROFILE_ID,
            adapter_helpers::compiled_capabilities(true, true, true),
        )
    }

    fn observe(
        &self,
        paths: &PlatformPaths,
        scope: &Scope,
        limits: ExternalTreeLimits,
    ) -> Result<AdapterObservationPaths, ObservationPathError> {
        let (skill_root, settings_path, skill_label, settings_label) = match scope {
            Scope::Global => (
                adapter_helpers::absolute_child(paths.kiro_home(), "skills"),
                adapter_helpers::absolute_child(paths.kiro_home(), "settings/mcp.json"),
                "kiro.skills",
                "kiro.settings",
            ),
            Scope::Project(project) => (
                adapter_helpers::absolute_child(project, ".kiro/skills"),
                adapter_helpers::absolute_child(project, ".kiro/settings/mcp.json"),
                "project.kiro.skills",
                "project.kiro.settings",
            ),
        };

        let mut canonical = Vec::new();
        let mut project_entry_count = 0usize;
        if let Some(root) = skill_root {
            match SystemExternalTreeObserver.observe(&ExternalTreeRequest::new(root, limits)) {
                Ok(snapshot) => {
                    project_entry_count = snapshot.entries().len();
                    canonical.push(crate::CanonicalObservation {
                        root: skill_label.to_owned(),
                        snapshot,
                    });
                }
                Err(ObservationRuntimeError::TreeRootUnavailable) => {}
                Err(error) => return Err(ObservationPathError::Runtime(error)),
            }
        }

        let surface_labels = if settings_path.as_ref().is_some_and(path_exists) {
            vec![settings_label]
        } else {
            Vec::new()
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
        Some(KiroManagedProjection::static_ref())
    }

    fn effective_state_probe(&self) -> Option<&dyn EffectiveStateProbePort> {
        Some(&PROBE)
    }

    fn native_root(&self, paths: &PlatformPaths) -> Option<AbsolutePath> {
        Some(paths.kiro_home().clone())
    }
}

impl SkillProjectionPort for KiroSkillProjection {
    fn destination(&self, paths: &PlatformPaths, scope: &Scope) -> Option<AbsolutePath> {
        match scope {
            Scope::Global => adapter_helpers::absolute_child(paths.kiro_home(), "skills"),
            Scope::Project(project) => adapter_helpers::absolute_child(project, ".kiro/skills"),
        }
    }
}

impl EffectiveStateProbePort for KiroEffectiveStateProbe {
    fn mcp_status_spec(&self, scope: &Scope) -> EffectiveProbeSpec {
        let (scope_name, working_directory) = match scope {
            Scope::Global => ("global", None),
            Scope::Project(project) => ("workspace", Some(project.clone())),
        };
        EffectiveProbeSpec {
            arguments: vec![
                OsString::from("mcp"),
                OsString::from("list"),
                OsString::from(scope_name),
            ],
            working_directory,
        }
    }

    fn decode_mcp_status(
        &self,
        stdout: &[u8],
        limits: JsonLimits,
    ) -> Result<EffectiveMcpStatus, EffectiveProbeError> {
        // Kiro 2.12.2's public contract does not document a machine-readable
        // `mcp list` payload. Keep the shared typed decoder as a fail-closed
        // fixture seam until an authenticated official output grammar is
        // available; malformed/plain output never becomes healthy state.
        decode_json_status(stdout, limits)
    }

    fn reload_semantics(&self) -> ReloadSemantics {
        ReloadSemantics::HotReload
    }
}

fn path_exists(path: &AbsolutePath) -> bool {
    std::fs::symlink_metadata(path.as_str()).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use skilltap_core::domain::{CapabilityId, CapabilitySupport};

    fn limits() -> JsonLimits {
        JsonLimits::new(16 * 1024, 32).expect("test limits are valid")
    }

    #[test]
    fn exact_version_profile_is_authorized_and_adjacent_versions_are_unknown() {
        let adapter = KiroAdapter;
        assert_eq!(adapter.identity().default_binary, "kiro-cli");
        assert_eq!(
            adapter
                .decode_version(b"kiro-cli 2.12.2\n")
                .unwrap()
                .as_str(),
            "2.12.2"
        );
        assert_eq!(
            adapter
                .select_profile(&NativeVersion::new("2.12.2").unwrap())
                .profile_id()
                .unwrap()
                .as_str(),
            PROFILE_ID
        );
        assert!(
            adapter
                .select_profile(&NativeVersion::new("2.12.2").unwrap())
                .mutation_capabilities()
                .is_some()
        );
        for version in ["2.12.1", "2.12.3", "3.0.0", "99.0.0"] {
            let profile = adapter.select_profile(&NativeVersion::new(version).unwrap());
            assert!(profile.profile_id().is_none(), "{version} must be unknown");
            assert!(profile.mutation_capabilities().is_none());
            assert_eq!(
                profile
                    .observation_capabilities()
                    .for_scope_kind(CapabilityScope::Global)
                    .support(&CapabilityId::new("harness.observe").unwrap()),
                Some(CapabilitySupport::Unverified)
            );
        }
    }

    #[test]
    fn version_decoder_rejects_extra_output_and_unprefixed_versions() {
        let adapter = KiroAdapter;
        for output in [
            b"2.12.2\n".as_slice(),
            b"kiro-cli 2.12.2 extra\n".as_slice(),
        ] {
            assert_eq!(
                adapter.decode_version(output),
                Err(crate::DetectionError::InvalidVersion)
            );
        }
    }

    #[test]
    fn project_skill_projection_uses_kiro_relative_links() {
        let project = AbsolutePath::new("/tmp/kiro-project").unwrap();
        let native_root = AbsolutePath::new("/tmp/kiro-project/.kiro/skills").unwrap();
        let result = skilltap_core::project_skill::project_skill_projection(
            &project,
            &native_root,
            &skilltap_core::skill_compatibility::AgentSkillName::new("demo").unwrap(),
        )
        .unwrap();
        let skilltap_core::project_skill::TargetProjectSkillProjection::RelativeLink(spec) = result
        else {
            panic!("Kiro's native project root must link to the canonical tree");
        };
        assert_eq!(spec.destination.as_str(), ".kiro/skills/demo");
        assert_eq!(
            spec.target.as_path(),
            std::path::Path::new("../../.agents/skills/demo")
        );
    }

    #[test]
    fn probe_uses_explicit_scope_and_project_working_directory() {
        let project = AbsolutePath::new("/tmp/kiro-project").unwrap();
        let global = PROBE.mcp_status_spec(&Scope::Global);
        assert_eq!(
            global.arguments,
            ["mcp", "list", "global"].map(OsString::from)
        );
        assert_eq!(global.working_directory, None);
        let workspace = PROBE.mcp_status_spec(&Scope::Project(project.clone()));
        assert_eq!(
            workspace.arguments,
            ["mcp", "list", "workspace"].map(OsString::from)
        );
        assert_eq!(workspace.working_directory, Some(project));
    }

    #[test]
    fn provisional_json_probe_is_bounded_and_fail_closed() {
        let status = PROBE
            .decode_mcp_status(
                br#"{"servers":{"docs":{"status":"connected"}},"trusted":true}"#,
                limits(),
            )
            .unwrap();
        assert_eq!(status.servers.len(), 1);
        assert!(matches!(
            PROBE.decode_mcp_status(b"kiro-cli mcp list output", limits()),
            Err(EffectiveProbeError::InvalidPayload | EffectiveProbeError::Runtime(_))
        ));
    }
}
