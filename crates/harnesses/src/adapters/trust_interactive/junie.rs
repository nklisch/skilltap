use std::ffi::OsString;

use skilltap_core::{
    domain::{
        AbsolutePath, CapabilityId, CapabilityProfileSelection, CapabilitySet, CapabilitySupport,
        HarnessId, NativeVersion, Scope, ScopedCapabilitySets,
    },
    mutation_authority::{ManagedDeclarationContract, ManagedSurfaceKind},
    runtime::{
        ExternalTreeLimits, ExternalTreeObserver, ExternalTreeRequest, ObservationRuntimeError,
        PlatformPaths, SystemExternalTreeObserver,
    },
};

use crate::{
    adapter_helpers,
    managed_projection::ManagedProjectionPort,
    registry::{
        AdapterObservationPaths, DistributionSurface, HarnessAdapter, ObservationPathError,
        SkillProjectionPort, TargetIdentity,
    },
};

use super::{contracts::JUNIE, junie_projection::JunieManagedProjection};

static ADAPTER: JunieAdapter = JunieAdapter;
static SKILLS: JunieSkillProjection = JunieSkillProjection;
static DECLARATION_CONTRACT: std::sync::LazyLock<ManagedDeclarationContract> =
    std::sync::LazyLock::new(|| {
        ManagedDeclarationContract::new([
            ManagedSurfaceKind::ManagedDocument,
            ManagedSurfaceKind::CompleteSkillTree,
        ])
        .expect("Junie declaration contract is non-empty")
    });

pub struct JunieAdapter;
pub struct JunieSkillProjection;

impl JunieAdapter {
    pub fn static_ref() -> &'static dyn HarnessAdapter {
        &ADAPTER
    }
}

fn capabilities() -> ScopedCapabilitySets {
    let capability = |id: &str| {
        (
            CapabilityId::new(id).expect("Junie capability is valid"),
            CapabilitySupport::Unverified,
        )
    };
    let make = || {
        CapabilitySet::new([
            (
                CapabilityId::new("harness.observe").expect("Junie capability is valid"),
                CapabilitySupport::Supported,
            ),
            capability("managed.projection"),
            capability("component.skill"),
            capability("component.mcp"),
            capability("skill.install"),
            capability("skill.update"),
            capability("skill.remove"),
        ])
    };
    ScopedCapabilitySets::new(make(), make())
}

impl HarnessAdapter for JunieAdapter {
    fn identity(&self) -> TargetIdentity {
        TargetIdentity {
            id: HarnessId::new("junie").expect("static harness id is valid"),
            display_name: "Junie",
            default_binary: JUNIE.default_binary,
            distribution_surface: DistributionSurface::Managed,
        }
    }

    fn version_arguments(&self) -> Vec<OsString> {
        JUNIE.version_arguments.iter().map(OsString::from).collect()
    }

    fn decode_version(&self, stdout: &[u8]) -> Result<NativeVersion, crate::DetectionError> {
        let text = std::str::from_utf8(stdout)
            .map_err(|_| crate::DetectionError::InvalidVersion)?
            .strip_suffix('\n')
            .ok_or(crate::DetectionError::InvalidVersion)?;
        if text.contains('\r') || text.chars().any(char::is_control) {
            return Err(crate::DetectionError::InvalidVersion);
        }
        let value = text
            .strip_prefix("Junie version: ")
            .and_then(|value| value.strip_suffix(')'))
            .and_then(|value| value.split_once(" ("))
            .filter(|(marketing, build)| {
                !marketing.is_empty()
                    && !build.is_empty()
                    && !marketing.chars().any(char::is_whitespace)
                    && !build.chars().any(char::is_whitespace)
            })
            .map(|(marketing, build)| format!("{marketing}+{build}"))
            .ok_or(crate::DetectionError::InvalidVersion)?;
        NativeVersion::new(value).map_err(|_| crate::DetectionError::InvalidVersion)
    }

    fn select_profile(&self, version: &NativeVersion) -> CapabilityProfileSelection {
        adapter_helpers::select_profile(
            version,
            JUNIE.verified_version,
            JUNIE.profile_id,
            capabilities(),
        )
    }

    fn observe(
        &self,
        paths: &PlatformPaths,
        scope: &Scope,
        limits: ExternalTreeLimits,
    ) -> Result<AdapterObservationPaths, ObservationPathError> {
        let (skill_root, mcp_document, skill_label, mcp_label) = match scope {
            Scope::Global => (
                adapter_helpers::absolute_child(&junie_home(paths), "skills")
                    .ok_or_else(|| ObservationPathError::Validation(invalid_path_error()))?,
                adapter_helpers::absolute_child(&junie_home(paths), "mcp/mcp.json")
                    .ok_or_else(|| ObservationPathError::Validation(invalid_path_error()))?,
                "junie.skills",
                "junie.mcp",
            ),
            Scope::Project(project) => (
                adapter_helpers::absolute_child(project, ".junie/skills")
                    .ok_or_else(|| ObservationPathError::Validation(invalid_path_error()))?,
                adapter_helpers::absolute_child(project, ".junie/mcp/mcp.json")
                    .ok_or_else(|| ObservationPathError::Validation(invalid_path_error()))?,
                "project.junie.skills",
                "project.junie.mcp",
            ),
        };
        let mut canonical = Vec::new();
        let mut project_entry_count = None;
        match SystemExternalTreeObserver.observe(&ExternalTreeRequest::new(skill_root, limits)) {
            Ok(snapshot) => {
                let count = snapshot.entries().len();
                if matches!(scope, Scope::Project(_)) {
                    project_entry_count = Some(count);
                }
                canonical.push(crate::CanonicalObservation {
                    root: skill_label.to_owned(),
                    snapshot,
                });
            }
            Err(ObservationRuntimeError::TreeRootUnavailable) => {}
            Err(error) => return Err(ObservationPathError::Runtime(error)),
        }
        let surface_labels = if path_exists(&mcp_document) {
            vec![mcp_label]
        } else {
            Vec::new()
        };
        Ok(AdapterObservationPaths {
            canonical,
            project_entry_count,
            surface_labels,
        })
    }

    fn skill_projection(&self) -> Option<&dyn SkillProjectionPort> {
        Some(&SKILLS)
    }

    fn managed_projection(&self) -> Option<&dyn ManagedProjectionPort> {
        Some(JunieManagedProjection::static_ref())
    }

    fn managed_declaration_contract(
        &self,
        _scope: skilltap_core::domain::CapabilityScope,
    ) -> Option<&'static ManagedDeclarationContract> {
        Some(&DECLARATION_CONTRACT)
    }

    fn native_root(&self, paths: &PlatformPaths) -> Option<AbsolutePath> {
        Some(junie_home(paths))
    }
}

impl SkillProjectionPort for JunieSkillProjection {
    fn destination(&self, paths: &PlatformPaths, scope: &Scope) -> Option<AbsolutePath> {
        match scope {
            Scope::Global => adapter_helpers::absolute_child(&junie_home(paths), "skills"),
            Scope::Project(project) => adapter_helpers::absolute_child(project, ".junie/skills"),
        }
    }
}

pub(super) fn junie_home(paths: &PlatformPaths) -> AbsolutePath {
    adapter_helpers::absolute_child(paths.home(), ".junie")
        .expect("Junie home path is derived from the validated home")
}

fn path_exists(path: &AbsolutePath) -> bool {
    std::fs::symlink_metadata(path.as_str()).is_ok()
}

fn invalid_path_error() -> skilltap_core::domain::ValidationError {
    skilltap_core::domain::ValidationError::InvalidFormat {
        kind: "Junie observation path",
        expected: "a validated absolute path",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use skilltap_core::domain::{CapabilityScope, CapabilitySupport};

    #[test]
    fn exact_junie_identity_requires_marketing_and_build() {
        let adapter = JunieAdapter;
        assert_eq!(
            adapter
                .decode_version(b"Junie version: 26.6.29 (2144.10)\n")
                .unwrap()
                .as_str(),
            JUNIE.verified_version
        );
        for output in [
            b"Junie version: 26.6.29\n".as_slice(),
            b"Junie version: 26.6.29 (2144.10)\nextra\n".as_slice(),
            b"Junie version: 26.6.29 (2144.10)\r\n".as_slice(),
        ] {
            assert_eq!(
                adapter.decode_version(output),
                Err(crate::DetectionError::InvalidVersion)
            );
        }
        assert_eq!(
            adapter
                .decode_version(b"Junie version: 26.6.29 (2144.11)\n")
                .unwrap()
                .as_str(),
            "26.6.29+2144.11"
        );
        let profile = adapter.select_profile(&NativeVersion::new(JUNIE.verified_version).unwrap());
        let capabilities = profile.mutation_capabilities().unwrap();
        for scope in [CapabilityScope::Global, CapabilityScope::Project] {
            let set = capabilities.for_scope_kind(scope);
            assert_eq!(
                set.support(&CapabilityId::new("managed.projection").unwrap()),
                Some(CapabilitySupport::Unverified)
            );
            assert_eq!(
                set.support(&CapabilityId::new("component.skill").unwrap()),
                Some(CapabilitySupport::Unverified)
            );
        }
        assert!(adapter.native_lifecycle().is_none());
        assert!(adapter.effective_state_probe().is_none());
        assert!(
            adapter
                .managed_declaration_contract(CapabilityScope::Global)
                .is_some()
        );
    }

    #[test]
    fn project_skills_use_the_documented_junie_root() {
        let paths = PlatformPaths::resolve_for(
            skilltap_core::runtime::SupportedPlatform::Linux,
            &TestEnvironment,
        )
        .unwrap();
        let project = AbsolutePath::new("/tmp/junie-project").unwrap();
        assert_eq!(
            JunieSkillProjection
                .destination(&paths, &Scope::Global)
                .unwrap()
                .as_str(),
            "/home/test/.junie/skills"
        );
        assert_eq!(
            JunieSkillProjection
                .destination(&paths, &Scope::Project(project))
                .unwrap()
                .as_str(),
            "/tmp/junie-project/.junie/skills"
        );
    }

    struct TestEnvironment;
    impl skilltap_core::runtime::Environment for TestEnvironment {
        fn value(&self, variable: skilltap_core::runtime::EnvironmentVariable) -> Option<OsString> {
            match variable {
                skilltap_core::runtime::EnvironmentVariable::Home => {
                    Some(OsString::from("/home/test"))
                }
                _ => None,
            }
        }
    }
}
