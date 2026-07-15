use std::ffi::OsString;

use skilltap_core::{
    domain::{
        AbsolutePath, CapabilityId, CapabilityProfileId, CapabilityProfileSelection, CapabilitySet,
        CapabilitySupport, HarnessId, NativeVersion, Scope, ScopedCapabilitySets,
    },
    runtime::{
        ExternalTreeLimits, ExternalTreeObserver, ExternalTreeRequest, ObservationRuntimeError,
        PlatformPaths, SystemExternalTreeObserver,
    },
};

use crate::{
    adapter_helpers,
    registry::{
        AdapterObservationPaths, DistributionSurface, HarnessAdapter, ObservationPathError,
        ReadOnlyTargetPort, TargetIdentity, TargetIdentityBoundary,
    },
};

const PROFILE_ID: &str = "zcode-observe-only";
static ADAPTER: ZCodeAdapter = ZCodeAdapter;
static READ_ONLY: ZCodeReadOnlyTarget = ZCodeReadOnlyTarget;

pub struct ZCodeAdapter;
pub struct ZCodeReadOnlyTarget;

impl ZCodeAdapter {
    pub fn static_ref() -> &'static dyn HarnessAdapter {
        &ADAPTER
    }
}

fn observation_capabilities() -> ScopedCapabilitySets {
    let set = CapabilitySet::new([
        (
            CapabilityId::new("harness.observe").expect("ZCode capability is valid"),
            CapabilitySupport::Supported,
        ),
        (
            CapabilityId::new("component.skill").expect("ZCode capability is valid"),
            CapabilitySupport::Unverified,
        ),
        (
            CapabilityId::new("component.mcp").expect("ZCode capability is valid"),
            CapabilitySupport::Unverified,
        ),
    ]);
    ScopedCapabilitySets::new(set.clone(), set)
}

impl HarnessAdapter for ZCodeAdapter {
    fn identity(&self) -> TargetIdentity {
        TargetIdentity {
            id: HarnessId::new("zcode").expect("static harness id is valid"),
            display_name: "ZCode",
            default_binary: None,
            distribution_surface: DistributionSurface::Managed,
            identity_boundary: TargetIdentityBoundary::FileOnly,
        }
    }

    fn version_arguments(&self) -> Option<Vec<OsString>> {
        None
    }

    fn decode_version(&self, _stdout: &[u8]) -> Result<NativeVersion, crate::DetectionError> {
        Err(crate::DetectionError::InvalidVersion)
    }

    fn select_profile(&self, _version: &NativeVersion) -> CapabilityProfileSelection {
        READ_ONLY.profile()
    }

    fn read_only_target(&self) -> Option<&dyn ReadOnlyTargetPort> {
        Some(&READ_ONLY)
    }

    fn observe(
        &self,
        paths: &PlatformPaths,
        scope: &Scope,
        limits: ExternalTreeLimits,
    ) -> Result<AdapterObservationPaths, ObservationPathError> {
        let mut canonical = Vec::new();
        let mut surface_labels = Vec::new();
        let mut project_entry_count = None;
        match scope {
            Scope::Global => {
                let root = adapter_helpers::absolute_child(paths.home(), ".zcode/skills");
                if let Some(root) = root {
                    match SystemExternalTreeObserver
                        .observe(&ExternalTreeRequest::new(root, limits))
                    {
                        Ok(snapshot) => {
                            canonical.push(crate::CanonicalObservation {
                                root: "zcode.skills".to_owned(),
                                snapshot,
                            });
                        }
                        Err(ObservationRuntimeError::TreeRootUnavailable) => {}
                        Err(error) => return Err(ObservationPathError::Runtime(error)),
                    }
                }
                if adapter_helpers::child_path_exists(paths.home(), ".zcode/cli/config.json") {
                    surface_labels.push("zcode.global.mcp");
                }
            }
            Scope::Project(project) => {
                // The public contract names the workspace MCP declaration but
                // does not name a project skill root; do not infer one.
                if adapter_helpers::child_path_exists(project, ".zcode/config.json") {
                    surface_labels.push("project.zcode.mcp");
                }
            }
        }
        if matches!(scope, Scope::Project(_)) {
            project_entry_count = Some(
                canonical
                    .iter()
                    .map(|item| item.snapshot.entries().len())
                    .sum(),
            );
        }
        Ok(AdapterObservationPaths {
            canonical,
            project_entry_count,
            surface_labels,
        })
    }

    fn unresolved_observation_boundaries(&self) -> &'static [&'static str] {
        &[
            "installed_identity",
            "project_skill_root",
            "effective_reload",
            "cache_independence",
        ]
    }

    fn native_root(&self, paths: &PlatformPaths) -> Option<AbsolutePath> {
        adapter_helpers::absolute_child(paths.home(), ".zcode")
    }
}

impl ReadOnlyTargetPort for ZCodeReadOnlyTarget {
    fn profile(&self) -> CapabilityProfileSelection {
        CapabilityProfileSelection::verified_observe_only(
            CapabilityProfileId::new(PROFILE_ID).expect("ZCode profile id is valid"),
            observation_capabilities(),
        )
    }

    fn unresolved_boundaries(&self) -> &'static [&'static str] {
        &[
            "installed_identity",
            "project_skill_root",
            "effective_reload",
            "cache_independence",
        ]
    }
}
