use std::{ffi::OsString, sync::LazyLock};

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

use super::kiro_managed::KiroManagedProjection;

const VERIFIED_VERSION: &str = "2.12.2";
const PROFILE_ID: &str = "kiro-2-12-2";

pub struct KiroAdapter;
pub struct KiroSkillProjection;

static ADAPTER: KiroAdapter = KiroAdapter;
static SKILLS: KiroSkillProjection = KiroSkillProjection;
static DECLARATION_CONTRACT: LazyLock<ManagedDeclarationContract> = LazyLock::new(|| {
    ManagedDeclarationContract::new([
        ManagedSurfaceKind::ManagedDocument,
        ManagedSurfaceKind::CompleteSkillTree,
    ])
    .expect("Kiro declaration contract is non-empty")
});

impl KiroAdapter {
    pub fn static_ref() -> &'static dyn HarnessAdapter {
        &ADAPTER
    }
}

fn declaration_capabilities() -> ScopedCapabilitySets {
    let capability = |id: &str, support| {
        (
            CapabilityId::new(id).expect("Kiro compiled capability is valid"),
            support,
        )
    };
    let capabilities = CapabilitySet::new([
        capability("harness.observe", CapabilitySupport::Supported),
        capability("managed.projection", CapabilitySupport::Unverified),
        // Standalone complete skills have a deterministic documented tree
        // contract; declaration uncertainty applies to managed plugin files
        // and MCP, not the canonical skill lifecycle.
        capability("component.skill", CapabilitySupport::Supported),
        capability("component.mcp", CapabilitySupport::Unverified),
        // Standalone Agent Skills use the shared canonical skill lifecycle.
        // Kiro's documented native root is still an attested projection path;
        // managed plugin declarations above remain acknowledgment-gated.
        capability("skill.install", CapabilitySupport::Supported),
        capability("skill.update", CapabilitySupport::Supported),
        capability("skill.remove", CapabilitySupport::Supported),
    ]);
    ScopedCapabilitySets::new(capabilities.clone(), capabilities)
}

impl HarnessAdapter for KiroAdapter {
    fn identity(&self) -> TargetIdentity {
        TargetIdentity {
            id: HarnessId::new("kiro").expect("static harness id is valid"),
            display_name: "Kiro CLI",
            default_binary: Some("kiro-cli"),
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
            declaration_capabilities(),
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
        // Kiro's managed roots are optional until the first declaration is
        // written. An empty documented surface is a valid observation; it
        // must not force marketplace source registration to fail after its
        // control-plane operation has completed.
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

    fn managed_declaration_contract(
        &self,
        _scope: skilltap_core::domain::CapabilityScope,
    ) -> Option<&'static ManagedDeclarationContract> {
        Some(&DECLARATION_CONTRACT)
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

fn path_exists(path: &AbsolutePath) -> bool {
    std::fs::symlink_metadata(path.as_str()).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    use skilltap_core::domain::{CapabilityScope, CapabilitySupport};

    #[test]
    fn exact_version_profile_is_authorized_and_adjacent_versions_are_unknown() {
        let adapter = KiroAdapter;
        assert_eq!(adapter.identity().default_binary, Some("kiro-cli"));
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
        let known = adapter.select_profile(&NativeVersion::new("2.12.2").unwrap());
        assert!(known.mutation_capabilities().is_some());
        let global = known
            .mutation_capabilities()
            .unwrap()
            .for_scope_kind(CapabilityScope::Global);
        let project = known
            .mutation_capabilities()
            .unwrap()
            .for_scope_kind(CapabilityScope::Project);
        for capabilities in [global, project] {
            assert_eq!(
                capabilities.support(
                    &skilltap_core::domain::CapabilityId::new("managed.projection").unwrap()
                ),
                Some(CapabilitySupport::Unverified)
            );
            assert_eq!(
                capabilities
                    .support(&skilltap_core::domain::CapabilityId::new("component.skill").unwrap()),
                Some(CapabilitySupport::Supported)
            );
            assert_eq!(
                capabilities
                    .support(&skilltap_core::domain::CapabilityId::new("component.mcp").unwrap()),
                Some(CapabilitySupport::Unverified)
            );
            assert!(capabilities.iter().all(|(id, _)| !matches!(
                id.as_str(),
                "plugin.install"
                    | "plugin.remove"
                    | "plugin.update"
                    | "marketplace.register"
                    | "marketplace.remove"
                    | "marketplace.update"
            )));
        }
        assert_eq!(
            adapter
                .managed_declaration_contract(CapabilityScope::Global)
                .unwrap()
                .surfaces(),
            &BTreeSet::from([
                ManagedSurfaceKind::ManagedDocument,
                ManagedSurfaceKind::CompleteSkillTree,
            ])
        );
        assert!(adapter.effective_state_probe().is_none());
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
}
