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

use super::file_managed::read_complete_source_plugin;

const MARKETPLACE_DOCUMENTS: &[&str] = &[
    ".agents/plugins/marketplace.json",
    ".claude-plugin/marketplace.json",
];
const MCP_SOURCE_DOCUMENTS: &[&str] = &[
    ".codex-plugin/mcp.json",
    ".claude-plugin/mcp.json",
    ".mcp.json",
    "mcp.json",
];
const MCP_CONTAINER: &str = "mcpServers";
const POWER_REQUIRED_CODE: &str = "kiro_power_required";
const STEERING_ID: &str = "kiro:steering";

static PROJECTION: KiroManagedProjection = KiroManagedProjection;

/// Kiro's file-managed projection. It owns only Agent Skills directories and
/// the documented `mcpServers` member of the global/workspace settings file.
/// Powers, IDE state, and Kiro's runtime cache are deliberately outside this
/// projection.
pub struct KiroManagedProjection;

impl KiroManagedProjection {
    pub fn static_ref() -> &'static dyn ManagedProjectionPort {
        &PROJECTION
    }
}

impl ManagedProjectionPort for KiroManagedProjection {
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
            code: "managed_project_plugin_unsupported",
            summary: "The plugin has no faithful Kiro skill or MCP projection.",
        });
    }

    let removal = matches!(context.input, ManagedProjectionInput::Remove);
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

fn read_selected_plugin(
    context: &ManagedProjectionContext<'_>,
    checkout: &skilltap_core::managed_projection::ResolvedSourceCheckout,
) -> Result<super::file_managed::CompleteSourcePlugin, ManagedProjectionError> {
    let selector = skilltap_core::marketplace::PluginSelector::parse(context.request.name.as_str())
        .map_err(|_| ManagedProjectionError::PluginSourceInvalid {
            detail: "The selected Kiro plugin selector is invalid.",
        })?;
    let catalog =
        read_marketplace_catalog(context.filesystem, checkout.root(), context.json_limits)?;
    let plugin_root = catalog
        .plugin_source(selector.plugin(), checkout.root())
        .map_err(|_| ManagedProjectionError::PluginSourceInvalid {
            detail: "The selected Kiro plugin source is not a contained local marketplace entry.",
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
                detail: "The selected Kiro marketplace document could not be read safely.",
            })?;
        if let Some(bytes) = bytes {
            return ManagedCodexCatalog::parse(&bytes, limits).map_err(|_| {
                ManagedProjectionError::CatalogInvalid {
                    detail: "The selected Kiro marketplace document is invalid.",
                }
            });
        }
    }
    Err(ManagedProjectionError::CatalogMissing)
}

fn destination_paths(
    context: &ManagedProjectionContext<'_>,
) -> Result<(AbsolutePath, AbsolutePath, RelativeArtifactPath), ManagedProjectionError> {
    let (skill_root, config_root, config_destination) = match context.scope {
        skilltap_core::domain::Scope::Global => (
            AbsolutePath::new(format!("{}/skills", context.paths.kiro_home().as_str())),
            context.paths.kiro_home().clone(),
            relative("settings/mcp.json")?,
        ),
        skilltap_core::domain::Scope::Project(project) => (
            AbsolutePath::new(format!("{}/.kiro/skills", project.as_str())),
            project.clone(),
            relative(".kiro/settings/mcp.json")?,
        ),
    };
    let skill_root = skill_root.map_err(|_| ManagedProjectionError::Other {
        code: "managed_destination_invalid",
        summary: "The Kiro managed skill destination is invalid.",
    })?;
    Ok((skill_root, config_root, config_destination))
}

type SkillPlan = (
    Vec<ManagedPluginWrite>,
    Vec<u8>,
    Vec<u8>,
    Vec<ManagedProjection>,
);

fn plan_skills(
    skill_root: &AbsolutePath,
    context: &ManagedProjectionContext<'_>,
    plugin: Option<&super::file_managed::CompleteSourcePlugin>,
) -> Result<SkillPlan, ManagedProjectionError> {
    let removal = matches!(context.input, ManagedProjectionInput::Remove);
    if let Some(plugin) = plugin
        && plugin
            .tree
            .files()
            .keys()
            .any(|path| path.as_str() == "POWER.md" || path.as_str().ends_with("/POWER.md"))
    {
        return Err(ManagedProjectionError::Other {
            code: POWER_REQUIRED_CODE,
            summary: "The selected source requires a Kiro Power; Powers are an IDE/web lifecycle and are not translated by skilltap.",
        });
    }

    let declarations = plugin.map_or(&[][..], |plugin| plugin.declarations.as_slice());
    let mut skill_names = BTreeSet::new();
    let mut manifest = Vec::new();
    let mut omitted = Vec::new();
    for declaration in declarations {
        match declaration.kind {
            ComponentKind::Skill => {
                let name = declaration.declared_name.as_deref().ok_or(
                    ManagedProjectionError::PluginMissing {
                        detail: "A Kiro plugin skill has no declared name.",
                    },
                )?;
                skill_names.insert(name.to_owned());
            }
            ComponentKind::McpServer => {}
            _ if declaration.requiredness == ComponentRequiredness::Required => {
                return Err(ManagedProjectionError::RequiredUnsupported);
            }
            _ => omitted.push(ManagedProjection::Omitted {
                id: declaration.id.clone(),
                consequence: evidence("unsupported_optional_component_omitted"),
            }),
        }
    }

    if plugin.is_some_and(|plugin| {
        plugin
            .tree
            .files()
            .keys()
            .any(|path| path.as_str().starts_with("steering/"))
    }) {
        omitted.push(ManagedProjection::Omitted {
            id: ComponentId::new(STEERING_ID).expect("static Kiro steering id is valid"),
            consequence: evidence("unsupported_optional_component_omitted"),
        });
    }
    if !omitted.is_empty() && !context.acknowledged && !removal {
        return Err(ManagedProjectionError::Other {
            code: "partial_operation_requires_acknowledgment",
            summary: "The plugin contains Kiro Power-adjacent steering or other unsupported optional components; rerun with `--yes` to accept their omission.",
        });
    }
    manifest.extend(omitted);

    for projection in context.prior {
        if let ManagedProjection::Skill { id, .. } = projection {
            skill_names.insert(id.as_str().to_owned());
        }
    }

    let mut trees = Vec::new();
    let mut current_parts = Vec::new();
    let mut desired_parts = Vec::new();
    for name in skill_names {
        let desired_tree = match plugin {
            Some(plugin) => skill_tree(&plugin.tree, &name)?,
            None => None,
        };
        if !removal
            && desired_tree.is_none()
            && declarations.iter().any(|declaration| {
                declaration.kind == ComponentKind::Skill
                    && declaration.declared_name.as_deref() == Some(name.as_str())
                    && declaration.requiredness == ComponentRequiredness::Required
            })
        {
            return Err(ManagedProjectionError::PluginMissing {
                detail: "A required Kiro plugin skill is missing its complete directory.",
            });
        }
        let destination = relative(&name)?;
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
        detail: "A Kiro plugin skill is not a complete artifact tree.",
    })?;
    if !tree
        .files()
        .contains_key(&RelativeArtifactPath::new("SKILL.md").expect("static path is valid"))
    {
        return Err(ManagedProjectionError::PluginMissing {
            detail: "A Kiro plugin skill is missing top-level SKILL.md.",
        });
    }
    Ok(Some(tree))
}

type ObservedTree = (skilltap_core::runtime::DirectoryIdentity, ArtifactTree);

fn observe_tree(
    filesystem: &dyn ConfinedFileSystem,
    root: &AbsolutePath,
    destination: &RelativeArtifactPath,
) -> Result<Option<ObservedTree>, ManagedProjectionError> {
    match filesystem.load_tree_bounded_no_follow(root, destination, tree_limits()) {
        Ok((identity, files)) => {
            let tree = ArtifactTree::new(
                files
                    .into_iter()
                    .map(|(path, file)| (path.as_str().to_owned(), file)),
            )
            .map_err(|_| ManagedProjectionError::PluginUnreadable {
                detail: "The Kiro managed skill tree is invalid.",
            })?;
            Ok(Some((identity, tree)))
        }
        Err(RuntimeError::FileSystem { source, .. })
            if source.kind() == io::ErrorKind::NotFound =>
        {
            Ok(None)
        }
        Err(_) => Err(ManagedProjectionError::PluginUnreadable {
            detail: "The Kiro managed skill tree could not be observed safely.",
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
    let observed = current.map(|(_, tree)| fingerprint_tree(destination, tree));
    if observed.as_ref() != Some(expected) {
        return Err(ManagedProjectionError::Drifted {
            detail: "An owned Kiro skill projection is missing or was replaced.",
        });
    }
    Ok(())
}

fn plan_mcp(
    config_root: &AbsolutePath,
    config_destination: &RelativeArtifactPath,
    context: &ManagedProjectionContext<'_>,
    plugin: Option<&super::file_managed::CompleteSourcePlugin>,
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
            detail: "The Kiro MCP settings document could not be read safely.",
        })?;
    let mut document = match expected.as_deref() {
        Some(bytes) => parse_json_object(bytes, context.json_limits)?,
        None => serde_json::Map::new(),
    };
    let current_servers = match document.get(MCP_CONTAINER) {
        None => BTreeMap::new(),
        Some(value) => value
            .as_object()
            .cloned()
            .ok_or(ManagedProjectionError::McpConflict)?
            .into_iter()
            .collect(),
    };
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
        let native_id = NativeId::new(&name).map_err(|_| ManagedProjectionError::McpInvalid {
            detail: "A Kiro MCP server name is invalid.",
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
            let observed = current.as_ref().map(json_fingerprint);
            if observed.as_ref() != Some(expected_fingerprint) {
                return Err(ManagedProjectionError::Drifted {
                    detail: "An owned Kiro MCP server is missing or was replaced.",
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
        let mapped = match map_kiro_server(source) {
            Ok(mapped) => mapped,
            Err(_) => {
                if is_required_mcp(declarations, &name) {
                    return Err(ManagedProjectionError::RequiredUnsupported);
                }
                if !context.acknowledged && !removal {
                    return Err(ManagedProjectionError::Other {
                        code: "partial_operation_requires_acknowledgment",
                        summary: "The plugin contains an MCP server that cannot be projected faithfully to Kiro; rerun with `--yes` to accept its omission.",
                    });
                }
                manifest.push(ManagedProjection::Omitted {
                    id: ComponentId::new(format!("mcp:{name}")).map_err(|_| {
                        ManagedProjectionError::McpInvalid {
                            detail: "An omitted Kiro MCP server name is invalid.",
                        }
                    })?,
                    consequence: evidence("unsupported_optional_component_omitted"),
                });
                if prior.is_some()
                    && document
                        .get_mut(MCP_CONTAINER)
                        .and_then(serde_json::Value::as_object_mut)
                        .is_some_and(|servers| servers.remove(&name).is_some())
                {
                    touched = true;
                }
                continue;
            }
        };
        let changed = current.as_ref() != Some(&mapped);
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
        touched |= changed;
    }

    if !touched {
        return Ok((None, manifest));
    }
    let empty = document
        .get(MCP_CONTAINER)
        .and_then(serde_json::Value::as_object)
        .is_some_and(serde_json::Map::is_empty);
    if empty {
        document.remove(MCP_CONTAINER);
    }
    let desired =
        if document.is_empty() {
            None
        } else {
            let mut bytes = serde_json::to_vec_pretty(&serde_json::Value::Object(document))
                .map_err(|_| ManagedProjectionError::McpInvalid {
                    detail: "The Kiro MCP settings document could not be encoded.",
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

fn source_mcp_servers(
    plugin: &super::file_managed::CompleteSourcePlugin,
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
            detail: "The selected Kiro MCP declaration is invalid JSON.",
        }
    })?;
    let servers = value
        .value()
        .get(MCP_CONTAINER)
        .and_then(serde_json::Value::as_object)
        .ok_or(ManagedProjectionError::McpInvalid {
            detail: "The selected Kiro MCP declaration has no mcpServers object.",
        })?;
    Ok(servers
        .iter()
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect())
}

fn map_kiro_server(value: &serde_json::Value) -> Result<serde_json::Value, &'static str> {
    let server = value.as_object().ok_or("server must be an object")?;
    let command = non_empty_string(server.get("command"))?;
    let url = non_empty_string(server.get("url"))?;
    if usize::from(command.is_some()) + usize::from(url.is_some()) != 1 {
        return Err("Kiro requires exactly one documented transport");
    }
    if server.contains_key("httpUrl") {
        return Err("Kiro's documented URL field is url");
    }
    if let Some(kind) = server.get("type") {
        let kind = kind.as_str().ok_or("server type is not a string")?;
        let valid = if command.is_some() {
            kind == "stdio"
        } else {
            matches!(kind, "sse" | "http")
        };
        if !valid {
            return Err("server transport type is not faithful");
        }
    }
    validate_string_array(server.get("args"))?;
    validate_string_array(server.get("disabledTools"))?;
    validate_string_array(server.get("includeTools"))?;
    validate_reference_map(server.get("env"))?;
    validate_reference_map(server.get("headers"))?;
    validate_optional_bool(server.get("disabled"))?;
    validate_optional_string(server.get("cwd"))?;
    validate_optional_number(server.get("timeout"))?;
    if server.contains_key("oauth") || server.contains_key("oauthScopes") {
        return Err("OAuth credentials and interactive auth are not source-portable");
    }
    Ok(value.clone())
}

fn non_empty_string(value: Option<&serde_json::Value>) -> Result<Option<&str>, &'static str> {
    match value {
        None => Ok(None),
        Some(value) => value
            .as_str()
            .filter(|value| !value.is_empty())
            .map(Some)
            .ok_or("transport is not a non-empty string"),
    }
}

fn validate_optional_bool(value: Option<&serde_json::Value>) -> Result<(), &'static str> {
    if let Some(value) = value {
        value.as_bool().ok_or("server boolean option is invalid")?;
    }
    Ok(())
}

fn validate_optional_string(value: Option<&serde_json::Value>) -> Result<(), &'static str> {
    if let Some(value) = value {
        value.as_str().ok_or("server string option is invalid")?;
    }
    Ok(())
}

fn validate_optional_number(value: Option<&serde_json::Value>) -> Result<(), &'static str> {
    if let Some(value) = value {
        value.as_u64().ok_or("server numeric option is invalid")?;
    }
    Ok(())
}

fn validate_string_array(value: Option<&serde_json::Value>) -> Result<(), &'static str> {
    if let Some(value) = value {
        value
            .as_array()
            .ok_or("server list option is invalid")?
            .iter()
            .all(serde_json::Value::is_string)
            .then_some(())
            .ok_or("server list option contains a non-string")?;
    }
    Ok(())
}

fn validate_reference_map(value: Option<&serde_json::Value>) -> Result<(), &'static str> {
    if let Some(value) = value {
        let object = value.as_object().ok_or("server reference map is invalid")?;
        if object
            .values()
            .any(|value| !value.as_str().is_some_and(|value| value.starts_with('$')))
        {
            return Err("server reference map contains a literal value");
        }
    }
    Ok(())
}

fn is_required_mcp(declarations: &[ComponentDeclaration], name: &str) -> bool {
    declarations.iter().any(|declaration| {
        declaration.kind == ComponentKind::McpServer
            && declaration.declared_name.as_deref() == Some(name)
            && declaration.requiredness == ComponentRequiredness::Required
    })
}

fn parse_json_object(
    bytes: &[u8],
    limits: JsonLimits,
) -> Result<serde_json::Map<String, serde_json::Value>, ManagedProjectionError> {
    StrictJson
        .decode(bytes, limits)
        .map_err(|_| ManagedProjectionError::McpInvalid {
            detail: "The existing Kiro MCP settings document is invalid JSON.",
        })?
        .value()
        .as_object()
        .cloned()
        .ok_or(ManagedProjectionError::McpInvalid {
            detail: "The existing Kiro MCP settings document must be a JSON object.",
        })
}

fn relative(path: &str) -> Result<RelativeArtifactPath, ManagedProjectionError> {
    RelativeArtifactPath::new(path).map_err(|_| ManagedProjectionError::Other {
        code: "managed_destination_invalid",
        summary: "The Kiro managed destination is invalid.",
    })
}

fn evidence(code: &'static str) -> skilltap_core::domain::EvidenceCode {
    skilltap_core::domain::EvidenceCode::new(code).expect("static Kiro evidence code is valid")
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
        .expect("static managed tree limits are valid")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kiro_codec_preserves_disabled_filters_and_unknown_fields() {
        let value = serde_json::json!({
            "command": "node",
            "args": ["server.mjs"],
            "env": {"TOKEN": "$TOKEN"},
            "disabled": true,
            "disabledTools": ["delete_file"],
            "includeTools": ["read_file"],
            "futureField": {"kept": true}
        });
        assert_eq!(map_kiro_server(&value).unwrap(), value);
    }

    #[test]
    fn kiro_codec_rejects_ambiguous_transports_literals_and_oauth() {
        for value in [
            serde_json::json!({"command":"node","url":"https://example.invalid"}),
            serde_json::json!({"command":"node","env":{"TOKEN":"secret"}}),
            serde_json::json!({"url":"https://example.invalid","oauth":{"clientId":"id"}}),
            serde_json::json!({"httpUrl":"https://example.invalid"}),
        ] {
            assert!(
                map_kiro_server(&value).is_err(),
                "unexpectedly accepted {value}"
            );
        }
    }

    #[test]
    fn unknown_settings_and_unowned_servers_survive_object_codec() {
        let settings = parse_json_object(
            br#"{"future":{"enabled":true},"mcpServers":{"unmanaged":{"command":"keep"}}}"#,
            JsonLimits::new(4096, 16).unwrap(),
        )
        .unwrap();
        assert_eq!(settings["future"]["enabled"], true);
        assert_eq!(settings[MCP_CONTAINER]["unmanaged"]["command"], "keep");
    }
}
