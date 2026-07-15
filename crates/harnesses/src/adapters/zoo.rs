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

const PROFILE_ID: &str = "zoo-observe-only";
static ADAPTER: ZooAdapter = ZooAdapter;
static READ_ONLY: ZooReadOnlyTarget = ZooReadOnlyTarget;

pub struct ZooAdapter;
pub struct ZooReadOnlyTarget;

impl ZooAdapter {
    pub fn static_ref() -> &'static dyn HarnessAdapter {
        &ADAPTER
    }
}

fn observation_capabilities() -> ScopedCapabilitySets {
    let set = CapabilitySet::new([
        (
            CapabilityId::new("harness.observe").expect("Zoo capability is valid"),
            CapabilitySupport::Supported,
        ),
        (
            CapabilityId::new("component.skill").expect("Zoo capability is valid"),
            CapabilitySupport::Unverified,
        ),
        (
            CapabilityId::new("component.mcp").expect("Zoo capability is valid"),
            CapabilitySupport::Unverified,
        ),
    ]);
    ScopedCapabilitySets::new(set.clone(), set)
}

impl HarnessAdapter for ZooAdapter {
    fn identity(&self) -> TargetIdentity {
        TargetIdentity {
            id: HarnessId::new("zoo").expect("static harness id is valid"),
            display_name: "Zoo Code",
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
        let roots = match scope {
            Scope::Global => vec![
                (
                    "zoo.roo.skills",
                    adapter_helpers::absolute_child(paths.home(), ".roo/skills"),
                ),
                (
                    "zoo.agents.skills",
                    adapter_helpers::absolute_child(paths.home(), ".agents/skills"),
                ),
            ],
            Scope::Project(project) => vec![
                (
                    "project.zoo.roo.skills",
                    adapter_helpers::absolute_child(project, ".roo/skills"),
                ),
                (
                    "project.zoo.agents.skills",
                    adapter_helpers::absolute_child(project, ".agents/skills"),
                ),
            ],
        };
        let mut canonical = Vec::new();
        let mut entries = 0usize;
        for (label, root) in roots
            .into_iter()
            .filter_map(|(label, root)| root.map(|root| (label, root)))
        {
            match SystemExternalTreeObserver.observe(&ExternalTreeRequest::new(root, limits)) {
                Ok(snapshot) => {
                    entries = entries.saturating_add(snapshot.entries().len());
                    canonical.push(crate::CanonicalObservation {
                        root: label.to_owned(),
                        snapshot,
                    });
                }
                Err(ObservationRuntimeError::TreeRootUnavailable) => {}
                Err(error) => return Err(ObservationPathError::Runtime(error)),
            }
        }
        let mut surface_labels = Vec::new();
        if matches!(scope, Scope::Project(project) if path_exists(project, ".roo/mcp.json")) {
            surface_labels.push("project.zoo.mcp");
        }
        // The global document lives under editor global storage; that root is
        // deliberately not guessed or read by this observe-only adapter.
        Ok(AdapterObservationPaths {
            canonical,
            project_entry_count: matches!(scope, Scope::Project(_)).then_some(entries),
            surface_labels,
        })
    }

    fn unresolved_observation_boundaries(&self) -> &'static [&'static str] {
        &[
            "installed_extension_identity",
            "host_redirection",
            "global_storage",
            "effective_reload",
        ]
    }

    fn native_root(&self, _paths: &PlatformPaths) -> Option<AbsolutePath> {
        None
    }
}

impl ReadOnlyTargetPort for ZooReadOnlyTarget {
    fn profile(&self) -> CapabilityProfileSelection {
        CapabilityProfileSelection::verified_observe_only(
            CapabilityProfileId::new(PROFILE_ID).expect("Zoo profile id is valid"),
            observation_capabilities(),
        )
    }

    fn unresolved_boundaries(&self) -> &'static [&'static str] {
        &[
            "installed_extension_identity",
            "host_redirection",
            "global_storage",
            "effective_reload",
        ]
    }
}

fn path_exists(root: &AbsolutePath, child: &str) -> bool {
    std::fs::symlink_metadata(std::path::Path::new(root.as_str()).join(child)).is_ok()
}
