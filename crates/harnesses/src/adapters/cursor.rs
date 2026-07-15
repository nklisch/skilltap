use std::ffi::OsString;

use skilltap_core::{
    domain::{
        AbsolutePath, CapabilityId, CapabilityProfileSelection, CapabilitySet, CapabilitySupport,
        HarnessId, NativeVersion, Scope, ScopedCapabilitySets,
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
        TargetIdentity, TargetIdentityBoundary,
    },
};

static ADAPTER: CursorAdapter = CursorAdapter;

pub struct CursorAdapter;

impl CursorAdapter {
    pub fn static_ref() -> &'static dyn HarnessAdapter {
        &ADAPTER
    }
}

fn observation_capabilities() -> ScopedCapabilitySets {
    let set = CapabilitySet::new([
        (
            CapabilityId::new("harness.observe").expect("Cursor capability is valid"),
            CapabilitySupport::Supported,
        ),
        (
            CapabilityId::new("component.skill").expect("Cursor capability is valid"),
            CapabilitySupport::Unverified,
        ),
        (
            CapabilityId::new("component.mcp").expect("Cursor capability is valid"),
            CapabilitySupport::Unverified,
        ),
    ]);
    ScopedCapabilitySets::new(set.clone(), set)
}

impl HarnessAdapter for CursorAdapter {
    fn identity(&self) -> TargetIdentity {
        TargetIdentity {
            id: HarnessId::new("cursor").expect("static harness id is valid"),
            display_name: "Cursor",
            default_binary: Some("agent"),
            distribution_surface: DistributionSurface::Managed,
            identity_boundary: TargetIdentityBoundary::Executable,
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

    fn select_profile(&self, _version: &NativeVersion) -> CapabilityProfileSelection {
        CapabilityProfileSelection::unknown_version(observation_capabilities())
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
                    "cursor.agents.skills",
                    adapter_helpers::absolute_child(paths.home(), ".agents/skills"),
                ),
                (
                    "cursor.skills",
                    adapter_helpers::absolute_child(paths.home(), ".cursor/skills"),
                ),
            ],
            Scope::Project(project) => vec![
                (
                    "project.cursor.agents.skills",
                    adapter_helpers::absolute_child(project, ".agents/skills"),
                ),
                (
                    "project.cursor.skills",
                    adapter_helpers::absolute_child(project, ".cursor/skills"),
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
        match scope {
            Scope::Global => {
                if path_exists(paths.home(), ".cursor/mcp.json") {
                    surface_labels.push("cursor.global.mcp");
                }
            }
            Scope::Project(project) => {
                if path_exists(project, ".cursor/mcp.json") {
                    surface_labels.push("project.cursor.mcp");
                }
            }
        }
        Ok(AdapterObservationPaths {
            canonical,
            project_entry_count: matches!(scope, Scope::Project(_)).then_some(entries),
            surface_labels,
        })
    }

    fn unresolved_observation_boundaries(&self) -> &'static [&'static str] {
        &[
            "skill_precedence",
            "editor_cli_skill_equivalence",
            "effective_reload",
        ]
    }

    fn native_root(&self, paths: &PlatformPaths) -> Option<AbsolutePath> {
        adapter_helpers::absolute_child(paths.home(), ".cursor")
    }
}

fn path_exists(root: &AbsolutePath, child: &str) -> bool {
    std::fs::symlink_metadata(std::path::Path::new(root.as_str()).join(child)).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_uses_agent_and_never_selects_mutation_authority() {
        let adapter = CursorAdapter;
        assert_eq!(adapter.identity().default_binary, Some("agent"));
        assert_eq!(
            adapter.version_arguments(),
            Some(vec![OsString::from("--version")])
        );
        assert_eq!(
            adapter.decode_version(b"0.1.2\n").unwrap().as_str(),
            "0.1.2"
        );
        assert!(
            adapter
                .select_profile(&NativeVersion::new("0.1.2").unwrap())
                .mutation_capabilities()
                .is_none()
        );
        assert!(adapter.native_lifecycle().is_none());
        assert!(adapter.skill_projection().is_none());
        assert!(adapter.managed_projection().is_none());
    }
}
