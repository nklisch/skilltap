use std::{
    collections::{BTreeMap, BTreeSet},
    io,
};

use skilltap_core::{
    domain::{
        AbsolutePath, ComponentId, ComponentKind, ComponentRequiredness, NativeId,
        RelativeArtifactPath, ResourceKind,
    },
    instructions::fingerprint_contents,
    managed_projection::{
        ManagedFileWrite, ManagedPluginWrite, ManagedProjectionError, ManagedProjectionPlan,
    },
    plugin_graph::ComponentDeclaration,
    runtime::{
        ConfinedFileSystem, ExternalTreeLimits, JsonLimits, RuntimeError, StrictJson,
        StrictJsonDecoder,
    },
    storage::{ArtifactTree, ManagedProjection},
};

use crate::{
    managed_codex_project::ManagedCodexCatalog,
    managed_projection::{ManagedProjectionContext, ManagedProjectionInput, ManagedProjectionPort},
};

use super::file_managed::{CompleteSourcePlugin, read_complete_source_plugin};

const MCP_CONTAINER: &str = "mcpServers";
const MARKETPLACE_DOCUMENTS: &[&str] = &[
    ".github/plugin/marketplace.json",
    ".claude-plugin/marketplace.json",
    ".agents/plugins/marketplace.json",
];
const MCP_SOURCE_DOCUMENTS: &[&str] = &[
    ".mcp.json",
    ".github/mcp.json",
    ".copilot/mcp-config.json",
    ".claude-plugin/mcp.json",
    ".codex-plugin/mcp.json",
    "mcp.json",
];
static PROJECTION: CopilotManagedProjection = CopilotManagedProjection;

pub struct CopilotManagedProjection;

impl CopilotManagedProjection {
    pub fn static_ref() -> &'static dyn ManagedProjectionPort {
        &PROJECTION
    }
}

impl ManagedProjectionPort for CopilotManagedProjection {
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
    let (skill_root, config_root, config_destination) = destination_paths(context)?;
    let (trees, mut current_parts, mut desired_parts, mut manifest) =
        plan_skills(&skill_root, context, plugin.as_ref())?;
    let (mcp_write, mcp_manifest) = plan_mcp(
        &config_root,
        &config_destination,
        context,
        plugin.as_ref(),
        (&mut current_parts, &mut desired_parts),
    )?;
    manifest.extend(mcp_manifest);
    manifest.sort();
    manifest.dedup();
    if trees.is_empty() && mcp_write.is_none() {
        return Err(ManagedProjectionError::Other {
            code: "copilot_managed_plugin_unsupported",
            summary: "The plugin has no faithful Copilot skill or MCP projection.",
        });
    }
    let removal = matches!(context.input, ManagedProjectionInput::Remove);
    if removal {
        manifest.clear();
    }
    Ok(ManagedProjectionPlan {
        trees,
        files: mcp_write.into_iter().collect(),
        manifest,
        current_fingerprint: (!current_parts.is_empty())
            .then(|| fingerprint_contents(&current_parts)),
        desired_fingerprint: (!removal && !desired_parts.is_empty())
            .then(|| fingerprint_contents(&desired_parts)),
    })
}

fn destination_paths(
    context: &ManagedProjectionContext<'_>,
) -> Result<(AbsolutePath, AbsolutePath, RelativeArtifactPath), ManagedProjectionError> {
    match context.scope {
        skilltap_core::domain::Scope::Global => Ok((
            AbsolutePath::new(format!("{}/.agents/skills", context.paths.home().as_str()))
                .map_err(|_| destination_error())?,
            context.paths.home().clone(),
            relative(".copilot/mcp-config.json")?,
        )),
        skilltap_core::domain::Scope::Project(project) => Ok((
            AbsolutePath::new(format!("{}/.agents/skills", project.as_str()))
                .map_err(|_| destination_error())?,
            project.clone(),
            relative(".mcp.json")?,
        )),
    }
}

fn destination_error() -> ManagedProjectionError {
    ManagedProjectionError::Other {
        code: "managed_destination_invalid",
        summary: "The Copilot managed destination is invalid.",
    }
}

fn relative(path: &str) -> Result<RelativeArtifactPath, ManagedProjectionError> {
    RelativeArtifactPath::new(path).map_err(|_| ManagedProjectionError::McpInvalid {
        detail: "The Copilot managed configuration path is invalid.",
    })
}

fn read_selected_plugin(
    context: &ManagedProjectionContext<'_>,
    checkout: &skilltap_core::managed_projection::ResolvedSourceCheckout,
) -> Result<CompleteSourcePlugin, ManagedProjectionError> {
    let selector = skilltap_core::marketplace::PluginSelector::parse(context.request.name.as_str())
        .map_err(|_| ManagedProjectionError::PluginSourceInvalid {
            detail: "The selected Copilot plugin selector is invalid.",
        })?;
    let catalog =
        read_marketplace_catalog(context.filesystem, checkout.root(), context.json_limits)?;
    let plugin_root = catalog
        .plugin_source(selector.plugin(), checkout.root())
        .map_err(|_| ManagedProjectionError::PluginSourceInvalid {
            detail: "The selected Copilot plugin source is not a contained marketplace entry.",
        })?;
    read_complete_source_plugin(&plugin_root, checkout.source(), context.json_limits)
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
                detail: "The selected Copilot marketplace document could not be read safely.",
            })?;
        if let Some(bytes) = bytes {
            return ManagedCodexCatalog::parse(&bytes, limits).map_err(|_| {
                ManagedProjectionError::CatalogInvalid {
                    detail: "The selected Copilot marketplace document is invalid.",
                }
            });
        }
    }
    Err(ManagedProjectionError::CatalogMissing)
}

type ObservedTree = (skilltap_core::runtime::DirectoryIdentity, ArtifactTree);
type SkillPlan = (
    Vec<ManagedPluginWrite>,
    Vec<u8>,
    Vec<u8>,
    Vec<ManagedProjection>,
);

fn plan_skills(
    skill_root: &AbsolutePath,
    context: &ManagedProjectionContext<'_>,
    plugin: Option<&CompleteSourcePlugin>,
) -> Result<SkillPlan, ManagedProjectionError> {
    let removal = matches!(context.input, ManagedProjectionInput::Remove);
    let declarations = plugin.map_or(&[][..], |plugin| plugin.declarations.as_slice());
    let mut names = BTreeSet::new();
    let mut manifest = Vec::new();
    for declaration in declarations {
        match declaration.kind {
            ComponentKind::Skill => {
                names.insert(declaration.declared_name.clone().ok_or(
                    ManagedProjectionError::PluginMissing {
                        detail: "A Copilot plugin skill has no declared name.",
                    },
                )?);
            }
            ComponentKind::McpServer => {}
            _ if declaration.requiredness == ComponentRequiredness::Required => {
                return Err(ManagedProjectionError::RequiredUnsupported);
            }
            _ => manifest.push(ManagedProjection::Omitted {
                id: declaration.id.clone(),
                consequence: evidence("unsupported_optional_component_omitted"),
            }),
        }
    }
    for projection in context.prior {
        if let ManagedProjection::Skill { id, .. } = projection {
            names.insert(id.as_str().to_owned());
        }
    }

    let mut trees = Vec::new();
    let mut current_parts = Vec::new();
    let mut desired_parts = Vec::new();
    for name in names {
        let desired_tree = plugin
            .map(|plugin| skill_tree(&plugin.tree, &name))
            .transpose()?
            .flatten();
        if !removal
            && desired_tree.is_none()
            && declarations.iter().any(|declaration| {
                declaration.kind == ComponentKind::Skill
                    && declaration.declared_name.as_deref() == Some(name.as_str())
                    && declaration.requiredness == ComponentRequiredness::Required
            })
        {
            return Err(ManagedProjectionError::PluginMissing {
                detail: "A required Copilot plugin skill is missing its complete directory.",
            });
        }
        let destination = RelativeArtifactPath::new(&name).map_err(|_| {
            ManagedProjectionError::PluginMissing {
                detail: "A Copilot plugin skill name is not a safe destination.",
            }
        })?;
        let current = observe_tree(context.filesystem, skill_root, &destination)?;
        verify_prior_skill(context.prior, &destination, current.as_ref())?;
        if let Some((_, tree)) = &current {
            append_tree_fingerprint(&mut current_parts, &destination, tree);
        }
        if !removal && let Some(tree) = &desired_tree {
            append_tree_fingerprint(&mut desired_parts, &destination, tree);
            manifest.push(ManagedProjection::Skill {
                id: destination.clone(),
                fingerprint: fingerprint_tree(&destination, tree),
            });
        }
        trees.push(ManagedPluginWrite {
            root: skill_root.clone(),
            destination,
            desired_tree: (!removal).then_some(desired_tree).flatten(),
            expected_tree: current.as_ref().map(|(_, tree)| tree.clone()),
            expected_identity: current.map(|(identity, _)| identity),
        });
    }
    Ok((trees, current_parts, desired_parts, manifest))
}

fn skill_tree(
    plugin: &ArtifactTree,
    name: &str,
) -> Result<Option<ArtifactTree>, ManagedProjectionError> {
    let prefix = format!("skills/{name}/");
    let files = plugin
        .files()
        .iter()
        .filter_map(|(path, file)| {
            path.as_str()
                .strip_prefix(&prefix)
                .map(|relative| (relative.to_owned(), file.clone()))
        })
        .collect::<Vec<_>>();
    if files.is_empty() {
        return Ok(None);
    }
    let tree = ArtifactTree::new(files).map_err(|_| ManagedProjectionError::PluginMissing {
        detail: "A Copilot plugin skill is not a complete artifact tree.",
    })?;
    if !tree
        .files()
        .contains_key(&RelativeArtifactPath::new("SKILL.md").expect("static path is valid"))
    {
        return Err(ManagedProjectionError::PluginMissing {
            detail: "A Copilot plugin skill is missing top-level SKILL.md.",
        });
    }
    Ok(Some(tree))
}

fn observe_tree(
    filesystem: &dyn ConfinedFileSystem,
    root: &AbsolutePath,
    destination: &RelativeArtifactPath,
) -> Result<Option<ObservedTree>, ManagedProjectionError> {
    match filesystem.load_tree_bounded_no_follow(root, destination, tree_limits()) {
        Ok((identity, files)) => Ok(Some((
            identity,
            ArtifactTree::new(
                files
                    .into_iter()
                    .map(|(path, file)| (path.as_str().to_owned(), file)),
            )
            .map_err(|_| ManagedProjectionError::PluginUnreadable {
                detail: "The Copilot managed skill tree is invalid.",
            })?,
        ))),
        Err(RuntimeError::FileSystem { source, .. })
            if source.kind() == io::ErrorKind::NotFound =>
        {
            Ok(None)
        }
        Err(_) => Err(ManagedProjectionError::PluginUnreadable {
            detail: "The Copilot managed skill tree could not be observed safely.",
        }),
    }
}

fn verify_prior_skill(
    prior: &[ManagedProjection],
    destination: &RelativeArtifactPath,
    current: Option<&ObservedTree>,
) -> Result<(), ManagedProjectionError> {
    let Some(expected) = prior.iter().find_map(|projection| match projection {
        ManagedProjection::Skill { id, fingerprint } if id == destination => Some(fingerprint),
        _ => None,
    }) else {
        return Ok(());
    };
    if current
        .map(|(_, tree)| fingerprint_tree(destination, tree))
        .as_ref()
        != Some(expected)
    {
        return Err(ManagedProjectionError::Drifted {
            detail: "An owned Copilot skill projection is missing or was replaced.",
        });
    }
    Ok(())
}

fn plan_mcp(
    config_root: &AbsolutePath,
    config_destination: &RelativeArtifactPath,
    context: &ManagedProjectionContext<'_>,
    plugin: Option<&CompleteSourcePlugin>,
    fingerprints: (&mut Vec<u8>, &mut Vec<u8>),
) -> Result<(Option<ManagedFileWrite>, Vec<ManagedProjection>), ManagedProjectionError> {
    let (current_parts, desired_parts) = fingerprints;
    let removal = matches!(context.input, ManagedProjectionInput::Remove);
    let source_servers = plugin
        .map(|plugin| source_mcp_servers(plugin, context.json_limits))
        .transpose()?
        .unwrap_or_default();
    let declarations = plugin.map_or(&[][..], |plugin| plugin.declarations.as_slice());
    let expected = context
        .filesystem
        .read_regular_bounded_no_follow(
            config_root,
            config_destination,
            context.json_limits.bytes(),
        )
        .map_err(|_| ManagedProjectionError::McpInvalid {
            detail: "The Copilot MCP document could not be read safely.",
        })?;
    let alternate_servers = match context.scope {
        skilltap_core::domain::Scope::Project(_) => read_alternate_project_servers(context)?,
        skilltap_core::domain::Scope::Global => BTreeMap::new(),
    };
    let mut document = match expected.as_deref() {
        Some(bytes) => parse_json_object(bytes, context.json_limits)?,
        None => serde_json::Map::new(),
    };
    let current_servers = document
        .get(MCP_CONTAINER)
        .map(parse_servers)
        .transpose()?
        .unwrap_or_default();
    let mut names = source_servers.keys().cloned().collect::<BTreeSet<_>>();
    names.extend(
        context
            .prior
            .iter()
            .filter_map(|projection| match projection {
                ManagedProjection::Mcp { id, .. } => Some(id.as_str().to_owned()),
                _ => None,
            }),
    );

    let mut manifest = Vec::new();
    let mut touched = false;
    for name in names {
        if alternate_servers.contains_key(&name) && !removal {
            return Err(ManagedProjectionError::McpConflict);
        }
        let native_id = NativeId::new(&name).map_err(|_| ManagedProjectionError::McpInvalid {
            detail: "A Copilot MCP server name is invalid.",
        })?;
        let current = current_servers.get(&name).cloned();
        let prior = context
            .prior
            .iter()
            .find_map(|projection| match projection {
                ManagedProjection::Mcp { id, fingerprint } if id.as_str() == name => {
                    Some(fingerprint)
                }
                _ => None,
            });
        if let Some(expected_fingerprint) = prior {
            if current.as_ref().map(json_fingerprint).as_ref() != Some(expected_fingerprint) {
                return Err(ManagedProjectionError::Drifted {
                    detail: "An owned Copilot MCP server is missing or was replaced.",
                });
            }
            if let Some(value) = &current {
                current_parts.extend(json_fingerprint_bytes(value));
            }
        } else if current.is_some() && !removal {
            return Err(ManagedProjectionError::McpConflict);
        }
        let Some(source) = source_servers.get(&name) else {
            if prior.is_some()
                && document
                    .get_mut(MCP_CONTAINER)
                    .and_then(serde_json::Value::as_object_mut)
                    .is_some_and(|servers| servers.remove(&name).is_some())
            {
                touched = true;
            }
            continue;
        };
        let mapped = match CopilotMcpCodec::encode_server(source) {
            Ok(value) => value,
            Err(_) if is_required_mcp(declarations, &name) => {
                return Err(ManagedProjectionError::RequiredUnsupported);
            }
            Err(_) => {
                manifest.push(ManagedProjection::Omitted {
                    id: ComponentId::new(format!("mcp:{name}")).map_err(|_| {
                        ManagedProjectionError::McpInvalid {
                            detail: "An omitted Copilot MCP server name is invalid.",
                        }
                    })?,
                    consequence: evidence("unsupported_optional_component_omitted"),
                });
                continue;
            }
        };
        touched |= current.as_ref() != Some(&mapped);
        desired_parts.extend(json_fingerprint_bytes(&mapped));
        manifest.push(ManagedProjection::Mcp {
            id: native_id,
            fingerprint: json_fingerprint(&mapped),
        });
        document
            .entry(MCP_CONTAINER.to_owned())
            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()))
            .as_object_mut()
            .ok_or(ManagedProjectionError::McpConflict)?
            .insert(name, mapped);
    }
    if !touched {
        return Ok((None, manifest));
    }
    if document
        .get(MCP_CONTAINER)
        .and_then(serde_json::Value::as_object)
        .is_some_and(serde_json::Map::is_empty)
    {
        document.remove(MCP_CONTAINER);
    }
    let desired =
        if document.is_empty() {
            None
        } else {
            let mut bytes = serde_json::to_vec_pretty(&serde_json::Value::Object(document))
                .map_err(|_| ManagedProjectionError::McpInvalid {
                    detail: "The Copilot MCP document could not be encoded.",
                })?;
            bytes.push(b'\n');
            Some(bytes)
        };
    Ok((
        Some(ManagedFileWrite {
            root: config_root.clone(),
            destination: config_destination.clone(),
            expected,
            desired,
        }),
        manifest,
    ))
}

fn read_alternate_project_servers(
    context: &ManagedProjectionContext<'_>,
) -> Result<BTreeMap<String, serde_json::Value>, ManagedProjectionError> {
    let skilltap_core::domain::Scope::Project(project) = context.scope else {
        return Ok(BTreeMap::new());
    };
    let path = relative(".github/mcp.json")?;
    let bytes = context
        .filesystem
        .read_regular_bounded_no_follow(project, &path, context.json_limits.bytes())
        .map_err(|_| ManagedProjectionError::McpInvalid {
            detail: "The alternate Copilot MCP document could not be read safely.",
        })?;
    let Some(bytes) = bytes else {
        return Ok(BTreeMap::new());
    };
    let document = parse_json_object(&bytes, context.json_limits)?;
    match document.get(MCP_CONTAINER) {
        Some(value) => parse_servers(value),
        None => Ok(BTreeMap::new()),
    }
}

fn source_mcp_servers(
    plugin: &CompleteSourcePlugin,
    limits: JsonLimits,
) -> Result<BTreeMap<String, serde_json::Value>, ManagedProjectionError> {
    let Some(file) = MCP_SOURCE_DOCUMENTS.iter().find_map(|path| {
        plugin
            .tree
            .files()
            .get(&RelativeArtifactPath::new(*path).ok()?)
    }) else {
        return Ok(BTreeMap::new());
    };
    let value = StrictJson.decode(file.contents(), limits).map_err(|_| {
        ManagedProjectionError::McpInvalid {
            detail: "The selected Copilot MCP declaration is invalid JSON.",
        }
    })?;
    let servers = value
        .value()
        .get(MCP_CONTAINER)
        .and_then(serde_json::Value::as_object)
        .ok_or(ManagedProjectionError::McpInvalid {
            detail: "The selected Copilot MCP declaration has no mcpServers object.",
        })?;
    Ok(servers
        .iter()
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect())
}

fn parse_json_object(
    bytes: &[u8],
    limits: JsonLimits,
) -> Result<serde_json::Map<String, serde_json::Value>, ManagedProjectionError> {
    StrictJson
        .decode(bytes, limits)
        .map_err(|_| ManagedProjectionError::McpInvalid {
            detail: "The existing Copilot MCP document is invalid JSON.",
        })?
        .value()
        .as_object()
        .cloned()
        .ok_or(ManagedProjectionError::McpInvalid {
            detail: "The existing Copilot MCP document must be a JSON object.",
        })
}

fn parse_servers(
    value: &serde_json::Value,
) -> Result<BTreeMap<String, serde_json::Value>, ManagedProjectionError> {
    value
        .as_object()
        .ok_or(ManagedProjectionError::McpConflict)
        .map(|object| {
            object
                .iter()
                .map(|(name, value)| (name.clone(), value.clone()))
                .collect()
        })
}

fn is_required_mcp(declarations: &[ComponentDeclaration], name: &str) -> bool {
    declarations.iter().any(|declaration| {
        declaration.kind == ComponentKind::McpServer
            && declaration.declared_name.as_deref() == Some(name)
            && declaration.requiredness == ComponentRequiredness::Required
    })
}

pub(crate) struct CopilotMcpCodec;

impl CopilotMcpCodec {
    fn encode_server(value: &serde_json::Value) -> Result<serde_json::Value, &'static str> {
        let server = value.as_object().ok_or("MCP server must be an object")?;
        let command = optional_string(server.get("command"))?;
        let url = optional_string(server.get("url").or_else(|| server.get("uri")))?;
        if command.is_some() == url.is_some() {
            return Err("MCP transport is absent or ambiguous");
        }
        if command.is_some_and(source_relative) || url.is_some_and(source_relative) {
            return Err("MCP endpoint depends on the plugin source");
        }
        if let Some(args) = server.get("args") {
            let args = args.as_array().ok_or("MCP args must be an array")?;
            if !args.iter().all(serde_json::Value::is_string)
                || args
                    .iter()
                    .any(|value| value.as_str().is_some_and(source_relative))
            {
                return Err("MCP args are not faithfully portable");
            }
        }
        if let Some(cwd) = server.get("cwd")
            && cwd.as_str().is_none_or(source_relative)
        {
            return Err("MCP cwd is not portable");
        }
        for key in ["env", "environment", "headers"] {
            if let Some(value) = server.get(key) {
                validate_reference_map(value)?;
            }
        }
        if let Some(auth) = server.get("auth")
            && !auth.as_str().is_some_and(is_reference)
        {
            return Err("MCP auth must remain a credential reference");
        }
        if let Some(kind) = server.get("type").and_then(serde_json::Value::as_str) {
            let expected = if command.is_some() { "stdio" } else { "http" };
            if !matches!(
                (expected, kind),
                ("stdio", "stdio" | "local") | ("http", "http" | "remote")
            ) {
                return Err("MCP transport type is not faithful");
            }
        }
        Ok(value.clone())
    }
}

fn optional_string(value: Option<&serde_json::Value>) -> Result<Option<&str>, &'static str> {
    match value {
        None => Ok(None),
        Some(value) => value
            .as_str()
            .filter(|value| !value.is_empty())
            .map(Some)
            .ok_or("MCP transport value must be a non-empty string"),
    }
}

fn validate_reference_map(value: &serde_json::Value) -> Result<(), &'static str> {
    let object = value
        .as_object()
        .ok_or("MCP references must be an object")?;
    if object
        .values()
        .any(|value| !value.as_str().is_some_and(is_reference))
    {
        return Err("literal MCP credential material is not portable");
    }
    Ok(())
}

fn is_reference(value: &str) -> bool {
    (value.starts_with('$') && value.len() > 1)
        || (value.starts_with("${") && value.ends_with('}') && value.len() > 3)
        || (value.starts_with("{env:") && value.ends_with('}') && value.len() > 6)
}

fn source_relative(value: &str) -> bool {
    value.starts_with("./")
        || value.starts_with("../")
        || value.contains("PLUGIN_ROOT")
        || value.contains("COPILOT_PLUGIN")
}

fn evidence(code: &'static str) -> skilltap_core::domain::EvidenceCode {
    skilltap_core::domain::EvidenceCode::new(code).expect("static Copilot evidence code is valid")
}

fn fingerprint_tree(
    destination: &RelativeArtifactPath,
    tree: &ArtifactTree,
) -> skilltap_core::domain::Fingerprint {
    let mut bytes = Vec::new();
    append_tree_fingerprint(&mut bytes, destination, tree);
    fingerprint_contents(&bytes)
}

fn append_tree_fingerprint(
    bytes: &mut Vec<u8>,
    destination: &RelativeArtifactPath,
    tree: &ArtifactTree,
) {
    bytes.extend_from_slice(destination.as_str().as_bytes());
    for (path, file) in tree.files() {
        bytes.extend_from_slice(path.as_str().as_bytes());
        bytes.push(u8::from(file.is_executable()));
        bytes.extend_from_slice(file.contents());
    }
}

fn json_fingerprint(value: &serde_json::Value) -> skilltap_core::domain::Fingerprint {
    fingerprint_contents(&json_fingerprint_bytes(value))
}

fn json_fingerprint_bytes(value: &serde_json::Value) -> Vec<u8> {
    serde_json::to_vec(value).unwrap_or_default()
}

fn tree_limits() -> ExternalTreeLimits {
    ExternalTreeLimits::new(64, 100_000, 64 * 1024 * 1024, 1024 * 1024 * 1024, 64 * 1024)
        .expect("static Copilot tree limits are valid")
}

#[cfg(test)]
mod tests {
    use super::*;
    use skilltap_core::domain::Scope;

    #[test]
    fn codec_preserves_unknown_members_and_reference_values() {
        let value = serde_json::json!({
            "type": "stdio",
            "command": "node",
            "args": ["server.mjs"],
            "env": {"TOKEN": "${MCP_TOKEN}"},
            "future": {"keep": true}
        });
        assert_eq!(CopilotMcpCodec::encode_server(&value).unwrap(), value);
    }

    #[test]
    fn codec_rejects_literal_secrets_ambiguous_transport_and_source_paths() {
        for value in [
            serde_json::json!({"command":"node","url":"https://example.invalid"}),
            serde_json::json!({"command":"node","env":{"TOKEN":"secret"}}),
            serde_json::json!({"command":"./server"}),
            serde_json::json!({"url":"https://example.invalid","auth":"literal"}),
        ] {
            assert!(
                CopilotMcpCodec::encode_server(&value).is_err(),
                "accepted {value}"
            );
        }
    }

    #[test]
    fn managed_paths_are_canonical_and_alternate_project_files_are_not_targets() {
        let project = AbsolutePath::new("/project").unwrap();
        assert_eq!(
            destination_paths_for_test(&Scope::Global).0.as_str(),
            "/home/.agents/skills"
        );
        assert_eq!(
            project_skill_path(&project).as_str(),
            "/project/.agents/skills"
        );
    }

    fn destination_paths_for_test(scope: &Scope) -> (AbsolutePath, RelativeArtifactPath) {
        match scope {
            Scope::Global => (
                AbsolutePath::new("/home/.agents/skills").unwrap(),
                relative(".copilot/mcp-config.json").unwrap(),
            ),
            Scope::Project(project) => {
                (project_skill_path(project), relative(".mcp.json").unwrap())
            }
        }
    }

    fn project_skill_path(project: &AbsolutePath) -> AbsolutePath {
        AbsolutePath::new(format!("{}/.agents/skills", project.as_str())).unwrap()
    }
}
