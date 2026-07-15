use std::ffi::OsString;

use skilltap_core::{
    domain::{AbsolutePath, CapabilityProfileSelection, HarnessId, NativeVersion, Scope},
    runtime::{
        ExternalTreeLimits, ExternalTreeObserver, ExternalTreeRequest, ObservationRuntimeError,
        PlatformPaths, SystemExternalTreeObserver,
    },
};

use crate::{
    adapter_helpers,
    conditional_profile::ConditionalProfilePort,
    registry::{
        AdapterObservationPaths, DistributionSurface, HarnessAdapter, ObservationPathError,
        SkillProjectionPort, TargetIdentity,
    },
};

use super::pi_profile::PiConditionalProfile;

pub struct PiAdapter;
pub struct PiSkillProjection;

static ADAPTER: PiAdapter = PiAdapter;
static SKILLS: PiSkillProjection = PiSkillProjection;
static PROFILE: PiConditionalProfile = PiConditionalProfile;

impl PiAdapter {
    pub fn static_ref() -> &'static dyn HarnessAdapter {
        &ADAPTER
    }
}

impl HarnessAdapter for PiAdapter {
    fn identity(&self) -> TargetIdentity {
        TargetIdentity {
            id: HarnessId::new("pi").expect("static harness id is valid"),
            display_name: "Pi",
            default_binary: Some("pi"),
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
        super::pi_profile::select_core_profile(version)
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
                    "pi.agents.skills",
                    adapter_helpers::absolute_child(paths.home(), ".agents/skills"),
                ),
                (
                    "pi.skills",
                    adapter_helpers::absolute_child(paths.pi_home(), "skills"),
                ),
            ],
            Scope::Project(project) => vec![
                (
                    "project.agents.skills",
                    adapter_helpers::absolute_child(project, ".agents/skills"),
                ),
                (
                    "project.pi.skills",
                    adapter_helpers::absolute_child(project, ".pi/skills"),
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
                let settings = adapter_helpers::absolute_child(paths.pi_home(), "settings.json");
                let global_mcp =
                    adapter_helpers::absolute_child(paths.config_home(), "mcp/mcp.json");
                let pi_mcp = adapter_helpers::absolute_child(paths.pi_home(), "mcp.json");
                let mut labels = Vec::new();
                push_if_exists(&mut labels, "pi.settings", settings.as_ref());
                push_if_exists(&mut labels, "pi.mcp.global", global_mcp.as_ref());
                push_if_exists(&mut labels, "pi.mcp.override", pi_mcp.as_ref());
                labels
            }
            Scope::Project(project) => {
                let settings = adapter_helpers::absolute_child(project, ".pi/settings.json");
                let project_mcp = adapter_helpers::absolute_child(project, ".mcp.json");
                let pi_mcp = adapter_helpers::absolute_child(project, ".pi/mcp.json");
                let mut labels = Vec::new();
                push_if_exists(&mut labels, "project.pi.settings", settings.as_ref());
                push_if_exists(&mut labels, "project.mcp", project_mcp.as_ref());
                push_if_exists(&mut labels, "project.pi.mcp", pi_mcp.as_ref());
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

    fn conditional_profile(&self) -> Option<&dyn ConditionalProfilePort> {
        Some(&PROFILE)
    }

    fn native_root(&self, paths: &PlatformPaths) -> Option<AbsolutePath> {
        Some(paths.pi_home().clone())
    }
}

impl SkillProjectionPort for PiSkillProjection {
    fn destination(&self, paths: &PlatformPaths, scope: &Scope) -> Option<AbsolutePath> {
        match scope {
            Scope::Global => adapter_helpers::absolute_child(paths.home(), ".agents/skills"),
            Scope::Project(project) => adapter_helpers::absolute_child(project, ".agents/skills"),
        }
    }
}

fn push_if_exists(
    labels: &mut Vec<&'static str>,
    label: &'static str,
    path: Option<&AbsolutePath>,
) {
    if path.is_some_and(adapter_helpers::path_exists) {
        labels.push(label);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use skilltap_core::{
        domain::{CapabilityId, CapabilityScope, CapabilitySupport},
        runtime::{Environment, EnvironmentVariable, SupportedPlatform},
    };
    use skilltap_test_support::TempRoot;
    use std::{collections::BTreeMap, fs};

    #[derive(Default)]
    struct TestEnvironment(BTreeMap<&'static str, OsString>);

    impl TestEnvironment {
        fn with(mut self, variable: EnvironmentVariable, value: &str) -> Self {
            self.0.insert(variable.as_str(), OsString::from(value));
            self
        }
    }

    impl Environment for TestEnvironment {
        fn value(&self, variable: EnvironmentVariable) -> Option<OsString> {
            self.0.get(variable.as_str()).cloned()
        }
    }

    fn paths(root: &TempRoot) -> PlatformPaths {
        PlatformPaths::resolve_for(
            SupportedPlatform::Linux,
            &TestEnvironment::default().with(
                EnvironmentVariable::Home,
                root.join("home").to_str().unwrap(),
            ),
        )
        .unwrap()
    }

    fn limits() -> ExternalTreeLimits {
        ExternalTreeLimits::new(16, 128, 4096, 32 * 1024, 4096).unwrap()
    }

    #[test]
    fn exact_version_bytes_are_strict_and_adjacent_versions_are_unknown() {
        let adapter = PiAdapter;
        assert_eq!(
            adapter.version_arguments(),
            Some(vec![OsString::from("--version")])
        );
        assert_eq!(
            adapter.decode_version(b"0.80.6\n").unwrap().as_str(),
            "0.80.6"
        );
        for output in [b"pi 0.80.6\n".as_slice(), b"0.80.6 extra\n".as_slice()] {
            assert_eq!(
                adapter.decode_version(output),
                Err(crate::DetectionError::InvalidVersion)
            );
        }
        let known = adapter.select_profile(&NativeVersion::new("0.80.6").unwrap());
        assert_eq!(known.profile_id().unwrap().as_str(), "pi-0-80-6");
        assert!(known.mutation_capabilities().is_none());
        let unknown = adapter.select_profile(&NativeVersion::new("0.80.7").unwrap());
        assert!(unknown.profile_id().is_none());
        assert!(unknown.mutation_capabilities().is_none());
        assert_eq!(
            unknown
                .observation_capabilities()
                .for_scope_kind(CapabilityScope::Global)
                .support(&CapabilityId::new("harness.observe").unwrap()),
            Some(CapabilitySupport::Supported)
        );
    }

    #[test]
    fn skill_projection_is_canonical_and_native_pi_skills_are_not_destinations() {
        let root = TempRoot::new("pi-skill-projection").unwrap();
        let paths = paths(&root);
        let project =
            skilltap_core::domain::AbsolutePath::new(root.join("project").to_str().unwrap())
                .unwrap();
        assert_eq!(
            SKILLS.destination(&paths, &Scope::Global).unwrap().as_str(),
            format!("{}/.agents/skills", root.join("home").display())
        );
        assert_eq!(
            SKILLS
                .destination(&paths, &Scope::Project(project.clone()))
                .unwrap()
                .as_str(),
            format!("{}/.agents/skills", project.as_str())
        );
        assert_ne!(
            SKILLS
                .destination(&paths, &Scope::Project(project))
                .unwrap()
                .as_str(),
            "project/.pi/skills"
        );
    }

    #[test]
    fn observation_uses_static_surface_labels_and_preserves_native_sibling_roots() {
        let root = TempRoot::new("pi-observation").unwrap();
        let paths = paths(&root);
        let project_path = root.join("project");
        fs::create_dir_all(project_path.join(".pi/skills")).unwrap();
        fs::write(project_path.join(".pi/skills/native"), b"native").unwrap();
        fs::write(project_path.join(".pi/settings.json"), b"{}").unwrap();
        fs::write(project_path.join(".mcp.json"), b"{}").unwrap();
        let project =
            skilltap_core::domain::AbsolutePath::new(project_path.to_str().unwrap()).unwrap();
        let observation = PiAdapter
            .observe(&paths, &Scope::Project(project), limits())
            .unwrap();
        assert!(
            observation
                .canonical
                .iter()
                .any(|root| root.root == "project.pi.skills")
        );
        assert_eq!(
            observation.surface_labels,
            vec!["project.pi.settings", "project.mcp"]
        );
        assert!(
            observation
                .surface_labels
                .iter()
                .all(|label| !label.contains("/"))
        );
    }

    #[test]
    fn pi_has_no_native_or_managed_write_ports() {
        let adapter = PiAdapter;
        assert!(adapter.native_lifecycle().is_none());
        assert!(adapter.managed_projection().is_none());
        assert!(adapter.instruction_bridge().is_none());
        assert!(adapter.conditional_profile().is_some());
    }
}
