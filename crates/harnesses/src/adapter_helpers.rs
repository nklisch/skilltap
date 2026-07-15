use skilltap_core::{
    domain::{
        CapabilityId, CapabilityProfileId, CapabilityProfileSelection, CapabilitySet,
        CapabilitySupport, NativeVersion, ScopedCapabilitySets,
    },
    runtime::{JsonLimits, StrictJson, StrictJsonDecoder},
};

use crate::DetectionError;

pub(crate) fn decode_codex_version(
    stdout: &[u8],
    json_limits: JsonLimits,
) -> Result<NativeVersion, DetectionError> {
    decode_native_version(stdout, json_limits, |text| text.strip_prefix("codex-cli "))
}

pub(crate) fn decode_claude_version(
    stdout: &[u8],
    json_limits: JsonLimits,
) -> Result<NativeVersion, DetectionError> {
    decode_native_version(stdout, json_limits, |text| {
        text.strip_suffix(" (Claude Code)")
    })
}

fn decode_native_version<'a>(
    stdout: &'a [u8],
    json_limits: JsonLimits,
    text_version: impl FnOnce(&'a str) -> Option<&'a str>,
) -> Result<NativeVersion, DetectionError> {
    let text = std::str::from_utf8(stdout).map_err(|_| DetectionError::InvalidVersion)?;
    let text = text.strip_suffix('\n').unwrap_or(text);
    let text = text.strip_suffix('\r').unwrap_or(text);
    if text.is_empty() || text.chars().any(char::is_control) {
        return Err(DetectionError::InvalidVersion);
    }

    let version = if text.starts_with('{') {
        let decoded = StrictJson
            .decode(stdout, json_limits)
            .map_err(|_| DetectionError::InvalidVersion)?;
        decoded
            .value()
            .as_object()
            .and_then(|object| object.get("version"))
            .and_then(serde_json::Value::as_str)
            .ok_or(DetectionError::InvalidVersion)?
            .to_owned()
    } else {
        text_version(text)
            .filter(|version| is_single_version_token(version))
            .ok_or(DetectionError::InvalidVersion)?
            .to_owned()
    };

    NativeVersion::new(&version).map_err(|_| DetectionError::InvalidVersion)
}

fn is_single_version_token(version: &str) -> bool {
    !version.is_empty() && !version.chars().any(char::is_whitespace)
}

pub(crate) fn select_profile(
    version: &NativeVersion,
    verified_version: &str,
    profile_id: &str,
    capabilities: ScopedCapabilitySets,
) -> CapabilityProfileSelection {
    if version.as_str() == verified_version {
        CapabilityProfileSelection::verified(
            CapabilityProfileId::new(profile_id).expect("compiled profile identifier is valid"),
            capabilities,
        )
    } else {
        CapabilityProfileSelection::unknown_version(unknown_capabilities(&capabilities))
    }
}

pub(crate) fn compiled_capabilities(
    plugin_update: bool,
    project_lifecycle: bool,
    managed_projection: bool,
) -> ScopedCapabilitySets {
    compiled_capabilities_with_components(
        plugin_update,
        project_lifecycle,
        managed_projection,
        CapabilitySupport::Supported,
        if managed_projection {
            CapabilitySupport::Supported
        } else {
            CapabilitySupport::Unverified
        },
    )
}

/// Build the common profile while keeping managed projection authority
/// independent from component-specific support. Targets such as Copilot can
/// verify MCP declarations while only declaring skill loading unverified.
pub(crate) fn compiled_capabilities_with_components(
    plugin_update: bool,
    project_lifecycle: bool,
    managed_projection: bool,
    component_skill: CapabilitySupport,
    component_mcp: CapabilitySupport,
) -> ScopedCapabilitySets {
    let support = |capability: &str, supported: bool| {
        (
            CapabilityId::new(capability).expect("compiled capability is valid"),
            if supported {
                CapabilitySupport::Supported
            } else {
                CapabilitySupport::Unverified
            },
        )
    };
    let component = |capability: &str, value: CapabilitySupport| {
        (
            CapabilityId::new(capability).expect("compiled capability is valid"),
            value,
        )
    };
    let global = CapabilitySet::new([
        support("harness.observe", true),
        support("managed.projection", managed_projection),
        support("plugin.install", true),
        support("plugin.remove", true),
        support("plugin.update", plugin_update),
        support("marketplace.register", true),
        support("marketplace.remove", true),
        support("marketplace.update", true),
        support("skill.install", true),
        support("skill.update", true),
        support("skill.remove", true),
        component("component.skill", component_skill),
        component("component.mcp", component_mcp),
    ]);
    let project_managed = project_lifecycle || managed_projection;
    let project = CapabilitySet::new([
        support("harness.observe", true),
        support("managed.projection", managed_projection),
        support("plugin.install", project_lifecycle),
        support("plugin.remove", project_lifecycle),
        support("plugin.update", project_lifecycle),
        support("marketplace.register", project_lifecycle),
        support("marketplace.remove", project_lifecycle),
        support("marketplace.update", project_lifecycle),
        support("skill.install", project_managed),
        support("skill.update", project_managed),
        support("skill.remove", project_managed),
        component("component.skill", component_skill),
        component("component.mcp", component_mcp),
    ]);
    ScopedCapabilitySets::new(global, project)
}

fn unknown_capabilities(baseline: &ScopedCapabilitySets) -> ScopedCapabilitySets {
    let unverified = |set: &CapabilitySet| {
        CapabilitySet::new(
            set.iter()
                .map(|(id, _)| (id.clone(), CapabilitySupport::Unverified)),
        )
    };
    ScopedCapabilitySets::new(
        unverified(baseline.for_scope_kind(skilltap_core::domain::CapabilityScope::Global)),
        unverified(baseline.for_scope_kind(skilltap_core::domain::CapabilityScope::Project)),
    )
}

pub(crate) fn observe_codex(
    paths: &skilltap_core::runtime::PlatformPaths,
    scope: &skilltap_core::domain::Scope,
    limits: skilltap_core::runtime::ExternalTreeLimits,
) -> Result<crate::registry::AdapterObservationPaths, crate::registry::ObservationPathError> {
    let inputs = crate::codex_observation_paths(paths, scope)?;
    let canonical = crate::observe_codex_canonical_resources(&inputs, scope, limits)?;
    let project_entry_count = if matches!(scope, skilltap_core::domain::Scope::Project(_)) {
        match crate::observe_codex_project_resources(&inputs, limits) {
            Ok(entries) => Some(entries),
            Err(skilltap_core::runtime::ObservationRuntimeError::TreeRootUnavailable) => None,
            Err(error) => return Err(error.into()),
        }
    } else {
        None
    };
    let surface_labels = codex_surface_labels(paths, scope, &inputs);
    Ok(crate::registry::AdapterObservationPaths {
        canonical,
        project_entry_count,
        surface_labels,
    })
}

pub(crate) fn observe_claude(
    paths: &skilltap_core::runtime::PlatformPaths,
    scope: &skilltap_core::domain::Scope,
    limits: skilltap_core::runtime::ExternalTreeLimits,
) -> Result<crate::registry::AdapterObservationPaths, crate::registry::ObservationPathError> {
    let inputs = crate::claude_observation_paths(paths, scope)?;
    let canonical = crate::observe_claude_canonical_resources(&inputs, scope, limits)?;
    let project_entry_count = if matches!(scope, skilltap_core::domain::Scope::Project(_)) {
        match crate::observe_claude_project_resources(&inputs, limits) {
            Ok(entries) => Some(entries),
            Err(skilltap_core::runtime::ObservationRuntimeError::TreeRootUnavailable) => None,
            Err(error) => return Err(error.into()),
        }
    } else {
        None
    };
    let surface_labels = claude_surface_labels(paths, scope, &inputs);
    Ok(crate::registry::AdapterObservationPaths {
        canonical,
        project_entry_count,
        surface_labels,
    })
}

fn codex_surface_labels(
    paths: &skilltap_core::runtime::PlatformPaths,
    scope: &skilltap_core::domain::Scope,
    inputs: &crate::CodexObservationPaths,
) -> Vec<&'static str> {
    let mut labels = Vec::new();
    match scope {
        skilltap_core::domain::Scope::Global => {
            push_if_exists(
                &mut labels,
                "codex.global.instructions",
                &inputs.global_agents,
            );
            push_child_if_exists(
                &mut labels,
                "codex.global.marketplace",
                paths.home(),
                ".agents/plugins/marketplace.json",
            );
            push_child_if_exists(
                &mut labels,
                "codex.global.config",
                paths.codex_home(),
                "config.toml",
            );
        }
        skilltap_core::domain::Scope::Project(project) => {
            if let Some(path) = inputs.project_agents.as_ref() {
                push_if_exists(&mut labels, "project.agents.instructions", path);
            }
            if let Some(path) = inputs.project_override.as_ref() {
                push_if_exists(&mut labels, "project.agents.override", path);
            }
            push_child_if_exists(
                &mut labels,
                "project.marketplace",
                project,
                ".agents/plugins/marketplace.json",
            );
            push_child_if_exists(
                &mut labels,
                "project.codex.config",
                project,
                ".codex/config.toml",
            );
        }
    }
    labels
}

fn claude_surface_labels(
    paths: &skilltap_core::runtime::PlatformPaths,
    scope: &skilltap_core::domain::Scope,
    inputs: &crate::ClaudeObservationPaths,
) -> Vec<&'static str> {
    let mut labels = Vec::new();
    match scope {
        skilltap_core::domain::Scope::Global => {
            push_if_exists(&mut labels, "claude.settings", &inputs.global_settings);
            push_child_if_exists(
                &mut labels,
                "claude.marketplace",
                paths.claude_home(),
                "plugins/known_marketplaces.json",
            );
            push_child_if_exists(
                &mut labels,
                "claude.instructions",
                paths.claude_home(),
                "CLAUDE.md",
            );
        }
        skilltap_core::domain::Scope::Project(project) => {
            if let Some(path) = inputs.project_settings.as_ref() {
                push_if_exists(&mut labels, "project.claude.settings", path);
            }
            if child_path_exists(project, "CLAUDE.md")
                || child_path_exists(project, ".claude/CLAUDE.md")
            {
                labels.push("project.claude.instructions");
            }
        }
    }
    labels
}

fn push_if_exists(
    labels: &mut Vec<&'static str>,
    label: &'static str,
    path: &skilltap_core::domain::AbsolutePath,
) {
    if path_exists(path) {
        labels.push(label);
    }
}

fn push_child_if_exists(
    labels: &mut Vec<&'static str>,
    label: &'static str,
    root: &skilltap_core::domain::AbsolutePath,
    child: &str,
) {
    if child_path_exists(root, child) {
        labels.push(label);
    }
}

/// Return true when a path exists as a filesystem entry, including a dangling
/// symlink. Adapter observation labels care about the declared surface being
/// present, not whether a symlink target currently resolves.
pub(crate) fn path_exists(path: &skilltap_core::domain::AbsolutePath) -> bool {
    std::fs::symlink_metadata(path.as_str()).is_ok()
}

/// Child-path variant of [`path_exists`] for documented adapter surfaces.
pub(crate) fn child_path_exists(root: &skilltap_core::domain::AbsolutePath, child: &str) -> bool {
    std::fs::symlink_metadata(std::path::Path::new(root.as_str()).join(child)).is_ok()
}

pub(crate) fn absolute_child(
    root: &skilltap_core::domain::AbsolutePath,
    child: &str,
) -> Option<skilltap_core::domain::AbsolutePath> {
    skilltap_core::domain::AbsolutePath::new(format!("{}/{child}", root.as_str())).ok()
}
