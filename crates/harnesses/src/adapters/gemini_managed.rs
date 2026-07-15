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
    file_managed::read_complete_source_plugin,
};

const MARKETPLACE_DOCUMENTS: &[&str] = &[
    ".agents/plugins/marketplace.json",
    ".claude-plugin/marketplace.json",
];
const MCP_SOURCE_DOCUMENTS: &[&str] = &[".codex-plugin/mcp.json", ".mcp.json", "mcp.json"];
const SETTINGS_DESTINATION: &str = ".gemini/settings.json";
const MCP_CONTAINER: &str = "mcpServers";
const SKILL_POLICY: SkillProjectionPolicy =
    SkillProjectionPolicy::complete_tree(SkillProjectionDiagnostics {
        missing_declared_name: "A Gemini plugin skill has no declared name.",
        required_missing_tree: "A required Gemini plugin skill is missing its complete directory.",
        unsafe_destination: SkillProjectionDestinationError::PluginMissing {
            detail: "A Gemini plugin skill name is not a safe destination.",
        },
        incomplete_tree: "A Gemini plugin skill is not a complete artifact tree.",
        missing_top_level_skill: "A required Gemini plugin skill is missing top-level SKILL.md.",
        invalid_agent_skill_name: None,
        invalid_contract: None,
        observed_tree_invalid: "The Gemini managed skill tree is invalid.",
        observe_safely_failed: "The Gemini managed skill tree could not be observed safely.",
        drifted: "An owned Gemini skill projection is missing or was replaced.",
    });

static PROJECTION: GeminiManagedProjection = GeminiManagedProjection;

/// Gemini's managed projection owns only Gemini's documented JSON destination
/// and skill-root choices. Source acquisition and transaction semantics remain
/// in the shared lifecycle.
pub struct GeminiManagedProjection;

impl GeminiManagedProjection {
    pub fn static_ref() -> &'static dyn ManagedProjectionPort {
        &PROJECTION
    }
}

impl ManagedProjectionPort for GeminiManagedProjection {
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
    let (skill_root, config_root) = destination_paths(context)?;
    let (trees, mut current_parts, mut desired_parts, skill_manifest) =
        plan_skills_with_policy(&skill_root, context, plugin.as_ref(), SKILL_POLICY)?;
    let (mcp_write, mcp_manifest) = plan_mcp(
        &config_root,
        context,
        plugin.as_ref(),
        (&mut current_parts, &mut desired_parts),
    )?;

    if trees.is_empty() && mcp_write.is_none() {
        return Err(ManagedProjectionError::Other {
            code: "managed_project_plugin_unsupported",
            summary: "The plugin has no faithful Gemini skill or MCP projection.",
        });
    }

    let removal = matches!(context.input, ManagedProjectionInput::Remove);
    let mut manifest = if removal {
        Vec::new()
    } else {
        let mut values = skill_manifest;
        values.extend(mcp_manifest);
        values
    };
    manifest.sort();
    manifest.dedup();

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
) -> Result<(AbsolutePath, AbsolutePath), ManagedProjectionError> {
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
        summary: "The Gemini managed skill destination is invalid.",
    })?;

    let config_root = match context.scope {
        skilltap_core::domain::Scope::Global => context.paths.home().clone(),
        skilltap_core::domain::Scope::Project(project) => project.clone(),
    };
    Ok((skill_root, config_root))
}

fn read_selected_plugin(
    context: &ManagedProjectionContext<'_>,
    checkout: &skilltap_core::managed_projection::ResolvedSourceCheckout,
) -> Result<super::file_managed::CompleteSourcePlugin, ManagedProjectionError> {
    let selector = skilltap_core::marketplace::PluginSelector::parse(context.request.name.as_str())
        .map_err(|_| ManagedProjectionError::PluginSourceInvalid {
            detail: "The selected Gemini plugin selector is invalid.",
        })?;
    let catalog =
        read_marketplace_catalog(context.filesystem, checkout.root(), context.json_limits)?;
    let plugin_root = catalog
        .plugin_source(selector.plugin(), checkout.root())
        .map_err(|_| ManagedProjectionError::PluginSourceInvalid {
            detail: "The selected Gemini plugin source is not a contained local marketplace entry.",
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
                detail: "The selected Gemini marketplace document path is invalid.",
            }
        })?;
        let bytes = filesystem
            .read_regular_bounded_no_follow(root, &destination, limits.bytes())
            .map_err(|_| ManagedProjectionError::CatalogInvalid {
                detail: "The selected Gemini marketplace document could not be read safely.",
            })?;
        if let Some(bytes) = bytes {
            return ManagedCodexCatalog::parse(&bytes, limits).map_err(|_| {
                ManagedProjectionError::CatalogInvalid {
                    detail: "The selected Gemini marketplace document is invalid.",
                }
            });
        }
    }
    Err(ManagedProjectionError::CatalogMissing)
}

fn plan_mcp(
    config_root: &AbsolutePath,
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
    let declaration_map = plugin.map_or(&[][..], |plugin| plugin.declarations.as_slice());
    let destination = RelativeArtifactPath::new(SETTINGS_DESTINATION).map_err(|_| {
        ManagedProjectionError::McpInvalid {
            detail: "The Gemini settings path is invalid.",
        }
    })?;
    let expected = context
        .filesystem
        .read_regular_bounded_no_follow(config_root, &destination, context.json_limits.bytes())
        .map_err(|_| ManagedProjectionError::McpInvalid {
            detail: "The Gemini settings document could not be read safely.",
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
            detail: "A Gemini MCP server name is invalid.",
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
                    detail: "An owned Gemini MCP server is missing or was replaced.",
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
        let mapped = match map_gemini_server(source) {
            Ok(mapped) => mapped,
            Err(_) => {
                if is_required_mcp(declaration_map, &name) {
                    return Err(ManagedProjectionError::RequiredUnsupported);
                }
                manifest.push(ManagedProjection::Omitted {
                    id: ComponentId::new(format!("mcp:{name}")).map_err(|_| {
                        ManagedProjectionError::McpInvalid {
                            detail: "An omitted Gemini MCP server name is invalid.",
                        }
                    })?,
                    consequence: skilltap_core::domain::EvidenceCode::new(
                        "unsupported_optional_component_omitted",
                    )
                    .expect("static evidence code is valid"),
                });
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
                    detail: "The Gemini settings document could not be encoded.",
                })?;
            bytes.push(b'\n');
            Some(bytes)
        };
    Ok((
        Some(ManagedFileWrite {
            root: config_root.clone(),
            destination,
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
            detail: "The selected Gemini MCP declaration is invalid JSON.",
        }
    })?;
    let servers = value
        .value()
        .get(MCP_CONTAINER)
        .and_then(serde_json::Value::as_object)
        .ok_or(ManagedProjectionError::McpInvalid {
            detail: "The selected Gemini MCP declaration has no mcpServers object.",
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
            detail: "The existing Gemini settings document is invalid JSON.",
        })?
        .value()
        .as_object()
        .cloned()
        .ok_or(ManagedProjectionError::McpInvalid {
            detail: "The existing Gemini settings document must be a JSON object.",
        })
}

fn is_required_mcp(declarations: &[ComponentDeclaration], name: &str) -> bool {
    declarations.iter().any(|declaration| {
        declaration.kind == ComponentKind::McpServer
            && declaration.declared_name.as_deref() == Some(name)
            && declaration.requiredness == ComponentRequiredness::Required
    })
}

fn map_gemini_server(value: &serde_json::Value) -> Result<serde_json::Value, &'static str> {
    let server = value.as_object().ok_or("server must be an object")?;
    let command = non_empty_string(server.get("command"))?;
    let url = non_empty_string(server.get("url"))?;
    let http_url = non_empty_string(server.get("httpUrl"))?;
    let transport_count = usize::from(command.is_some())
        + usize::from(url.is_some())
        + usize::from(http_url.is_some());
    if transport_count != 1 {
        return Err("server transport is absent or ambiguous");
    }
    if let Some(kind) = server.get("type").and_then(serde_json::Value::as_str) {
        let expected = if command.is_some() {
            "stdio"
        } else if url.is_some() {
            "sse"
        } else {
            "http"
        };
        if kind != expected {
            return Err("server transport type is not faithful");
        }
    } else if server.get("type").is_some() {
        return Err("server transport type is not a string");
    }
    if command.is_some_and(path_depends_on_source)
        || url.is_some_and(path_depends_on_source)
        || http_url.is_some_and(path_depends_on_source)
    {
        return Err("server endpoint depends on the source plugin root");
    }
    validate_string_array(server.get("args"))?;
    validate_string_array(server.get("includeTools"))?;
    validate_string_array(server.get("excludeTools"))?;
    if server.get("args").is_some_and(array_depends_on_source)
        || server.get("cwd").is_some_and(string_depends_on_source)
    {
        return Err("server path depends on the source plugin root");
    }
    validate_reference_map(server.get("env"))?;
    validate_reference_map(server.get("headers"))?;
    validate_optional_string(server.get("cwd"))?;
    validate_optional_number(server.get("timeout"))?;
    for key in ["targetAudience", "targetServiceAccount", "authProviderType"] {
        validate_optional_string(server.get(key))?;
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
            .ok_or("server transport is not a non-empty string"),
    }
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

fn path_depends_on_source(value: &str) -> bool {
    value.starts_with("./")
        || value.starts_with("../")
        || value.contains("${CLAUDE_PLUGIN_ROOT}")
        || value.contains("${CODEX_PLUGIN_ROOT}")
        || value.contains("${GEMINI_EXTENSION_PATH}")
        || value.contains("PLUGIN_ROOT")
}

fn string_depends_on_source(value: &serde_json::Value) -> bool {
    value.as_str().is_some_and(path_depends_on_source)
}

fn array_depends_on_source(value: &serde_json::Value) -> bool {
    value
        .as_array()
        .is_some_and(|values| values.iter().any(string_depends_on_source))
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
    fn gemini_codec_accepts_documented_stdio_sse_and_http_shapes() {
        for value in [
            serde_json::json!({"command":"node","args":["server.mjs"],"env":{"TOKEN":"$TOKEN"}}),
            serde_json::json!({"type":"sse","url":"https://example.invalid/sse","headers":{"Authorization":"${TOKEN}"}}),
            serde_json::json!({"type":"http","httpUrl":"https://example.invalid/mcp"}),
        ] {
            assert_eq!(map_gemini_server(&value).unwrap(), value);
        }
    }

    #[test]
    fn gemini_codec_rejects_ambiguous_relative_and_literal_secret_values() {
        for value in [
            serde_json::json!({"command":"./server"}),
            serde_json::json!({"command":"node","args":["${CLAUDE_PLUGIN_ROOT}/server"]}),
            serde_json::json!({"command":"node","env":{"TOKEN":"literal"}}),
            serde_json::json!({"url":"https://example.invalid","httpUrl":"https://example.invalid/mcp"}),
            serde_json::json!({"type":"http","url":"https://example.invalid"}),
        ] {
            assert!(
                map_gemini_server(&value).is_err(),
                "unexpectedly accepted {value}"
            );
        }
    }

    #[test]
    fn unknown_settings_and_servers_are_not_rewritten_by_codec_helpers() {
        let settings = parse_json_object(
            br#"{"future":{"enabled":true},"mcpServers":{"unmanaged":{"command":"keep"}}}"#,
            JsonLimits::new(4096, 16).unwrap(),
        )
        .unwrap();
        assert_eq!(settings["future"]["enabled"], true);
        assert_eq!(settings[MCP_CONTAINER]["unmanaged"]["command"], "keep");
    }
}
