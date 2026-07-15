use std::collections::{BTreeMap, BTreeSet};

use skilltap_core::{
    domain::{
        AbsolutePath, ComponentId, ComponentKind, ComponentRequiredness, NativeId,
        RelativeArtifactPath, ResourceKind,
    },
    instructions::fingerprint_contents,
    managed_projection::{ManagedFileWrite, ManagedProjectionError, ManagedProjectionPlan},
    plugin_graph::ComponentDeclaration,
    runtime::{ConfinedFileSystem, JsonLimits, StrictJson, StrictJsonDecoder},
    storage::ManagedProjection,
};

use crate::{
    managed_codex_project::ManagedCodexCatalog,
    managed_projection::{ManagedProjectionContext, ManagedProjectionInput, ManagedProjectionPort},
};

use super::{
    configuration_constrained::common::{
        SkillProjectionDestinationError, SkillProjectionDiagnostics, SkillProjectionPolicy,
        plan_skills_with_policy,
    },
    file_managed::{CompleteSourcePlugin, read_complete_source_plugin},
};

const MARKETPLACE_DOCUMENTS: &[&str] = &[
    ".agents/plugins/marketplace.json",
    ".claude-plugin/marketplace.json",
];
const MCP_SOURCE_DOCUMENTS: &[&str] = &[".codex-plugin/mcp.json", ".mcp.json", "mcp.json"];
const MCP_CONTAINER: &str = "mcp";
const SKILL_POLICY: SkillProjectionPolicy = SkillProjectionPolicy::complete_tree(
    SkillProjectionDiagnostics {
        missing_declared_name: "An OpenCode plugin skill has no declared name.",
        required_missing_tree: "A required OpenCode plugin skill is missing its complete directory.",
        unsafe_destination: SkillProjectionDestinationError::PluginMissing {
            detail: "An OpenCode plugin skill name is not a safe destination.",
        },
        incomplete_tree: "An OpenCode plugin skill is not a complete artifact tree.",
        missing_top_level_skill: "An OpenCode plugin skill is missing top-level SKILL.md.",
        invalid_agent_skill_name: None,
        invalid_contract: None,
        observed_tree_invalid: "The OpenCode managed skill tree is invalid.",
        observe_safely_failed: "The OpenCode managed skill tree could not be observed safely.",
        drifted: "An owned OpenCode skill projection is missing or was replaced.",
    },
);

static PROJECTION: OpenCodeManagedProjection = OpenCodeManagedProjection;

pub struct OpenCodeManagedProjection;

impl OpenCodeManagedProjection {
    pub fn static_ref() -> &'static dyn ManagedProjectionPort {
        &PROJECTION
    }
}

impl ManagedProjectionPort for OpenCodeManagedProjection {
    fn plan(
        &self,
        context: &ManagedProjectionContext<'_>,
    ) -> Result<ManagedProjectionPlan, ManagedProjectionError> {
        if context.resource_kind == ResourceKind::Marketplace {
            return Ok(ManagedProjectionPlan::default());
        }
        if context.resource_kind != ResourceKind::Plugin {
            return Err(ManagedProjectionError::UnsupportedResourceKind);
        }

        let plugin = match &context.input {
            ManagedProjectionInput::Apply { checkout } => {
                Some(read_selected_plugin(context, checkout)?)
            }
            ManagedProjectionInput::Remove => None,
        };
        let (skill_root, config_root, config_destination) = destination_paths(context)?;
        let (trees, mut current_parts, mut desired_parts, mut manifest) =
            plan_skills_with_policy(&skill_root, context, plugin.as_ref(), SKILL_POLICY)?;
        let (mcp_file, mcp_manifest) = plan_mcp(
            &config_root,
            &config_destination,
            context,
            plugin.as_ref(),
            (&mut current_parts, &mut desired_parts),
        )?;
        manifest.extend(mcp_manifest);
        manifest.sort();
        manifest.dedup();

        if trees.is_empty() && mcp_file.is_none() {
            return Err(ManagedProjectionError::Other {
                code: "managed_project_plugin_unsupported",
                summary: "The plugin has no faithful OpenCode skill or MCP projection.",
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
}

fn destination_paths(
    context: &ManagedProjectionContext<'_>,
) -> Result<(AbsolutePath, AbsolutePath, RelativeArtifactPath), ManagedProjectionError> {
    let skill_root = match context.scope {
        skilltap_core::domain::Scope::Global => {
            AbsolutePath::new(format!("{}/.agents/skills", context.paths.home().as_str()))
        }
        skilltap_core::domain::Scope::Project(project) => {
            AbsolutePath::new(format!("{}/.agents/skills", project.as_str()))
        }
    }
    .map_err(|_| ManagedProjectionError::Other {
        code: "managed_destination_invalid",
        summary: "The OpenCode managed skill destination is invalid.",
    })?;
    let (config_root, config_destination) = match context.scope {
        skilltap_core::domain::Scope::Global => (
            context.paths.config_home().clone(),
            RelativeArtifactPath::new("opencode/opencode.json"),
        ),
        skilltap_core::domain::Scope::Project(project) => {
            (project.clone(), RelativeArtifactPath::new("opencode.json"))
        }
    };
    let config_destination =
        config_destination.map_err(|_| ManagedProjectionError::McpInvalid {
            detail: "The OpenCode configuration path is invalid.",
        })?;
    Ok((skill_root, config_root, config_destination))
}

fn read_selected_plugin(
    context: &ManagedProjectionContext<'_>,
    checkout: &skilltap_core::managed_projection::ResolvedSourceCheckout,
) -> Result<CompleteSourcePlugin, ManagedProjectionError> {
    let selector = skilltap_core::marketplace::PluginSelector::parse(context.request.name.as_str())
        .map_err(|_| ManagedProjectionError::PluginSourceInvalid {
            detail: "The selected OpenCode plugin selector is invalid.",
        })?;
    let catalog =
        read_marketplace_catalog(context.filesystem, checkout.root(), context.json_limits)?;
    let plugin_root = catalog
        .plugin_source(selector.plugin(), checkout.root())
        .map_err(|_| ManagedProjectionError::PluginSourceInvalid {
            detail: "The selected OpenCode plugin source is not a contained local marketplace entry.",
        })?;
    read_complete_source_plugin(&plugin_root, checkout.source(), context.json_limits)
}

fn read_marketplace_catalog(
    filesystem: &dyn ConfinedFileSystem,
    root: &AbsolutePath,
    limits: JsonLimits,
) -> Result<ManagedCodexCatalog, ManagedProjectionError> {
    for path in MARKETPLACE_DOCUMENTS {
        let destination = RelativeArtifactPath::new(*path).map_err(|_| {
            ManagedProjectionError::CatalogInvalid {
                detail: "The selected OpenCode marketplace document path is invalid.",
            }
        })?;
        let bytes = filesystem
            .read_regular_bounded_no_follow(root, &destination, limits.bytes())
            .map_err(|_| ManagedProjectionError::CatalogInvalid {
                detail: "The selected OpenCode marketplace document could not be read safely.",
            })?;
        if let Some(bytes) = bytes {
            return ManagedCodexCatalog::parse(&bytes, limits).map_err(|_| {
                ManagedProjectionError::CatalogInvalid {
                    detail: "The selected OpenCode marketplace document is invalid.",
                }
            });
        }
    }
    Err(ManagedProjectionError::CatalogMissing)
}

fn omitted_component(id: &ComponentId) -> Result<ManagedProjection, ManagedProjectionError> {
    Ok(ManagedProjection::Omitted {
        id: id.clone(),
        consequence: skilltap_core::domain::EvidenceCode::new(
            "unsupported_optional_component_omitted",
        )
        .expect("static evidence code is valid"),
    })
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
        .map(|value| source_mcp_servers(value, context.json_limits))
        .transpose()?
        .unwrap_or_default();
    let declarations = plugin
        .map(|value| value.declarations.as_slice())
        .unwrap_or(&[]);
    let expected = context
        .filesystem
        .read_regular_bounded_no_follow(
            config_root,
            config_destination,
            context.json_limits.bytes(),
        )
        .map_err(|_| ManagedProjectionError::McpInvalid {
            detail: "The OpenCode configuration document could not be read safely.",
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
            detail: "An OpenCode MCP server name is invalid.",
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
                    detail: "An owned OpenCode MCP server is missing or was replaced.",
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
                && let Some(servers) = document
                    .get_mut(MCP_CONTAINER)
                    .and_then(serde_json::Value::as_object_mut)
            {
                servers.remove(&name);
                touched = true;
            }
            continue;
        };
        let mapped = match OpenCodeMcpCodec::encode_server(source) {
            Ok(value) => value,
            Err(_) => {
                if is_required_mcp(declarations, &name) {
                    return Err(ManagedProjectionError::RequiredUnsupported);
                }
                manifest.push(omitted_component(
                    &ComponentId::new(format!("mcp:{name}")).map_err(|_| {
                        ManagedProjectionError::McpInvalid {
                            detail: "An omitted OpenCode MCP server name is invalid.",
                        }
                    })?,
                )?);
                if prior.is_some()
                    && let Some(servers) = document
                        .get_mut(MCP_CONTAINER)
                        .and_then(serde_json::Value::as_object_mut)
                {
                    servers.remove(&name);
                    touched = true;
                }
                continue;
            }
        };
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
        touched = true;
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
                    detail: "The OpenCode configuration document could not be encoded.",
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
            detail: "The selected OpenCode MCP declaration is invalid JSON.",
        }
    })?;
    let servers = value
        .value()
        .get("mcpServers")
        .and_then(serde_json::Value::as_object)
        .ok_or(ManagedProjectionError::McpInvalid {
            detail: "The selected OpenCode MCP declaration has no mcpServers object.",
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
            detail: "The existing OpenCode configuration document is invalid JSON.",
        })?
        .value()
        .as_object()
        .cloned()
        .ok_or(ManagedProjectionError::McpInvalid {
            detail: "The existing OpenCode configuration document must be a JSON object.",
        })
}

fn is_required_mcp(declarations: &[ComponentDeclaration], name: &str) -> bool {
    declarations.iter().any(|declaration| {
        declaration.kind == ComponentKind::McpServer
            && declaration.declared_name.as_deref() == Some(name)
            && declaration.requiredness == ComponentRequiredness::Required
    })
}

/// OpenCode's MCP schema is private to this adapter. In particular, source
/// `mcpServers` values are never copied into the native `mcp` object wholesale.
pub(crate) struct OpenCodeMcpCodec;

impl OpenCodeMcpCodec {
    fn encode_server(value: &serde_json::Value) -> Result<serde_json::Value, &'static str> {
        let source = value.as_object().ok_or("MCP server must be an object")?;
        let command = optional_non_empty_string(source.get("command"))?;
        let url = optional_non_empty_string(source.get("url").or_else(|| source.get("uri")))?;
        if command.is_some() == url.is_some() {
            return Err("MCP transport is absent or ambiguous");
        }
        if source
            .get("oauth")
            .is_some_and(|oauth| !oauth.is_boolean() || oauth.as_bool() != Some(false))
        {
            return Err("OAuth configuration requires native credentials and is not portable");
        }
        if source.keys().any(|key| {
            matches!(
                key.as_str(),
                "tools" | "includeTools" | "excludeTools" | "toolFilter" | "transport"
            )
        }) {
            return Err(
                "the source tool-filter or transport field has no faithful OpenCode server equivalent",
            );
        }

        let mut output = serde_json::Map::new();
        if let Some(command) = command {
            if path_depends_on_source(command) {
                return Err("MCP command depends on the source plugin root");
            }
            if source.get("headers").is_some() {
                return Err("local MCP headers have no OpenCode equivalent");
            }
            if let Some(kind) = source.get("type").and_then(serde_json::Value::as_str)
                && !matches!(kind, "local" | "stdio")
            {
                return Err("the source local transport is not the OpenCode local transport");
            }
            let mut argv = vec![serde_json::Value::String(command.to_owned())];
            if let Some(args) = source.get("args") {
                let args = args.as_array().ok_or("MCP args must be an array")?;
                if !args.iter().all(serde_json::Value::is_string) {
                    return Err("MCP args must contain only strings");
                }
                if args.iter().any(string_value_depends_on_source) {
                    return Err("MCP args depend on the source plugin root");
                }
                argv.extend(args.iter().cloned());
            }
            output.insert(
                "type".to_owned(),
                serde_json::Value::String("local".to_owned()),
            );
            output.insert("command".to_owned(), serde_json::Value::Array(argv));
            if let Some(environment) = source.get("environment").or_else(|| source.get("env")) {
                validate_reference_map(environment)?;
                output.insert("environment".to_owned(), environment.clone());
            }
            copy_local_options(source, &mut output)?;
        } else if let Some(url) = url {
            if source.get("command").is_some()
                || source.get("args").is_some()
                || source.get("environment").is_some()
                || source.get("env").is_some()
                || source.get("cwd").is_some()
            {
                return Err("remote MCP contains local-only fields");
            }
            if url.contains("token=") || url.contains("secret=") {
                return Err("the MCP URL contains literal credential material");
            }
            if let Some(kind) = source.get("type").and_then(serde_json::Value::as_str)
                && kind != "remote"
            {
                return Err("the source remote transport is not the OpenCode remote transport");
            }
            output.insert(
                "type".to_owned(),
                serde_json::Value::String("remote".to_owned()),
            );
            output.insert("url".to_owned(), serde_json::Value::String(url.to_owned()));
            if let Some(headers) = source.get("headers") {
                validate_reference_map(headers)?;
                output.insert("headers".to_owned(), headers.clone());
            }
            if source.get("oauth").and_then(serde_json::Value::as_bool) == Some(false) {
                output.insert("oauth".to_owned(), serde_json::Value::Bool(false));
            }
            copy_remote_options(source, &mut output)?;
        }
        Ok(serde_json::Value::Object(output))
    }
}

fn copy_local_options(
    source: &serde_json::Map<String, serde_json::Value>,
    output: &mut serde_json::Map<String, serde_json::Value>,
) -> Result<(), &'static str> {
    if let Some(cwd) = source.get("cwd") {
        let cwd = cwd.as_str().ok_or("MCP cwd must be a string")?;
        if path_depends_on_source(cwd) {
            return Err("MCP cwd depends on the source plugin root");
        }
        output.insert("cwd".to_owned(), serde_json::Value::String(cwd.to_owned()));
    }
    copy_common_options(source, output)
}

fn copy_remote_options(
    source: &serde_json::Map<String, serde_json::Value>,
    output: &mut serde_json::Map<String, serde_json::Value>,
) -> Result<(), &'static str> {
    copy_common_options(source, output)
}

fn copy_common_options(
    source: &serde_json::Map<String, serde_json::Value>,
    output: &mut serde_json::Map<String, serde_json::Value>,
) -> Result<(), &'static str> {
    if let Some(enabled) = source.get("enabled") {
        if !enabled.is_boolean() {
            return Err("MCP enabled must be boolean");
        }
        output.insert("enabled".to_owned(), enabled.clone());
    }
    if let Some(timeout) = source.get("timeout") {
        if timeout.as_u64().is_none_or(|value| value == 0) {
            return Err("MCP timeout must be a positive integer");
        }
        output.insert("timeout".to_owned(), timeout.clone());
    }
    Ok(())
}

fn optional_non_empty_string(
    value: Option<&serde_json::Value>,
) -> Result<Option<&str>, &'static str> {
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
        .ok_or("MCP reference map must be an object")?;
    if object
        .values()
        .any(|value| !value.as_str().is_some_and(is_reference))
    {
        return Err("literal MCP secret/environment material is not portable");
    }
    Ok(())
}

fn is_reference(value: &str) -> bool {
    (value.starts_with('$') && value.len() > 1)
        || (value.starts_with("{env:") && value.ends_with('}') && value.len() > 6)
        || (value.starts_with("${") && value.ends_with('}') && value.len() > 3)
}

fn path_depends_on_source(value: &str) -> bool {
    value.starts_with("./")
        || value.starts_with("../")
        || value.contains("${CLAUDE_PLUGIN_ROOT}")
        || value.contains("${CODEX_PLUGIN_ROOT}")
        || value.contains("PLUGIN_ROOT")
}

fn string_value_depends_on_source(value: &serde_json::Value) -> bool {
    value.as_str().is_some_and(path_depends_on_source)
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
    fn codec_maps_local_and_remote_servers_to_open_code_schema() {
        let local = OpenCodeMcpCodec::encode_server(&serde_json::json!({
            "command": "node",
            "args": ["server.mjs"],
            "env": {"TOKEN": "{env:MCP_TOKEN}"},
            "enabled": true,
            "timeout": 1000
        }))
        .unwrap();
        assert_eq!(local["type"], "local");
        assert_eq!(local["command"], serde_json::json!(["node", "server.mjs"]));
        assert_eq!(local["environment"]["TOKEN"], "{env:MCP_TOKEN}");

        let remote = OpenCodeMcpCodec::encode_server(&serde_json::json!({
            "url": "https://example.invalid/mcp",
            "headers": {"Authorization": "${MCP_AUTH}"},
            "enabled": false
        }))
        .unwrap();
        assert_eq!(remote["type"], "remote");
        assert_eq!(remote["url"], "https://example.invalid/mcp");
        assert_eq!(remote["headers"]["Authorization"], "${MCP_AUTH}");
    }

    #[test]
    fn codec_rejects_ambiguous_transports_secrets_oauth_and_unattested_filters() {
        for value in [
            serde_json::json!({"command":"node", "url":"https://example.invalid"}),
            serde_json::json!({"command":"node", "env":{"TOKEN":"literal"}}),
            serde_json::json!({"url":"https://example.invalid", "oauth": {}}),
            serde_json::json!({"url":"https://example.invalid", "tools":{"foo":false}}),
            serde_json::json!({"command":"./server"}),
        ] {
            assert!(
                OpenCodeMcpCodec::encode_server(&value).is_err(),
                "accepted {value}"
            );
        }
    }

    #[test]
    fn codec_preserves_only_documented_native_options() {
        let encoded = OpenCodeMcpCodec::encode_server(&serde_json::json!({
            "type":"remote",
            "url":"https://example.invalid/mcp",
            "headers":{"X-Key":"$KEY"},
            "enabled":true,
            "timeout":5000,
            "unknown":"must not be copied"
        }))
        .unwrap();
        assert!(encoded.get("unknown").is_none());
        assert_eq!(encoded["headers"]["X-Key"], "$KEY");
    }
}
