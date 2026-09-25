use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
};

use skilltap_core::{
    domain::{
        AbsolutePath, CapabilityId, CapabilityProfileSelection, CapabilitySet, CapabilitySupport,
        HarnessId, NativeId, NativeVersion, Scope, ScopedCapabilitySets,
    },
    instructions::fingerprint_contents,
    mutation_authority::{ManagedDeclarationContract, ManagedSurfaceKind},
    runtime::{
        ExternalTreeLimits, ExternalTreeObserver, ExternalTreeRequest, ObservationRuntimeError,
        PlatformPaths, StrictJson, StrictJsonDecoder, SystemExternalTreeObserver,
    },
    storage::ManagedProjection,
};

use crate::{
    adapter_helpers,
    managed_projection::{ManagedProjectionContext, ManagedProjectionInput, ManagedProjectionPort},
    registry::{
        AdapterObservationPaths, DistributionSurface, HarnessAdapter, ObservationPathError,
        SkillProjectionPort, TargetIdentity,
    },
};

use super::configuration_constrained::common::{evidence, plan_skills, read_optional_file};
use super::configuration_constrained::{
    AuthenticationRequirement, PortableMcpServer, PortableRemoteTransport, SelectedPortablePlugin,
    load_selected_plugin,
};

const PROFILE_ID: &str = "agy-1-1-13";
const MARKETPLACE_DOCUMENTS: &[&str] = &[
    ".agents/plugins/marketplace.json",
    ".claude-plugin/marketplace.json",
];
static ADAPTER: AgyAdapter = AgyAdapter;
static SKILLS: AgySkillProjection = AgySkillProjection;
static PROJECTION: AgyManagedProjection = AgyManagedProjection;
static DECLARATION_CONTRACT: std::sync::LazyLock<ManagedDeclarationContract> =
    std::sync::LazyLock::new(|| {
        ManagedDeclarationContract::new([
            ManagedSurfaceKind::ManagedDocument,
            ManagedSurfaceKind::CompleteSkillTree,
        ])
        .expect("AGY declaration contract is non-empty")
    });

/// Antigravity CLI 1.1.13 is configuration-constrained: its documented load
/// surfaces are files beneath the global `~/.gemini/config/` root and the
/// project `.agents/` root, with no non-interactive effective-state observer
/// and no lifecycle contract skilltap can bind to a resolved checkout. All
/// mutation is therefore declaration-managed projection, and every write stays
/// beneath `~/.gemini/config/` even though other products share `~/.gemini`.
pub struct AgyAdapter;
pub struct AgySkillProjection;
pub struct AgyManagedProjection;

impl AgyAdapter {
    pub fn static_ref() -> &'static dyn HarnessAdapter {
        &ADAPTER
    }
}

fn capabilities() -> ScopedCapabilitySets {
    let cap = |id: &str, support: CapabilitySupport| {
        (
            CapabilityId::new(id).expect("AGY capability is valid"),
            support,
        )
    };
    let global = CapabilitySet::new([
        cap("harness.observe", CapabilitySupport::Supported),
        cap("managed.projection", CapabilitySupport::Unverified),
        cap("component.skill", CapabilitySupport::Supported),
        cap("component.mcp", CapabilitySupport::Unverified),
        cap("skill.install", CapabilitySupport::Supported),
        cap("skill.update", CapabilitySupport::Supported),
        cap("skill.remove", CapabilitySupport::Supported),
    ]);
    let project = CapabilitySet::new([
        cap("harness.observe", CapabilitySupport::Supported),
        cap("managed.projection", CapabilitySupport::Unverified),
        cap("component.skill", CapabilitySupport::Supported),
        // The installed AGY MCP contract documents only the global document
        // and plugin-bundled declarations, so standalone project MCP remains
        // unsupported rather than optimistically written.
        cap("component.mcp", CapabilitySupport::Unsupported),
        cap("skill.install", CapabilitySupport::Supported),
        cap("skill.update", CapabilitySupport::Supported),
        cap("skill.remove", CapabilitySupport::Supported),
    ]);
    ScopedCapabilitySets::new(global, project)
}

/// The documented AGY global customization root. Writes must never escape
/// this directory because sibling products share the `~/.gemini` parent.
fn global_config_root(
    paths: &PlatformPaths,
) -> Result<AbsolutePath, skilltap_core::domain::ValidationError> {
    AbsolutePath::new(format!("{}/.gemini/config", paths.home().as_str()))
}

impl HarnessAdapter for AgyAdapter {
    fn identity(&self) -> TargetIdentity {
        TargetIdentity {
            id: HarnessId::new("agy").expect("static harness id is valid"),
            display_name: "Antigravity CLI",
            default_binary: Some("agy"),
            distribution_surface: DistributionSurface::Managed,
            identity_boundary: crate::TargetIdentityBoundary::Executable,
        }
    }

    fn version_arguments(&self) -> Option<Vec<OsString>> {
        Some(vec![OsString::from("--version")])
    }

    fn decode_version(&self, stdout: &[u8]) -> Result<NativeVersion, crate::DetectionError> {
        // `agy --version` emits the bare version token, for example `1.1.13`.
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
        adapter_helpers::capability_profile(PROFILE_ID, capabilities())
    }

    fn observe(
        &self,
        paths: &PlatformPaths,
        scope: &Scope,
        limits: ExternalTreeLimits,
    ) -> Result<AdapterObservationPaths, ObservationPathError> {
        let (label, root) = match scope {
            Scope::Global => (
                "agy.config.skills",
                adapter_helpers::absolute_child(paths.home(), ".gemini/config/skills"),
            ),
            Scope::Project(project) => (
                "agy.agents.skills",
                adapter_helpers::absolute_child(project, ".agents/skills"),
            ),
        };
        let mut canonical = Vec::new();
        let mut project_entry_count = None;
        if let Some(root) = root {
            match SystemExternalTreeObserver.observe(&ExternalTreeRequest::new(root, limits)) {
                Ok(snapshot) => {
                    if matches!(scope, Scope::Project(_)) {
                        project_entry_count = Some(snapshot.entries().len());
                    }
                    canonical.push(crate::CanonicalObservation {
                        root: label.to_owned(),
                        snapshot,
                    });
                }
                Err(ObservationRuntimeError::TreeRootUnavailable) => {}
                Err(error) => return Err(ObservationPathError::Runtime(error)),
            }
        }
        let mut labels = Vec::new();
        if matches!(scope, Scope::Global) {
            let config_root = global_config_root(paths)?;
            if adapter_helpers::child_path_exists(&config_root, "mcp_config.json") {
                labels.push("agy.global.mcp");
            }
        }
        Ok(AdapterObservationPaths {
            canonical,
            project_entry_count,
            surface_labels: labels,
        })
    }

    fn skill_projection(&self) -> Option<&dyn SkillProjectionPort> {
        Some(&SKILLS)
    }
    fn managed_projection(&self) -> Option<&dyn ManagedProjectionPort> {
        Some(&PROJECTION)
    }
    fn managed_declaration_contract(
        &self,
        _scope: skilltap_core::domain::CapabilityScope,
    ) -> Option<&'static ManagedDeclarationContract> {
        Some(&DECLARATION_CONTRACT)
    }
    fn native_root(&self, paths: &PlatformPaths) -> Option<AbsolutePath> {
        global_config_root(paths).ok()
    }
}

impl SkillProjectionPort for AgySkillProjection {
    fn destination(&self, paths: &PlatformPaths, scope: &Scope) -> Option<AbsolutePath> {
        match scope {
            Scope::Global => adapter_helpers::absolute_child(paths.home(), ".gemini/config/skills"),
            Scope::Project(project) => adapter_helpers::absolute_child(project, ".agents/skills"),
        }
    }
}

impl ManagedProjectionPort for AgyManagedProjection {
    fn plan(
        &self,
        context: &ManagedProjectionContext<'_>,
    ) -> Result<
        skilltap_core::managed_projection::ManagedProjectionPlan,
        skilltap_core::managed_projection::ManagedProjectionError,
    > {
        match context.resource_kind {
            skilltap_core::domain::ResourceKind::Marketplace => Ok(Default::default()),
            skilltap_core::domain::ResourceKind::Plugin => plan_plugin(context),
            _ => Err(
                skilltap_core::managed_projection::ManagedProjectionError::UnsupportedResourceKind,
            ),
        }
    }
}

fn plan_plugin(
    context: &ManagedProjectionContext<'_>,
) -> Result<
    skilltap_core::managed_projection::ManagedProjectionPlan,
    skilltap_core::managed_projection::ManagedProjectionError,
> {
    let plugin = match &context.input {
        ManagedProjectionInput::Apply { checkout } => Some(load_selected_plugin(
            context,
            checkout,
            MARKETPLACE_DOCUMENTS,
        )?),
        ManagedProjectionInput::Remove => None,
    };
    let skill_root = AbsolutePath::new(match context.scope {
        Scope::Global => format!("{}/.gemini/config/skills", context.paths.home().as_str()),
        Scope::Project(project) => format!("{}/.agents/skills", project.as_str()),
    })
    .map_err(|_| destination_error())?;
    let (trees, mut current_parts, mut desired_parts, mut manifest) =
        plan_skills(&skill_root, context, plugin.as_ref())?;
    let (mcp_write, mcp_manifest) = plan_mcp(
        context,
        plugin.as_ref(),
        (&mut current_parts, &mut desired_parts),
    )?;
    manifest.extend(mcp_manifest);
    manifest.sort();
    manifest.dedup();
    if trees.is_empty() && mcp_write.is_none() {
        return Err(
            skilltap_core::managed_projection::ManagedProjectionError::Other {
                code: "agy_managed_plugin_unsupported",
                summary: "The plugin has no faithful AGY skill or MCP projection for this scope.",
            },
        );
    }
    let removal = matches!(context.input, ManagedProjectionInput::Remove);
    if removal {
        manifest.clear();
    }
    Ok(skilltap_core::managed_projection::ManagedProjectionPlan {
        trees,
        files: mcp_write.into_iter().collect(),
        manifest,
        current_fingerprint: (!current_parts.is_empty())
            .then(|| fingerprint_contents(&current_parts)),
        desired_fingerprint: (!removal && !desired_parts.is_empty())
            .then(|| fingerprint_contents(&desired_parts)),
    })
}

fn plan_mcp(
    context: &ManagedProjectionContext<'_>,
    plugin: Option<&SelectedPortablePlugin>,
    fingerprints: (&mut Vec<u8>, &mut Vec<u8>),
) -> Result<
    (
        Option<skilltap_core::managed_projection::ManagedFileWrite>,
        Vec<ManagedProjection>,
    ),
    skilltap_core::managed_projection::ManagedProjectionError,
> {
    let Some(config_root) = (matches!(context.scope, Scope::Global))
        .then(|| global_config_root(context.paths))
        .transpose()
        .map_err(|_| destination_error())?
    else {
        let mut omitted = Vec::new();
        if let Some(plugin) = plugin {
            for id in plugin.mcp.keys() {
                if is_required_mcp(&plugin.declarations, id.as_str()) {
                    return Err(
                        skilltap_core::managed_projection::ManagedProjectionError::RequiredUnsupported,
                    );
                }
                omitted.push(ManagedProjection::Omitted {
                    id: mcp_component(id)?,
                    consequence: evidence("unsupported_scope"),
                });
            }
        }
        return Ok((None, omitted));
    };
    let destination = skilltap_core::domain::RelativeArtifactPath::new("mcp_config.json")
        .map_err(|_| mcp_invalid("The AGY MCP path is invalid."))?;
    let expected = read_optional_file(
        context.filesystem,
        &config_root,
        &destination,
        context.json_limits.bytes(),
        "The AGY MCP document could not be read safely.",
    )?;
    let mut document = match expected.as_deref() {
        Some(bytes) => StrictJson
            .decode(bytes, context.json_limits)
            .map_err(|_| mcp_invalid("The AGY MCP document is invalid JSON."))?
            .value()
            .as_object()
            .cloned()
            .ok_or_else(|| mcp_invalid("The AGY MCP document must be an object."))?,
        None => serde_json::Map::new(),
    };
    let current_servers = match document.get("mcpServers") {
        None => BTreeMap::new(),
        Some(value) => value
            .as_object()
            .cloned()
            .ok_or_else(|| mcp_invalid("The AGY mcpServers member must be an object."))?
            .into_iter()
            .collect(),
    };
    let mut names = BTreeSet::new();
    if let Some(plugin) = plugin {
        names.extend(plugin.mcp.keys().cloned());
    }
    names.extend(context.prior.iter().filter_map(|projection| {
        if let ManagedProjection::Mcp { id, .. } = projection {
            Some(id.clone())
        } else {
            None
        }
    }));
    let mut manifest = Vec::new();
    let mut touched = false;
    for id in names {
        let name = id.as_str();
        let current = current_servers.get(name).cloned();
        let prior = context
            .prior
            .iter()
            .find_map(|projection| match projection {
                ManagedProjection::Mcp {
                    id: prior_id,
                    fingerprint,
                } if prior_id == &id => Some(fingerprint),
                _ => None,
            });
        if let Some(expected_fingerprint) = prior {
            if current.as_ref().map(json_fingerprint).as_ref() != Some(expected_fingerprint) {
                return Err(
                    skilltap_core::managed_projection::ManagedProjectionError::Drifted {
                        detail: "An owned AGY MCP server is missing or was replaced.",
                    },
                );
            }
            if let Some(current) = &current {
                fingerprints.0.extend(json_fingerprint_bytes(current));
            }
        } else if current.is_some() && !matches!(context.input, ManagedProjectionInput::Remove) {
            return Err(skilltap_core::managed_projection::ManagedProjectionError::McpConflict);
        }
        let source = plugin.and_then(|plugin| plugin.mcp.get(&id));
        let Some(source) = source else {
            if prior.is_some()
                && document
                    .get_mut("mcpServers")
                    .and_then(serde_json::Value::as_object_mut)
                    .is_some_and(|servers| servers.remove(name).is_some())
            {
                touched = true;
            }
            continue;
        };
        let mapped = match map_agy_server(source) {
            Ok(mapped) => mapped,
            Err(_) if is_required_mcp(&plugin.expect("source exists").declarations, name) => {
                return Err(
                    skilltap_core::managed_projection::ManagedProjectionError::RequiredUnsupported,
                );
            }
            Err(_) => {
                manifest.push(ManagedProjection::Omitted {
                    id: mcp_component(&id)?,
                    consequence: evidence("unsupported_optional_component_omitted"),
                });
                continue;
            }
        };
        let mapped = merge_object(current.as_ref(), mapped);
        touched |= current.as_ref() != Some(&mapped);
        fingerprints.1.extend(json_fingerprint_bytes(&mapped));
        manifest.push(ManagedProjection::Mcp {
            id: id.clone(),
            fingerprint: json_fingerprint(&mapped),
        });
        document
            .entry("mcpServers".to_owned())
            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()))
            .as_object_mut()
            .ok_or_else(|| mcp_invalid("The AGY mcpServers member must be an object."))?
            .insert(name.to_owned(), mapped);
    }
    if !touched {
        return Ok((None, manifest));
    }
    let mut bytes = serde_json::to_vec_pretty(&serde_json::Value::Object(document))
        .map_err(|_| mcp_invalid("The AGY MCP document could not be encoded."))?;
    bytes.push(b'\n');
    Ok((
        Some(skilltap_core::managed_projection::ManagedFileWrite {
            root: config_root,
            destination,
            expected,
            desired: Some(bytes),
        }),
        manifest,
    ))
}

/// Map a portable server to the exact AGY 1.1.13 `mcp_config.json` schema.
/// The documented shapes are stdio servers with `command`, `args`, and `env`,
/// and SSE servers addressed by `serverUrl`. Every other transport, field, or
/// authentication shape is unproven and must be omitted rather than written.
fn map_agy_server(source: &PortableMcpServer) -> Result<serde_json::Value, ()> {
    match source {
        PortableMcpServer::Stdio {
            command,
            args,
            environment,
            cwd,
            enabled,
            timeout_ms,
            tools,
        } => {
            if !*enabled || cwd.is_some() || timeout_ms.is_some() || tools.is_some() {
                return Err(());
            }
            let mut value = serde_json::Map::from_iter([(
                "command".to_owned(),
                serde_json::Value::String(command.clone()),
            )]);
            if !args.is_empty() {
                value.insert(
                    "args".to_owned(),
                    serde_json::Value::Array(
                        args.iter()
                            .cloned()
                            .map(serde_json::Value::String)
                            .collect(),
                    ),
                );
            }
            if !environment.is_empty() {
                value.insert(
                    "env".to_owned(),
                    serde_json::Value::Object(
                        environment
                            .iter()
                            .map(|(key, value)| {
                                (key.clone(), serde_json::Value::String(value.clone()))
                            })
                            .collect(),
                    ),
                );
            }
            Ok(serde_json::Value::Object(value))
        }
        PortableMcpServer::Remote {
            transport,
            url,
            headers,
            authentication,
            enabled,
            timeout_ms,
            tools,
        } => {
            if !matches!(transport, PortableRemoteTransport::Sse)
                || !matches!(authentication, AuthenticationRequirement::None)
                || !headers.is_empty()
                || !*enabled
                || timeout_ms.is_some()
                || tools.is_some()
            {
                return Err(());
            }
            Ok(serde_json::Value::Object(serde_json::Map::from_iter([(
                "serverUrl".to_owned(),
                serde_json::Value::String(url.clone()),
            )])))
        }
    }
}

fn merge_object(
    current: Option<&serde_json::Value>,
    mapped: serde_json::Value,
) -> serde_json::Value {
    let Some(current) = current.and_then(serde_json::Value::as_object) else {
        return mapped;
    };
    let Some(mut mapped_object) = mapped.as_object().cloned() else {
        return mapped;
    };
    for (key, value) in current {
        mapped_object
            .entry(key.clone())
            .or_insert_with(|| value.clone());
    }
    serde_json::Value::Object(mapped_object)
}

fn is_required_mcp(
    declarations: &[skilltap_core::plugin_graph::ComponentDeclaration],
    name: &str,
) -> bool {
    declarations.iter().any(|declaration| {
        declaration.kind == skilltap_core::domain::ComponentKind::McpServer
            && declaration.declared_name.as_deref() == Some(name)
            && declaration.requiredness == skilltap_core::domain::ComponentRequiredness::Required
    })
}

fn mcp_component(
    id: &NativeId,
) -> Result<
    skilltap_core::domain::ComponentId,
    skilltap_core::managed_projection::ManagedProjectionError,
> {
    skilltap_core::domain::ComponentId::new(format!("mcp:{}", id.as_str()))
        .map_err(|_| mcp_invalid("The AGY MCP component id is invalid."))
}
fn destination_error() -> skilltap_core::managed_projection::ManagedProjectionError {
    skilltap_core::managed_projection::ManagedProjectionError::Other {
        code: "agy_managed_destination_invalid",
        summary: "The AGY managed destination is invalid.",
    }
}
fn mcp_invalid(detail: &'static str) -> skilltap_core::managed_projection::ManagedProjectionError {
    skilltap_core::managed_projection::ManagedProjectionError::McpInvalid { detail }
}
fn json_fingerprint(value: &serde_json::Value) -> skilltap_core::domain::Fingerprint {
    fingerprint_contents(&json_fingerprint_bytes(value))
}
fn json_fingerprint_bytes(value: &serde_json::Value) -> Vec<u8> {
    serde_json::to_vec(value).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use skilltap_core::domain::CapabilityScope;

    #[test]
    fn agy_contract_is_global_mcp_only_across_versions_and_has_no_probe() {
        let adapter = AgyAdapter;
        assert_eq!(
            adapter.decode_version(b"1.1.13\n").unwrap().as_str(),
            "1.1.13"
        );
        assert!(adapter.decode_version(b"agy 1.1.13\n").is_err());
        let profile = adapter.select_profile(&NativeVersion::new("1.1.13").unwrap());
        assert_eq!(profile.profile_id().unwrap().as_str(), PROFILE_ID);
        let sets = profile.mutation_capabilities().unwrap();
        assert_eq!(
            sets.for_scope_kind(CapabilityScope::Global)
                .support(&CapabilityId::new("component.mcp").unwrap()),
            Some(CapabilitySupport::Unverified)
        );
        assert_eq!(
            sets.for_scope_kind(CapabilityScope::Project)
                .support(&CapabilityId::new("component.mcp").unwrap()),
            Some(CapabilitySupport::Unsupported)
        );
        let adjacent = adapter.select_profile(&NativeVersion::new("1.1.14").unwrap());
        assert_eq!(adjacent.profile_id(), profile.profile_id());
        assert_eq!(
            adjacent.mutation_capabilities(),
            profile.mutation_capabilities()
        );
        assert!(adapter.effective_state_probe().is_none());
        assert!(adapter.native_lifecycle().is_none());
    }

    #[test]
    fn agy_codec_accepts_only_documented_stdio_and_sse_shapes() {
        let stdio = PortableMcpServer::Stdio {
            command: "node".to_owned(),
            args: vec!["server.mjs".to_owned()],
            environment: BTreeMap::from([("TOKEN".to_owned(), "${TOKEN}".to_owned())]),
            cwd: None,
            enabled: true,
            timeout_ms: None,
            tools: None,
        };
        assert_eq!(
            map_agy_server(&stdio).unwrap(),
            serde_json::json!({
                "command": "node",
                "args": ["server.mjs"],
                "env": {"TOKEN": "${TOKEN}"},
            })
        );
        let sse = PortableMcpServer::Remote {
            transport: PortableRemoteTransport::Sse,
            url: "https://example.invalid/sse".to_owned(),
            headers: BTreeMap::new(),
            authentication: AuthenticationRequirement::None,
            enabled: true,
            timeout_ms: None,
            tools: None,
        };
        assert_eq!(
            map_agy_server(&sse).unwrap(),
            serde_json::json!({"serverUrl": "https://example.invalid/sse"})
        );
    }

    #[test]
    fn agy_codec_rejects_unproven_transports_fields_and_authentication() {
        let stdio = |cwd: Option<String>, enabled: bool| PortableMcpServer::Stdio {
            command: "node".to_owned(),
            args: Vec::new(),
            environment: BTreeMap::new(),
            cwd,
            enabled,
            timeout_ms: None,
            tools: None,
        };
        assert!(map_agy_server(&stdio(Some("/opt/demo".to_owned()), true)).is_err());
        assert!(map_agy_server(&stdio(None, false)).is_err());
        for (transport, authentication, headers) in [
            (
                PortableRemoteTransport::Http,
                AuthenticationRequirement::None,
                BTreeMap::new(),
            ),
            (
                PortableRemoteTransport::StreamableHttp,
                AuthenticationRequirement::None,
                BTreeMap::new(),
            ),
            (
                PortableRemoteTransport::Sse,
                AuthenticationRequirement::OAuth,
                BTreeMap::new(),
            ),
            (
                PortableRemoteTransport::Sse,
                AuthenticationRequirement::StaticReferences,
                BTreeMap::from([("Authorization".to_owned(), "${TOKEN}".to_owned())]),
            ),
        ] {
            let remote = PortableMcpServer::Remote {
                transport,
                url: "https://example.invalid/sse".to_owned(),
                headers,
                authentication,
                enabled: true,
                timeout_ms: None,
                tools: None,
            };
            assert!(map_agy_server(&remote).is_err());
        }
    }
}
