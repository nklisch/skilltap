use std::collections::{BTreeMap, BTreeSet};

use skilltap_core::{
    domain::{
        AbsolutePath, ComponentId, ComponentKind, ComponentRequiredness, NativeId,
        RelativeArtifactPath, ResourceKind,
    },
    instructions::fingerprint_contents,
    managed_projection::{ManagedFileWrite, ManagedProjectionError, ManagedProjectionPlan},
    plugin_graph::ComponentDeclaration,
    runtime::{ConfinedFileSystem, JsonLimits, RuntimeError, StrictJson, StrictJsonDecoder},
    storage::{ArtifactTree, ManagedProjection},
};

use crate::{
    managed_codex_project::ManagedCodexCatalog,
    managed_projection::{ManagedProjectionContext, ManagedProjectionInput, ManagedProjectionPort},
};

use super::super::{
    configuration_constrained::{
        AuthenticationRequirement, PortableMcpServer, PortableRemoteTransport,
        SelectedPortablePlugin,
        common::{evidence, plan_skills, read_optional_file, tree_limits},
    },
    file_managed::{CompleteSourcePlugin, read_complete_source_plugin},
};

const MARKETPLACE_DOCUMENTS: &[&str] = &[
    ".agents/plugins/marketplace.json",
    ".claude-plugin/marketplace.json",
];
const MCP_SOURCE_DOCUMENTS: &[&str] = &[
    ".mcp.json",
    ".codex-plugin/mcp.json",
    ".claude-plugin/mcp.json",
    "mcp.json",
];

static PROJECTION: AmpManagedProjection = AmpManagedProjection;

pub struct AmpManagedProjection;

impl AmpManagedProjection {
    pub fn static_ref() -> &'static dyn ManagedProjectionPort {
        &PROJECTION
    }
}

impl ManagedProjectionPort for AmpManagedProjection {
    fn plan(
        &self,
        context: &ManagedProjectionContext<'_>,
    ) -> Result<ManagedProjectionPlan, ManagedProjectionError> {
        match context.resource_kind {
            ResourceKind::Marketplace => Ok(ManagedProjectionPlan::default()),
            ResourceKind::Plugin => plan_plugin(context),
            _ => Err(ManagedProjectionError::UnsupportedResourceKind),
        }
    }
}

fn plan_plugin(
    context: &ManagedProjectionContext<'_>,
) -> Result<ManagedProjectionPlan, ManagedProjectionError> {
    let plugin = match &context.input {
        ManagedProjectionInput::Apply { checkout } => {
            Some(read_selected_plugin(context, checkout)?)
        }
        ManagedProjectionInput::Remove => None,
    };
    let (skill_root, config_root, config_destination, alternate_destination) =
        destination_paths(context)?;
    if let Some(plugin) = &plugin {
        reject_skill_precedence_conflicts(context, plugin)?;
    }
    let (trees, mut current_parts, mut desired_parts, mut manifest) =
        plan_skills(&skill_root, context, plugin.as_ref())?;
    let (mcp_file, mcp_manifest) = plan_mcp(
        &config_root,
        &config_destination,
        &alternate_destination,
        context,
        plugin.as_ref(),
        (&mut current_parts, &mut desired_parts),
    )?;
    manifest.extend(mcp_manifest);
    manifest.sort();
    manifest.dedup();
    if trees.is_empty() && mcp_file.is_none() {
        return Err(ManagedProjectionError::Other {
            code: "amp_managed_plugin_unsupported",
            summary: "The plugin has no faithful Amp skill or MCP projection.",
        });
    }
    let removal = matches!(context.input, ManagedProjectionInput::Remove);
    if removal {
        manifest.clear();
    }
    Ok(ManagedProjectionPlan {
        trees,
        files: mcp_file.into_iter().collect(),
        manifest,
        current_fingerprint: (!current_parts.is_empty())
            .then(|| fingerprint_contents(&current_parts)),
        desired_fingerprint: (!removal && !desired_parts.is_empty())
            .then(|| fingerprint_contents(&desired_parts)),
    })
}

fn destination_paths(
    context: &ManagedProjectionContext<'_>,
) -> Result<
    (
        AbsolutePath,
        AbsolutePath,
        RelativeArtifactPath,
        RelativeArtifactPath,
    ),
    ManagedProjectionError,
> {
    let skill_root = match context.scope {
        skilltap_core::domain::Scope::Global => {
            AbsolutePath::new(format!("{}/.agents/skills", context.paths.home().as_str()))
        }
        skilltap_core::domain::Scope::Project(project) => {
            AbsolutePath::new(format!("{}/.agents/skills", project.as_str()))
        }
    }
    .map_err(|_| destination_error())?;
    let (config_root, config_destination, alternate_destination) = match context.scope {
        skilltap_core::domain::Scope::Global => (
            context.paths.config_home().clone(),
            relative("amp/settings.json")?,
            relative("amp/settings.jsonc")?,
        ),
        skilltap_core::domain::Scope::Project(project) => (
            project.clone(),
            relative(".amp/settings.json")?,
            relative(".amp/settings.jsonc")?,
        ),
    };
    Ok((
        skill_root,
        config_root,
        config_destination,
        alternate_destination,
    ))
}

fn read_selected_plugin(
    context: &ManagedProjectionContext<'_>,
    checkout: &skilltap_core::managed_projection::ResolvedSourceCheckout,
) -> Result<SelectedPortablePlugin, ManagedProjectionError> {
    let selector = skilltap_core::marketplace::PluginSelector::parse(context.request.name.as_str())
        .map_err(|_| ManagedProjectionError::PluginSourceInvalid {
            detail: "The selected Amp plugin selector is invalid.",
        })?;
    let catalog =
        read_marketplace_catalog(context.filesystem, checkout.root(), context.json_limits)?;
    let plugin_root = catalog
        .plugin_source(selector.plugin(), checkout.root())
        .map_err(|_| ManagedProjectionError::PluginSourceInvalid {
            detail: "The selected Amp plugin source is not a contained local marketplace entry.",
        })?;
    let complete =
        read_complete_source_plugin(&plugin_root, checkout.source(), context.json_limits)?;
    validate_skill_local_mcp(&complete.tree, context.json_limits)?;
    let mcp = normalize_root_mcp(&complete, context.json_limits)?;
    Ok(SelectedPortablePlugin {
        tree: complete.tree,
        declarations: complete.declarations,
        mcp,
    })
}

fn read_marketplace_catalog(
    filesystem: &dyn ConfinedFileSystem,
    root: &AbsolutePath,
    limits: JsonLimits,
) -> Result<ManagedCodexCatalog, ManagedProjectionError> {
    for path in MARKETPLACE_DOCUMENTS {
        let destination = relative(path)?;
        let bytes = filesystem
            .read_regular_bounded_no_follow(root, &destination, limits.bytes())
            .map_err(|_| ManagedProjectionError::CatalogInvalid {
                detail: "The selected Amp marketplace document could not be read safely.",
            })?;
        if let Some(bytes) = bytes {
            return ManagedCodexCatalog::parse(&bytes, limits).map_err(|_| {
                ManagedProjectionError::CatalogInvalid {
                    detail: "The selected Amp marketplace document is invalid.",
                }
            });
        }
    }
    Err(ManagedProjectionError::CatalogMissing)
}

fn normalize_root_mcp(
    plugin: &CompleteSourcePlugin,
    limits: JsonLimits,
) -> Result<BTreeMap<NativeId, PortableMcpServer>, ManagedProjectionError> {
    let Some(file) = MCP_SOURCE_DOCUMENTS.iter().find_map(|path| {
        plugin
            .tree
            .files()
            .get(&RelativeArtifactPath::new(*path).ok()?)
    }) else {
        return Ok(BTreeMap::new());
    };
    let decoded = StrictJson
        .decode(file.contents(), limits)
        .map_err(|_| mcp_invalid("The selected Amp MCP declaration is invalid JSON."))?;
    let servers = decoded
        .value()
        .get("mcpServers")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| mcp_invalid("The selected Amp MCP declaration has no mcpServers object."))?;
    servers
        .iter()
        .map(|(name, value)| {
            let id = NativeId::new(name)
                .map_err(|_| mcp_invalid("An Amp MCP server name is invalid."))?;
            Ok((id, normalize_amp_server(value)?))
        })
        .collect()
}

fn normalize_amp_server(
    value: &serde_json::Value,
) -> Result<PortableMcpServer, ManagedProjectionError> {
    let object = value
        .as_object()
        .ok_or_else(|| mcp_invalid("An Amp MCP server must be an object."))?;
    let command = object.get("command").and_then(serde_json::Value::as_str);
    let url = object
        .get("url")
        .or_else(|| object.get("uri"))
        .and_then(serde_json::Value::as_str);
    if command.is_some() == url.is_some() {
        return Err(mcp_invalid("An Amp MCP server must select one transport."));
    }
    let enabled = object
        .get("enabled")
        .and_then(serde_json::Value::as_bool)
        .or_else(|| {
            object
                .get("disabled")
                .and_then(serde_json::Value::as_bool)
                .map(|value| !value)
        })
        .unwrap_or(true);
    let args = object
        .get("args")
        .map(parse_strings)
        .transpose()?
        .unwrap_or_default();
    let env = object
        .get("env")
        .or_else(|| object.get("environment"))
        .map(parse_references)
        .transpose()?
        .unwrap_or_default();
    if let Some(command) = command {
        reject_cwd_relative(command)?;
        for arg in &args {
            reject_cwd_relative(arg)?;
        }
        if object.contains_key("cwd") {
            return Err(mcp_invalid(
                "Amp MCP cwd semantics are not safely portable.",
            ));
        }
        return Ok(PortableMcpServer::Stdio {
            command: command.to_owned(),
            args,
            environment: env,
            cwd: None,
            enabled,
            timeout_ms: None,
            tools: None,
        });
    }
    let url = url.ok_or_else(|| mcp_invalid("An Amp MCP URL is empty."))?;
    if url.starts_with('.') || !url.contains("://") {
        return Err(mcp_invalid("An Amp MCP URL is not an absolute URL."));
    }
    let headers = object
        .get("headers")
        .map(parse_references)
        .transpose()?
        .unwrap_or_default();
    let authentication = if object
        .get("auth")
        .and_then(serde_json::Value::as_object)
        .and_then(|auth| auth.get("type"))
        .and_then(serde_json::Value::as_str)
        == Some("oauth")
    {
        AuthenticationRequirement::OAuth
    } else if headers.is_empty() {
        AuthenticationRequirement::None
    } else {
        AuthenticationRequirement::StaticReferences
    };
    Ok(PortableMcpServer::Remote {
        transport: PortableRemoteTransport::Http,
        url: url.to_owned(),
        headers,
        authentication,
        enabled,
        timeout_ms: None,
        tools: None,
    })
}

fn validate_skill_local_mcp(
    tree: &ArtifactTree,
    limits: JsonLimits,
) -> Result<(), ManagedProjectionError> {
    for (path, file) in tree.files() {
        if !path.as_str().starts_with("skills/") || !path.as_str().ends_with("/mcp.json") {
            continue;
        }
        let decoded = StrictJson
            .decode(file.contents(), limits)
            .map_err(|_| mcp_invalid("A skill-local Amp mcp.json is invalid JSON."))?;
        let servers = decoded
            .value()
            .as_object()
            .ok_or_else(|| mcp_invalid("A skill-local Amp mcp.json must be an object."))?;
        for value in servers.values() {
            let server = value
                .as_object()
                .ok_or_else(|| mcp_invalid("A skill-local Amp server must be an object."))?;
            if server.contains_key("cwd") {
                return Err(mcp_invalid(
                    "A skill-local Amp MCP cwd assumption cannot be rewritten safely.",
                ));
            }
            if let Some(command) = server.get("command").and_then(serde_json::Value::as_str) {
                reject_cwd_relative(command)?;
            }
            if let Some(args) = server.get("args") {
                for arg in parse_strings(args)? {
                    reject_cwd_relative(&arg)?;
                }
            }
        }
    }
    Ok(())
}

fn reject_cwd_relative(value: &str) -> Result<(), ManagedProjectionError> {
    if value == "." || value == ".." || value.starts_with("./") || value.starts_with("../") {
        return Err(mcp_invalid(
            "An Amp MCP command assumes a skill-relative cwd and cannot be rewritten safely.",
        ));
    }
    Ok(())
}

fn parse_strings(value: &serde_json::Value) -> Result<Vec<String>, ManagedProjectionError> {
    value
        .as_array()
        .ok_or_else(|| mcp_invalid("An Amp MCP argument list is invalid."))?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| mcp_invalid("An Amp MCP argument is not a string."))
        })
        .collect()
}

fn parse_references(
    value: &serde_json::Value,
) -> Result<BTreeMap<String, String>, ManagedProjectionError> {
    let object = value
        .as_object()
        .ok_or_else(|| mcp_invalid("An Amp MCP reference map is invalid."))?;
    object
        .iter()
        .map(|(key, value)| {
            let value = value
                .as_str()
                .filter(|value| value.starts_with('$'))
                .map(str::to_owned)
                .ok_or_else(|| {
                    mcp_invalid("Literal Amp MCP credential material is unsupported.")
                })?;
            Ok((key.clone(), value))
        })
        .collect()
}

fn reject_skill_precedence_conflicts(
    context: &ManagedProjectionContext<'_>,
    plugin: &SelectedPortablePlugin,
) -> Result<(), ManagedProjectionError> {
    let names = plugin
        .declarations
        .iter()
        .filter(|declaration| declaration.kind == ComponentKind::Skill)
        .filter_map(|declaration| declaration.declared_name.as_deref())
        .collect::<BTreeSet<_>>();
    let higher_roots = match context.scope {
        skilltap_core::domain::Scope::Global => {
            vec![(context.paths.home().clone(), ".config/agents/skills")]
        }
        skilltap_core::domain::Scope::Project(_) => vec![
            (context.paths.home().clone(), ".config/agents/skills"),
            (context.paths.home().clone(), ".agents/skills"),
            (context.paths.config_home().clone(), "amp/skills"),
        ],
    };
    for name in names {
        let path = RelativeArtifactPath::new(name.to_string()).map_err(|_| {
            ManagedProjectionError::PluginMissing {
                detail: "An Amp skill name is not a safe destination.",
            }
        })?;
        for (root, relative) in &higher_roots {
            let root = AbsolutePath::new(format!("{}/{}", root.as_str(), relative))
                .map_err(|_| destination_error())?;
            match context
                .filesystem
                .load_tree_bounded_no_follow(&root, &path, tree_limits())
            {
                Ok(_) => {
                    return Err(ManagedProjectionError::Other {
                        code: "amp_skill_precedence_conflict",
                        summary: "A higher-precedence Amp skill shadows the managed skill destination.",
                    });
                }
                Err(RuntimeError::FileSystem { source, .. })
                    if source.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => {
                    return Err(ManagedProjectionError::PluginUnreadable {
                        detail: "A higher-precedence Amp skill root could not be inspected safely.",
                    });
                }
            }
        }
    }
    Ok(())
}

fn plan_mcp(
    config_root: &AbsolutePath,
    destination: &RelativeArtifactPath,
    alternate_destination: &RelativeArtifactPath,
    context: &ManagedProjectionContext<'_>,
    plugin: Option<&SelectedPortablePlugin>,
    fingerprints: (&mut Vec<u8>, &mut Vec<u8>),
) -> Result<(Option<ManagedFileWrite>, Vec<ManagedProjection>), ManagedProjectionError> {
    let alternate = read_optional_file(
        context.filesystem,
        config_root,
        alternate_destination,
        context.json_limits.bytes(),
        "The Amp JSONC settings alternative could not be read safely.",
    )?;
    if alternate.is_some() {
        return Err(ManagedProjectionError::Other {
            code: "amp_settings_jsonc_conflict",
            summary: "An Amp settings.jsonc alternative exists; skilltap will not choose between JSON and JSONC.",
        });
    }
    let expected = read_optional_file(
        context.filesystem,
        config_root,
        destination,
        context.json_limits.bytes(),
        "The Amp settings document could not be read safely.",
    )?;
    let mut document = match expected.as_deref() {
        Some(bytes) => StrictJson
            .decode(bytes, context.json_limits)
            .map_err(|_| mcp_invalid("The Amp settings document is invalid JSON."))?
            .value()
            .as_object()
            .cloned()
            .ok_or_else(|| mcp_invalid("The Amp settings document must be an object."))?,
        None => serde_json::Map::new(),
    };
    let current_servers = document
        .get("amp.mcpServers")
        .map(|value| {
            value
                .as_object()
                .cloned()
                .ok_or_else(|| mcp_invalid("Amp amp.mcpServers must be an object."))
        })
        .transpose()?
        .unwrap_or_default();

    if let skilltap_core::domain::Scope::Project(_) = context.scope {
        let global_root = context.paths.config_home().clone();
        let global_destination = relative("amp/settings.json")?;
        let global = read_optional_file(
            context.filesystem,
            &global_root,
            &global_destination,
            context.json_limits.bytes(),
            "The Amp global settings document could not be read safely.",
        )?;
        if let Some(global) = global {
            let global = StrictJson
                .decode(&global, context.json_limits)
                .map_err(|_| mcp_invalid("The Amp global settings document is invalid JSON."))?;
            let global_servers = global
                .value()
                .as_object()
                .and_then(|object| object.get("amp.mcpServers"))
                .and_then(serde_json::Value::as_object)
                .ok_or_else(|| mcp_invalid("Amp global amp.mcpServers must be an object."))?;
            let source_names = plugin
                .map(|plugin| {
                    plugin
                        .mcp
                        .keys()
                        .map(NativeId::as_str)
                        .collect::<BTreeSet<_>>()
                })
                .unwrap_or_default();
            if global_servers.keys().any(|name| {
                current_servers.contains_key(name) || source_names.contains(name.as_str())
            }) {
                return Err(ManagedProjectionError::McpConflict);
            }
        }
    }

    let mut names = BTreeSet::new();
    if let Some(plugin) = plugin {
        names.extend(plugin.mcp.keys().cloned());
    }
    names.extend(
        context
            .prior
            .iter()
            .filter_map(|projection| match projection {
                ManagedProjection::Mcp { id, .. } => Some(id.clone()),
                _ => None,
            }),
    );
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
                return Err(ManagedProjectionError::Drifted {
                    detail: "An owned Amp MCP server is missing or was replaced.",
                });
            }
            if let Some(current) = &current {
                fingerprints.0.extend(json_fingerprint_bytes(current));
            }
        } else if current.is_some() && !matches!(context.input, ManagedProjectionInput::Remove) {
            return Err(ManagedProjectionError::McpConflict);
        }
        let source = plugin.and_then(|plugin| plugin.mcp.get(&id));
        let Some(source) = source else {
            if prior.is_some()
                && document
                    .get_mut("amp.mcpServers")
                    .and_then(serde_json::Value::as_object_mut)
                    .is_some_and(|servers| servers.remove(name).is_some())
            {
                touched = true;
            }
            continue;
        };
        let mapped = match map_amp_server(source) {
            Ok(value) => value,
            Err(_) if is_required_mcp(&plugin.expect("source exists").declarations, name) => {
                return Err(ManagedProjectionError::RequiredUnsupported);
            }
            Err(_) => {
                manifest.push(ManagedProjection::Omitted {
                    id: component_id(&id)?,
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
            .entry("amp.mcpServers".to_owned())
            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()))
            .as_object_mut()
            .ok_or_else(|| mcp_invalid("Amp amp.mcpServers must be an object."))?
            .insert(name.to_owned(), mapped);
    }
    if !touched {
        return Ok((None, manifest));
    }
    let mut bytes = serde_json::to_vec_pretty(&serde_json::Value::Object(document))
        .map_err(|_| mcp_invalid("The Amp settings document could not be encoded."))?;
    bytes.push(b'\n');
    Ok((
        Some(ManagedFileWrite {
            root: config_root.clone(),
            destination: destination.clone(),
            expected,
            desired: Some(bytes),
        }),
        manifest,
    ))
}

fn map_amp_server(source: &PortableMcpServer) -> Result<serde_json::Value, ()> {
    match source {
        PortableMcpServer::Stdio {
            command,
            args,
            environment,
            cwd,
            enabled,
            ..
        } => {
            if cwd.is_some() {
                return Err(());
            }
            Ok(serde_json::json!({
                "command": command,
                "args": args,
                "env": environment,
                "disabled": !enabled,
            }))
        }
        PortableMcpServer::Remote {
            transport,
            url,
            headers,
            authentication,
            enabled,
            ..
        } => {
            if !matches!(transport, PortableRemoteTransport::Http)
                || matches!(authentication, AuthenticationRequirement::OAuth)
            {
                return Err(());
            }
            Ok(serde_json::json!({
                "url": url,
                "headers": headers,
                "disabled": !enabled,
            }))
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

fn is_required_mcp(declarations: &[ComponentDeclaration], name: &str) -> bool {
    declarations.iter().any(|declaration| {
        declaration.kind == ComponentKind::McpServer
            && declaration.declared_name.as_deref() == Some(name)
            && declaration.requiredness == ComponentRequiredness::Required
    })
}

fn component_id(id: &NativeId) -> Result<ComponentId, ManagedProjectionError> {
    ComponentId::new(format!("mcp:{}", id.as_str()))
        .map_err(|_| mcp_invalid("The Amp MCP component id is invalid."))
}

fn relative(path: &str) -> Result<RelativeArtifactPath, ManagedProjectionError> {
    RelativeArtifactPath::new(path).map_err(|_| destination_error())
}

fn destination_error() -> ManagedProjectionError {
    ManagedProjectionError::Other {
        code: "amp_managed_destination_invalid",
        summary: "The Amp managed destination is invalid.",
    }
}

fn mcp_invalid(detail: &'static str) -> ManagedProjectionError {
    ManagedProjectionError::McpInvalid { detail }
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

    #[test]
    fn amp_preserves_skill_local_files_and_rejects_relative_cwd_assumptions() {
        let tree = ArtifactTree::new([
            (
                "skills/demo/SKILL.md",
                skilltap_core::domain::ArtifactFile::new(
                    b"---\nname: demo\ndescription: demo\n---\n".to_vec(),
                    false,
                ),
            ),
            (
                "skills/demo/mcp.json",
                skilltap_core::domain::ArtifactFile::new(
                    br#"{"local":{"command":"./server.sh"}}"#.to_vec(),
                    false,
                ),
            ),
        ])
        .unwrap();
        assert!(validate_skill_local_mcp(&tree, JsonLimits::new(4096, 16).unwrap()).is_err());
    }

    #[test]
    fn amp_settings_map_references_without_auth_or_cwd() {
        let server = PortableMcpServer::Stdio {
            command: "/bin/true".into(),
            args: vec![],
            environment: BTreeMap::from([(String::from("TOKEN"), String::from("${TOKEN}"))]),
            cwd: None,
            enabled: true,
            timeout_ms: None,
            tools: None,
        };
        assert_eq!(map_amp_server(&server).unwrap()["command"], "/bin/true");
        assert!(
            map_amp_server(&PortableMcpServer::Stdio {
                command: "/bin/true".into(),
                args: vec![],
                environment: BTreeMap::new(),
                cwd: Some("/tmp".into()),
                enabled: true,
                timeout_ms: None,
                tools: None,
            })
            .is_err()
        );
    }
}
