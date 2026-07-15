use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
};

use skilltap_core::{
    domain::{
        AbsolutePath, CapabilityProfileSelection, CapabilityScope, HarnessId, NativeId,
        NativeVersion, Scope,
    },
    materialization::{MaterializationSupport, plan_materialization},
    plugin_graph::normalize,
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
    lifecycle::{
        NativeLifecycleAction, NativeLifecycleError, NativeLifecycleRequest,
        NativeObservationFailure, NativeResourceObservation,
    },
    managed_projection::ManagedProjectionPort,
    native_distribution::{
        NativeDistributionAssessment, NativeDistributionContext, NativeDistributionError,
        NativeDistributionPort,
    },
    registry::{
        AdapterObservationPaths, DistributionSurface, HarnessAdapter, NativeLifecycleVector,
        ObservationPathError, SkillProjectionPort, TargetIdentity,
    },
};

use super::qwen_managed::{QwenManagedProjection, QwenSourceFlavor, read_qwen_source_plugin};

const VERIFIED_VERSION: &str = "0.19.10";
const PROFILE_ID: &str = "qwen-0-19-10";
const QWEN_HOME: &str = ".qwen";

pub struct QwenAdapter;
pub struct QwenLifecycle;
pub struct QwenSkillProjection;
pub struct QwenNativeDistribution;
pub struct QwenEffectiveStateProbe;

static ADAPTER: QwenAdapter = QwenAdapter;
static LIFECYCLE: QwenLifecycle = QwenLifecycle;
static SKILLS: QwenSkillProjection = QwenSkillProjection;
static DISTRIBUTION: QwenNativeDistribution = QwenNativeDistribution;
static PROBE: QwenEffectiveStateProbe = QwenEffectiveStateProbe;

impl QwenAdapter {
    pub fn static_ref() -> &'static dyn HarnessAdapter {
        &ADAPTER
    }
}

impl HarnessAdapter for QwenAdapter {
    fn identity(&self) -> TargetIdentity {
        TargetIdentity {
            id: HarnessId::new("qwen").expect("static harness id is valid"),
            display_name: "Qwen Code",
            default_binary: Some("qwen"),
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
        if text.is_empty() || text.chars().any(|c| c.is_control() || c.is_whitespace()) {
            return Err(crate::DetectionError::InvalidVersion);
        }
        NativeVersion::new(text).map_err(|_| crate::DetectionError::InvalidVersion)
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
        let root = match scope {
            Scope::Global => absolute_child(paths.home(), QWEN_HOME),
            Scope::Project(project) => absolute_child(project, QWEN_HOME),
        };
        let mut canonical = Vec::new();
        if let Some(root) = root {
            match SystemExternalTreeObserver.observe(&ExternalTreeRequest::new(root, limits)) {
                Ok(snapshot) => canonical.push(crate::CanonicalObservation {
                    root: match scope {
                        Scope::Global => "qwen.home",
                        Scope::Project(_) => "project.qwen",
                    }
                    .to_owned(),
                    snapshot,
                }),
                Err(ObservationRuntimeError::TreeRootUnavailable) => {}
                Err(error) => return Err(ObservationPathError::Runtime(error)),
            }
        }
        let base = match scope {
            Scope::Global => paths.home(),
            Scope::Project(project) => project,
        };
        let mut surface_labels = Vec::new();
        for (label, path) in [
            ("qwen.settings", ".qwen/settings.json"),
            ("qwen.skills", ".qwen/skills"),
            ("qwen.extensions", ".qwen/extensions"),
            ("qwen.extension_store", ".qwen/extension-store/state.json"),
        ] {
            if crate::adapter_helpers::child_path_exists(base, path) {
                surface_labels.push(match scope {
                    Scope::Global => label,
                    Scope::Project(_) => match label {
                        "qwen.settings" => "project.qwen.settings",
                        "qwen.skills" => "project.qwen.skills",
                        "qwen.extensions" => "project.qwen.extensions",
                        _ => "project.qwen.extension_store",
                    },
                });
            }
        }
        if canonical.is_empty() && surface_labels.is_empty() {
            return Err(ObservationPathError::Runtime(
                ObservationRuntimeError::TreeRootUnavailable,
            ));
        }
        Ok(AdapterObservationPaths {
            canonical,
            project_entry_count: matches!(scope, Scope::Project(_)).then(|| 1),
            surface_labels,
        })
    }

    fn native_lifecycle(&self) -> Option<&dyn NativeLifecycleVector> {
        Some(&LIFECYCLE)
    }
    fn skill_projection(&self) -> Option<&dyn SkillProjectionPort> {
        Some(&SKILLS)
    }
    fn native_distribution(&self) -> Option<&dyn NativeDistributionPort> {
        Some(&DISTRIBUTION)
    }
    fn managed_projection(&self) -> Option<&dyn ManagedProjectionPort> {
        Some(QwenManagedProjection::static_ref())
    }
    fn effective_state_probe(&self) -> Option<&dyn EffectiveStateProbePort> {
        Some(&PROBE)
    }
    fn native_root(&self, paths: &PlatformPaths) -> Option<AbsolutePath> {
        absolute_child(paths.home(), QWEN_HOME)
    }
}

impl NativeLifecycleVector for QwenLifecycle {
    fn arguments(
        &self,
        request: &NativeLifecycleRequest,
    ) -> Result<Vec<OsString>, NativeLifecycleError> {
        crate::lifecycle::validate_native_request(request)?;
        let mut args = vec![OsString::from("extensions")];
        match request.action {
            NativeLifecycleAction::MarketplaceAdd => {
                args.extend(["sources", "add"].into_iter().map(OsString::from));
                args.push(OsString::from(
                    request
                        .source
                        .as_ref()
                        .ok_or(NativeLifecycleError::MissingSource)?
                        .as_str(),
                ));
            }
            NativeLifecycleAction::MarketplaceRemove => args.extend(
                ["sources", "remove", request.name.as_str()]
                    .into_iter()
                    .map(OsString::from),
            ),
            NativeLifecycleAction::MarketplaceUpdate => args.extend(
                ["sources", "update", request.name.as_str()]
                    .into_iter()
                    .map(OsString::from),
            ),
            NativeLifecycleAction::PluginInstall => {
                args.push(OsString::from("install"));
                args.push(OsString::from(native_selector(request)));
                args.extend(
                    ["--scope", qwen_scope(request)]
                        .into_iter()
                        .map(OsString::from),
                );
            }
            NativeLifecycleAction::PluginRemove => {
                args.push(OsString::from("uninstall"));
                args.push(OsString::from(request.name.as_str()));
                args.extend(
                    ["--scope", qwen_scope(request)]
                        .into_iter()
                        .map(OsString::from),
                );
            }
            NativeLifecycleAction::PluginUpdate => {
                args.push(OsString::from("update"));
                args.push(OsString::from(request.name.as_str()));
                args.extend(
                    ["--scope", qwen_scope(request)]
                        .into_iter()
                        .map(OsString::from),
                );
            }
        }
        Ok(args)
    }

    fn observation_scope(&self, _scope: &Scope) -> Option<CapabilityScope> {
        Some(CapabilityScope::from(_scope))
    }

    fn observation_arguments(
        &self,
        request: &NativeLifecycleRequest,
    ) -> Result<Vec<OsString>, NativeLifecycleError> {
        crate::lifecycle::validate_native_request(request)?;
        match request.action {
            NativeLifecycleAction::MarketplaceAdd
            | NativeLifecycleAction::MarketplaceRemove
            | NativeLifecycleAction::MarketplaceUpdate => Ok(["extensions", "sources", "list"]
                .into_iter()
                .map(OsString::from)
                .collect()),
            NativeLifecycleAction::PluginInstall
            | NativeLifecycleAction::PluginRemove
            | NativeLifecycleAction::PluginUpdate => {
                Ok(["extensions", "list", "--scope", qwen_scope(request)]
                    .into_iter()
                    .map(OsString::from)
                    .collect())
            }
        }
    }

    fn decode_observation(
        &self,
        stdout: &[u8],
        dispatch: &crate::NativeLifecycleDispatch,
        limits: JsonLimits,
    ) -> NativeResourceObservation {
        match dispatch.request().action {
            NativeLifecycleAction::MarketplaceAdd
            | NativeLifecycleAction::MarketplaceRemove
            | NativeLifecycleAction::MarketplaceUpdate => {
                decode_qwen_source_list(stdout, dispatch.request().name.as_str(), limits)
            }
            NativeLifecycleAction::PluginInstall
            | NativeLifecycleAction::PluginRemove
            | NativeLifecycleAction::PluginUpdate => decode_qwen_extension_list(
                stdout,
                dispatch.request().name.as_str(),
                qwen_scope(dispatch.request()),
                limits,
            ),
        }
    }
}

impl QwenLifecycle {
    /// Qwen exposes enablement as a separate native lifecycle from install.
    /// The shared lifecycle enum predates target-specific enable operations, so
    /// these vectors remain explicit and are not inferred from installation.
    pub fn enable_arguments(
        &self,
        request: &NativeLifecycleRequest,
    ) -> Result<Vec<OsString>, NativeLifecycleError> {
        qwen_enable_arguments(request, "enable")
    }
    pub fn disable_arguments(
        &self,
        request: &NativeLifecycleRequest,
    ) -> Result<Vec<OsString>, NativeLifecycleError> {
        qwen_enable_arguments(request, "disable")
    }
}

impl SkillProjectionPort for QwenSkillProjection {
    fn destination(&self, paths: &PlatformPaths, scope: &Scope) -> Option<AbsolutePath> {
        match scope {
            Scope::Global => absolute_child(paths.home(), ".qwen/skills"),
            Scope::Project(project) => absolute_child(project, ".qwen/skills"),
        }
    }
}

impl NativeDistributionPort for QwenNativeDistribution {
    fn assess(
        &self,
        context: &NativeDistributionContext<'_>,
    ) -> Result<Option<NativeDistributionAssessment>, NativeDistributionError> {
        // Qwen's native updater owns its source revision. A requested Git
        // revision must remain managed until Qwen proves it can honor that pin.
        if context.requested_revision.is_some() {
            return Ok(None);
        }
        let plugin = match read_qwen_source_plugin(
            context.filesystem,
            context.checkout.root(),
            context.checkout.source(),
            context.json_limits,
        ) {
            Ok(plugin) => plugin,
            Err(NativeDistributionError::UnsupportedSource) => return Ok(None),
            Err(error) => return Err(error),
        };
        let flavor = plugin.flavor;
        let graph = normalize(
            context.checkout.source().clone(),
            plugin.plugin.declarations,
        )
        .map_err(|_| NativeDistributionError::InvalidAssessment)?;
        if graph.components().is_empty() {
            return Ok(None);
        }
        let supported = graph
            .components()
            .iter()
            .filter(|(_, component)| qwen_conversion_support(flavor, &component.kind))
            .map(|(id, _)| id.clone())
            .collect::<BTreeSet<_>>();
        let plan = plan_materialization(
            graph.components(),
            &MaterializationSupport {
                target: context.target.clone(),
                supported,
            },
        );
        Ok(Some(NativeDistributionAssessment { graph, plan }))
    }
}

impl EffectiveStateProbePort for QwenEffectiveStateProbe {
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
        decode_qwen_mcp_status(output, limits)
    }
    fn reload_semantics(&self) -> ReloadSemantics {
        ReloadSemantics::InteractiveRequired {
            next_action: "Start a new Qwen Code session in the same project, then re-run status.",
        }
    }
}

fn qwen_conversion_support(
    flavor: QwenSourceFlavor,
    kind: &skilltap_core::domain::ComponentKind,
) -> bool {
    use skilltap_core::domain::ComponentKind;
    match flavor {
        QwenSourceFlavor::Qwen => {
            matches!(
                kind,
                ComponentKind::Skill
                    | ComponentKind::McpServer
                    | ComponentKind::Agent
                    | ComponentKind::Command
            ) || matches!(kind, ComponentKind::HarnessSpecific(id) if id.as_str() == "context")
        }
        QwenSourceFlavor::Claude | QwenSourceFlavor::Gemini => {
            matches!(kind, ComponentKind::Skill | ComponentKind::Command)
                || matches!(kind, ComponentKind::HarnessSpecific(id) if id.as_str() == "context")
        }
    }
}

fn qwen_scope(request: &NativeLifecycleRequest) -> &'static str {
    if matches!(request.scope, Scope::Global) {
        "user"
    } else {
        "workspace"
    }
}

fn native_selector(request: &NativeLifecycleRequest) -> String {
    request
        .source
        .as_ref()
        .map(|source| format!("{}:{}", source.as_str(), request.name.as_str()))
        .unwrap_or_else(|| request.name.as_str().to_owned())
}

fn qwen_enable_arguments(
    request: &NativeLifecycleRequest,
    action: &'static str,
) -> Result<Vec<OsString>, NativeLifecycleError> {
    crate::lifecycle::validate_native_request(request)?;
    Ok([
        "extensions",
        action,
        request.name.as_str(),
        "--scope",
        qwen_scope(request),
    ]
    .into_iter()
    .map(OsString::from)
    .collect())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QwenExtensionRecord {
    pub name: String,
    pub version: Option<NativeId>,
    pub path: Option<String>,
    pub source: Option<String>,
    pub source_type: Option<String>,
    pub enabled_user: bool,
    pub enabled_workspace: bool,
    pub components: BTreeSet<String>,
}

/// Decode Qwen 0.19.10's human-only extension list. The parser is deliberately
/// version-owned: JSON and unscoped/ambiguous records are not presence evidence.
pub fn decode_qwen_extensions(
    output: &[u8],
    limits: JsonLimits,
) -> Result<Vec<QwenExtensionRecord>, NativeObservationFailure> {
    if output.len() as u64 > limits.bytes() {
        return Err(NativeObservationFailure::InvalidJson);
    }
    let text =
        std::str::from_utf8(output).map_err(|_| NativeObservationFailure::UnsupportedShape)?;
    if text.trim() == "No extensions installed." {
        return Ok(Vec::new());
    }
    if text.contains('{')
        || text.contains('}')
        || !text
            .lines()
            .any(|line| line.trim() == "Installed extensions:")
    {
        return Err(NativeObservationFailure::UnsupportedShape);
    }
    let mut records = Vec::new();
    let mut current: Option<QwenExtensionRecord> = None;
    let mut section: Option<&str> = None;
    for raw in text
        .lines()
        .skip_while(|line| line.trim() != "Installed extensions:")
        .skip(1)
    {
        let line = raw.trim_end();
        if line.trim().is_empty() {
            continue;
        }
        let trimmed = line.trim();
        let indentation = raw.len() - raw.trim_start_matches([' ', '\t']).len();
        if indentation == 0 || indentation == 2 {
            if let Some(record) = current.take() {
                records.push(record);
            }
            current = Some(QwenExtensionRecord {
                name: trimmed.trim_end_matches(':').to_owned(),
                version: None,
                path: None,
                source: None,
                source_type: None,
                enabled_user: false,
                enabled_workspace: false,
                components: BTreeSet::new(),
            });
            section = None;
            continue;
        }
        let Some(record) = current.as_mut() else {
            return Err(NativeObservationFailure::UnsupportedShape);
        };
        if let Some(value) = trimmed.strip_prefix("Version:") {
            record.version = Some(
                NativeId::new(value.trim())
                    .map_err(|_| NativeObservationFailure::UnsupportedShape)?,
            );
        } else if let Some(value) = trimmed.strip_prefix("Path:") {
            record.path = Some(value.trim().to_owned());
        } else if let Some(value) = trimmed.strip_prefix("Source:") {
            record.source = Some(value.trim().to_owned());
        } else if let Some(value) = trimmed.strip_prefix("Type:") {
            record.source_type = Some(value.trim().to_owned());
        } else if let Some(value) = trimmed.strip_prefix("Enabled (User):") {
            record.enabled_user = parse_bool(value.trim())?;
        } else if let Some(value) = trimmed.strip_prefix("Enabled (Workspace):") {
            record.enabled_workspace = parse_bool(value.trim())?;
        } else if trimmed == "Enabled (User)" {
            record.enabled_user = true;
        } else if trimmed == "Enabled (Workspace)" {
            record.enabled_workspace = true;
        } else if matches!(
            trimmed,
            "Context files:" | "Commands:" | "Skills:" | "MCP servers:" | "Agents:"
        ) {
            section = Some(trimmed);
        } else if let Some(value) = trimmed.strip_prefix('-') {
            if section.is_some() {
                record.components.insert(value.trim().to_owned());
            }
        } else if trimmed.starts_with("Description:") {
            return Err(NativeObservationFailure::UnsupportedShape);
        }
    }
    if let Some(record) = current {
        records.push(record);
    }
    if records.is_empty() {
        return Err(NativeObservationFailure::UnsupportedShape);
    }
    Ok(records)
}

fn parse_bool(value: &str) -> Result<bool, NativeObservationFailure> {
    match value {
        "true" | "enabled" | "yes" | "Enabled" => Ok(true),
        "false" | "disabled" | "no" | "Disabled" => Ok(false),
        _ => Err(NativeObservationFailure::UnsupportedShape),
    }
}

fn decode_qwen_extension_list(
    output: &[u8],
    expected_name: &str,
    scope: &str,
    limits: JsonLimits,
) -> NativeResourceObservation {
    let records = match decode_qwen_extensions(output, limits) {
        Ok(records) => records,
        Err(error) => return NativeResourceObservation::Indeterminate(error),
    };
    let matches = records
        .iter()
        .filter(|record| {
            record.name == expected_name
                && ((scope == "user" && record.enabled_user)
                    || (scope == "workspace" && record.enabled_workspace))
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [] => NativeResourceObservation::Missing,
        [record] => NativeResourceObservation::Present {
            scope: Some(if scope == "user" {
                CapabilityScope::Global
            } else {
                CapabilityScope::Project
            }),
            revision: record
                .version
                .clone()
                .map(skilltap_core::domain::ResolvedRevision::Native),
        },
        _ => NativeResourceObservation::Indeterminate(NativeObservationFailure::AmbiguousScope),
    }
}

fn decode_qwen_source_list(
    output: &[u8],
    expected_name: &str,
    limits: JsonLimits,
) -> NativeResourceObservation {
    if output.len() as u64 > limits.bytes() {
        return NativeResourceObservation::Indeterminate(NativeObservationFailure::InvalidJson);
    }
    let text = match std::str::from_utf8(output) {
        Ok(text) => text,
        Err(_) => {
            return NativeResourceObservation::Indeterminate(
                NativeObservationFailure::UnsupportedShape,
            );
        }
    };
    if text.contains('{')
        || !text.lines().any(|line| {
            matches!(
                line.trim(),
                "Configured extension sources:" | "Extension sources:"
            )
        })
    {
        return NativeResourceObservation::Indeterminate(
            NativeObservationFailure::UnsupportedShape,
        );
    }
    let names = text
        .lines()
        .skip_while(|line| {
            !matches!(
                line.trim(),
                "Configured extension sources:" | "Extension sources:"
            )
        })
        .skip(1)
        .filter_map(|line| {
            if line.trim().is_empty() || line.starts_with(' ') || line.starts_with('\t') {
                None
            } else {
                Some(line.trim().trim_end_matches(':').to_owned())
            }
        })
        .collect::<Vec<_>>();
    match names
        .iter()
        .filter(|name| name.as_str() == expected_name)
        .count()
    {
        0 => NativeResourceObservation::Missing,
        1 => NativeResourceObservation::Present {
            scope: None,
            revision: None,
        },
        _ => NativeResourceObservation::Indeterminate(NativeObservationFailure::AmbiguousScope),
    }
}

/// Decode the exact human MCP status boundary used by the Qwen 0.19.10 probe.
pub fn decode_qwen_mcp_status(
    output: &[u8],
    limits: JsonLimits,
) -> Result<EffectiveMcpStatus, EffectiveProbeError> {
    if output.len() as u64 > limits.bytes() {
        return Err(EffectiveProbeError::InvalidPayload);
    }
    let text = std::str::from_utf8(output).map_err(|_| EffectiveProbeError::InvalidPayload)?;
    if text.contains('{')
        || !text
            .lines()
            .any(|line| line.trim() == "Configured MCP servers:")
    {
        if text.contains("No MCP servers configured.") {
            return Ok(EffectiveMcpStatus {
                servers: BTreeMap::new(),
                project_trust: None,
            });
        }
        return Err(EffectiveProbeError::InvalidPayload);
    }
    let mut servers = BTreeMap::new();
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let Some((name, detail)) = line.split_once(": ") else {
            continue;
        };
        if matches!(name, "Configured MCP servers" | "Project" | "Approval") {
            continue;
        }
        let id = NativeId::new(name).map_err(|_| EffectiveProbeError::InvalidPayload)?;
        let detail_lower = detail.to_ascii_lowercase();
        let health = if detail_lower.contains("pending approval") {
            EffectiveServerHealth::Unknown
        } else if detail_lower.contains("connected") || detail_lower.contains("ready") {
            EffectiveServerHealth::Healthy
        } else if detail_lower.contains("disabled") {
            EffectiveServerHealth::Disabled
        } else if detail_lower.contains("failed")
            || detail_lower.contains("disconnected")
            || detail_lower.contains("error")
        {
            EffectiveServerHealth::Unhealthy
        } else {
            EffectiveServerHealth::Unknown
        };
        if servers.insert(id, health).is_some() {
            return Err(EffectiveProbeError::InvalidPayload);
        }
    }
    if servers.is_empty() {
        return Err(EffectiveProbeError::InvalidPayload);
    }
    let project_trust = if text.to_ascii_lowercase().contains("untrusted") {
        Some(ProjectTrustHealth::Untrusted)
    } else {
        Some(ProjectTrustHealth::Trusted)
    };
    Ok(EffectiveMcpStatus {
        servers,
        project_trust,
    })
}

fn absolute_child(base: &AbsolutePath, child: &str) -> Option<AbsolutePath> {
    skilltap_core::domain::AbsolutePath::new(format!("{}/{}", base.as_str(), child)).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> JsonLimits {
        JsonLimits::new(16 * 1024, 64).unwrap()
    }

    #[test]
    fn exact_qwen_version_and_profile_are_narrow() {
        let adapter = QwenAdapter;
        assert_eq!(
            adapter.decode_version(b"0.19.10\n").unwrap().as_str(),
            "0.19.10"
        );
        assert_eq!(
            adapter
                .select_profile(&NativeVersion::new("0.19.10").unwrap())
                .profile_id()
                .unwrap()
                .as_str(),
            PROFILE_ID
        );
        assert!(
            adapter
                .select_profile(&NativeVersion::new("0.19.10").unwrap())
                .mutation_capabilities()
                .is_some()
        );
        for version in ["0.19.9", "0.19.11", "99.0.0"] {
            assert!(
                adapter
                    .select_profile(&NativeVersion::new(version).unwrap())
                    .mutation_capabilities()
                    .is_none()
            );
        }
    }

    #[test]
    fn lifecycle_uses_qwen_sources_and_workspace_scope_without_json_claims() {
        let source = skilltap_core::domain::SourceLocator::new("/tmp/qwen-market").unwrap();
        let global = NativeLifecycleRequest {
            action: NativeLifecycleAction::PluginInstall,
            scope: Scope::Global,
            name: NativeId::new("demo").unwrap(),
            source: Some(source.clone()),
        };
        assert_eq!(
            LIFECYCLE.arguments(&global).unwrap(),
            [
                "extensions",
                "install",
                "/tmp/qwen-market:demo",
                "--scope",
                "user"
            ]
            .map(OsString::from)
        );
        let project = NativeLifecycleRequest {
            action: NativeLifecycleAction::PluginUpdate,
            scope: Scope::Project(AbsolutePath::new("/tmp/project").unwrap()),
            name: NativeId::new("demo").unwrap(),
            source: None,
        };
        assert_eq!(
            LIFECYCLE.arguments(&project).unwrap(),
            ["extensions", "update", "demo", "--scope", "workspace"].map(OsString::from)
        );
        assert_eq!(
            LIFECYCLE.observation_arguments(&project).unwrap(),
            ["extensions", "list", "--scope", "workspace"].map(OsString::from)
        );
        assert_eq!(
            PROBE
                .mcp_status_spec(&Scope::Project(AbsolutePath::new("/tmp/project").unwrap()))
                .arguments,
            ["mcp", "list"].map(OsString::from)
        );
    }

    #[test]
    fn human_extension_list_preserves_enablement_and_components() {
        let records = decode_qwen_extensions(b"Installed extensions:\n  demo\n    Version: 1.2.3\n    Source: local\n    Type: local\n    Enabled (User): true\n    Enabled (Workspace): false\n    Skills:\n      - demo\n    Commands:\n      - review\n", limits()).unwrap();
        assert_eq!(records[0].version.as_ref().unwrap().as_str(), "1.2.3");
        assert!(records[0].enabled_user);
        assert!(!records[0].enabled_workspace);
        assert_eq!(records[0].components.len(), 2);
        assert_eq!(decode_qwen_extension_list(b"Installed extensions:\n  demo\n    Version: 1.2.3\n    Enabled (User): false\n    Enabled (Workspace): true\n", "demo", "workspace", limits()), NativeResourceObservation::Present { scope: Some(CapabilityScope::Project), revision: Some(skilltap_core::domain::ResolvedRevision::Native(NativeId::new("1.2.3").unwrap())) });
    }

    #[test]
    fn qwen_conversion_uses_the_concrete_graph_and_does_not_trust_exit_status() {
        use skilltap_core::{
            domain::{Source, SourceKind, SourceLocator},
            managed_projection::ResolvedSourceCheckout,
            runtime::SystemFileSystem,
        };
        use skilltap_test_support::TempRoot;
        use std::fs;

        let root = TempRoot::new("skilltap-qwen-conversion").unwrap();
        fs::create_dir_all(root.join(".claude-plugin")).unwrap();
        fs::write(
            root.join(".claude-plugin/plugin.json"),
            br#"{"name":"fixture"}"#,
        )
        .unwrap();
        fs::create_dir_all(root.join("skills/demo")).unwrap();
        fs::write(
            root.join("skills/demo/SKILL.md"),
            b"---\nname: demo\ndescription: demo\n---\n",
        )
        .unwrap();
        fs::create_dir_all(root.join("commands")).unwrap();
        fs::write(root.join("commands/review.md"), b"review").unwrap();
        fs::create_dir_all(root.join("agents")).unwrap();
        fs::write(root.join("agents/release.md"), b"release").unwrap();
        fs::create_dir_all(root.join("hooks")).unwrap();
        fs::write(root.join("hooks/start.sh"), b"#!/bin/sh\n").unwrap();
        fs::write(
            root.join(".mcp.json"),
            br#"{"mcpServers":{"docs":{"command":"node"}}}"#,
        )
        .unwrap();
        let source = Source::new(
            SourceKind::Local,
            SourceLocator::new(root.path().to_string_lossy().into_owned()).unwrap(),
            None,
        )
        .unwrap();
        let checkout = ResolvedSourceCheckout::new(
            AbsolutePath::new(root.path().to_string_lossy().into_owned()).unwrap(),
            source,
            None,
        );
        let target = HarnessId::new("qwen").unwrap();
        let assessment = DISTRIBUTION
            .assess(&NativeDistributionContext {
                target: &target,
                scope: &Scope::Global,
                checkout: &checkout,
                requested_revision: None,
                filesystem: &SystemFileSystem,
                json_limits: limits(),
            })
            .unwrap()
            .unwrap();
        let ids = assessment
            .graph
            .components()
            .iter()
            .map(|(id, _)| id.as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            ids,
            [
                "agent:release.md",
                "command:review.md",
                "hook:start.sh",
                "mcp:docs",
                "skill:demo"
            ]
            .into_iter()
            .collect()
        );
        assert!(
            assessment
                .plan
                .included
                .iter()
                .any(|id| id.as_str() == "skill:demo")
        );
        assert!(
            assessment
                .plan
                .included
                .iter()
                .any(|id| id.as_str() == "command:review.md")
        );
        assert!(
            assessment
                .plan
                .omitted_optional
                .iter()
                .any(|id| id.as_str() == "agent:release.md")
        );
        assert!(
            assessment
                .plan
                .omitted_optional
                .iter()
                .any(|id| id.as_str() == "hook:start.sh")
        );
        assert!(
            assessment
                .plan
                .omitted_optional
                .iter()
                .any(|id| id.as_str() == "mcp:docs")
        );
        assert!(!assessment.plan.blocked());
    }

    #[test]
    fn qwen_mcp_status_is_fresh_session_evidence_and_not_json() {
        let status = decode_qwen_mcp_status(b"Configured MCP servers:\n\n  docs: node server.mjs (stdio) - Connected\n  pending: https://example.invalid/mcp - Pending approval\n", limits()).unwrap();
        assert_eq!(
            status.servers[&NativeId::new("docs").unwrap()],
            EffectiveServerHealth::Healthy
        );
        assert_eq!(
            status.servers[&NativeId::new("pending").unwrap()],
            EffectiveServerHealth::Unknown
        );
        assert!(matches!(
            PROBE.reload_semantics(),
            ReloadSemantics::InteractiveRequired { .. }
        ));
        assert!(
            PROBE
                .decode_mcp_status(br#"{"servers":{}}"#, limits())
                .is_err()
        );
    }
}
