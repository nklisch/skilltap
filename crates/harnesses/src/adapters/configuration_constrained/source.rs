use std::collections::{BTreeMap, BTreeSet};

use skilltap_core::{
    domain::{NativeId, RelativeArtifactPath},
    managed_projection::{ManagedProjectionError, ResolvedSourceCheckout},
    plugin_graph::ComponentDeclaration as GraphComponentDeclaration,
    runtime::{ConfinedFileSystem, StrictJson, StrictJsonDecoder},
    storage::ArtifactTree,
};

use super::super::file_managed::{CompleteSourcePlugin, read_complete_source_plugin};
use crate::managed_projection::ManagedProjectionContext;

/// A source MCP server after the family-level portability checks. Target
/// codecs consume this value but retain their own field names and syntax.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum PortableMcpServer {
    Stdio {
        command: String,
        args: Vec<String>,
        environment: BTreeMap<String, String>,
        cwd: Option<String>,
        enabled: bool,
        timeout_ms: Option<u64>,
        tools: Option<BTreeSet<String>>,
    },
    Remote {
        transport: PortableRemoteTransport,
        url: String,
        headers: BTreeMap<String, String>,
        authentication: AuthenticationRequirement,
        enabled: bool,
        timeout_ms: Option<u64>,
        tools: Option<BTreeSet<String>>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PortableRemoteTransport {
    Http,
    Sse,
    StreamableHttp,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AuthenticationRequirement {
    None,
    StaticReferences,
    OAuth,
}

#[derive(Debug)]
pub(crate) struct SelectedPortablePlugin {
    pub(crate) tree: ArtifactTree,
    pub(crate) declarations: Vec<GraphComponentDeclaration>,
    pub(crate) mcp: BTreeMap<NativeId, PortableMcpServer>,
}

/// Read exactly one selected plugin from an already resolved checkout. The
/// source catalog and plugin root are resolved by the caller's explicit
/// selector; no repository or marketplace traversal is performed here.
pub(crate) fn load_selected_plugin(
    context: &ManagedProjectionContext<'_>,
    checkout: &ResolvedSourceCheckout,
    catalog_documents: &[&str],
) -> Result<SelectedPortablePlugin, ManagedProjectionError> {
    let selector = skilltap_core::marketplace::PluginSelector::parse(context.request.name.as_str())
        .map_err(|_| ManagedProjectionError::PluginSourceInvalid {
            detail: "The selected plugin selector is invalid.",
        })?;
    let catalog = read_catalog(
        context.filesystem,
        checkout,
        context.json_limits,
        catalog_documents,
    )?;
    let plugin_root = catalog
        .plugin_source(selector.plugin(), checkout.root())
        .map_err(|_| ManagedProjectionError::PluginSourceInvalid {
            detail: "The selected plugin source is not a contained marketplace entry.",
        })?;
    let plugin = read_complete_source_plugin(&plugin_root, checkout.source(), context.json_limits)?;
    let mcp = normalize_mcp(&plugin, context.json_limits)?;
    Ok(SelectedPortablePlugin {
        tree: plugin.tree,
        declarations: plugin.declarations,
        mcp,
    })
}

fn read_catalog(
    filesystem: &dyn ConfinedFileSystem,
    checkout: &ResolvedSourceCheckout,
    limits: skilltap_core::runtime::JsonLimits,
    candidates: &[&str],
) -> Result<crate::ManagedCodexCatalog, ManagedProjectionError> {
    for candidate in candidates {
        let path = RelativeArtifactPath::new(*candidate).map_err(|_| {
            ManagedProjectionError::CatalogInvalid {
                detail: "The selected marketplace document path is invalid.",
            }
        })?;
        let bytes = filesystem
            .read_regular_bounded_no_follow(checkout.root(), &path, limits.bytes())
            .map_err(|_| ManagedProjectionError::CatalogInvalid {
                detail: "The selected marketplace document could not be read safely.",
            })?;
        if let Some(bytes) = bytes {
            return crate::ManagedCodexCatalog::parse(&bytes, limits).map_err(|_| {
                ManagedProjectionError::CatalogInvalid {
                    detail: "The selected marketplace document is invalid.",
                }
            });
        }
    }
    Err(ManagedProjectionError::CatalogMissing)
}

fn normalize_mcp(
    plugin: &CompleteSourcePlugin,
    limits: skilltap_core::runtime::JsonLimits,
) -> Result<BTreeMap<NativeId, PortableMcpServer>, ManagedProjectionError> {
    let Some(file) = [
        ".mcp.json",
        "mcp.json",
        ".claude-plugin/mcp.json",
        ".codex-plugin/mcp.json",
    ]
    .iter()
    .find_map(|path| {
        plugin
            .tree
            .files()
            .get(&RelativeArtifactPath::new(*path).ok()?)
    }) else {
        return Ok(BTreeMap::new());
    };
    let value = StrictJson.decode(file.contents(), limits).map_err(|_| {
        ManagedProjectionError::McpInvalid {
            detail: "The selected plugin MCP declaration is invalid JSON.",
        }
    })?;
    let servers = value
        .value()
        .get("mcpServers")
        .and_then(serde_json::Value::as_object)
        .ok_or(ManagedProjectionError::McpInvalid {
            detail: "The selected plugin MCP declaration has no mcpServers object.",
        })?;
    servers
        .iter()
        .map(|(name, value)| {
            let id = NativeId::new(name).map_err(|_| ManagedProjectionError::McpInvalid {
                detail: "A plugin MCP server name is invalid.",
            })?;
            Ok((id, normalize_server(value)?))
        })
        .collect()
}

fn normalize_server(
    value: &serde_json::Value,
) -> Result<PortableMcpServer, ManagedProjectionError> {
    let object = value
        .as_object()
        .ok_or(ManagedProjectionError::McpInvalid {
            detail: "A plugin MCP server must be an object.",
        })?;
    let command = optional_string(object.get("command"))?;
    let url = optional_string(object.get("url").or_else(|| object.get("uri")))?;
    if command.is_some() == url.is_some() {
        return Err(ManagedProjectionError::McpInvalid {
            detail: "A plugin MCP server must select exactly one transport.",
        });
    }
    let enabled = object
        .get("enabled")
        .and_then(serde_json::Value::as_bool)
        .or_else(|| {
            object
                .get("disabled")
                .and_then(serde_json::Value::as_bool)
                .map(|v| !v)
        })
        .unwrap_or(true);
    let timeout_ms = object
        .get("timeout_ms")
        .or_else(|| object.get("timeout"))
        .map(|value| {
            value.as_u64().ok_or(ManagedProjectionError::McpInvalid {
                detail: "A plugin MCP timeout must be an unsigned integer.",
            })
        })
        .transpose()?;
    let tools = object
        .get("tools")
        .or_else(|| object.get("includeTools"))
        .map(parse_string_set)
        .transpose()?;
    let args = object
        .get("args")
        .map(parse_strings)
        .transpose()?
        .unwrap_or_default();
    let environment = object
        .get("env")
        .or_else(|| object.get("environment"))
        .map(parse_references)
        .transpose()?
        .unwrap_or_default();
    let cwd = object
        .get("cwd")
        .map(|value| optional_string(Some(value)))
        .transpose()?
        .flatten()
        .map(str::to_owned);
    if let Some(command) = command {
        let command = reject_source_relative(command)?;
        return Ok(PortableMcpServer::Stdio {
            command: command.to_owned(),
            args: args
                .into_iter()
                .map(|arg| reject_source_relative(&arg).map(str::to_owned))
                .collect::<Result<_, _>>()?,
            environment,
            cwd: cwd
                .map(|value| reject_source_relative(&value).map(str::to_owned))
                .transpose()?,
            enabled,
            timeout_ms,
            tools,
        });
    }
    let url = reject_source_relative(url.expect("transport checked"))?;
    let headers = object
        .get("headers")
        .map(parse_references)
        .transpose()?
        .unwrap_or_default();
    let authentication = match object
        .get("auth")
        .and_then(serde_json::Value::as_object)
        .and_then(|auth| auth.get("type"))
        .and_then(serde_json::Value::as_str)
    {
        Some("oauth") => AuthenticationRequirement::OAuth,
        Some(_) => AuthenticationRequirement::StaticReferences,
        None if !headers.is_empty() => AuthenticationRequirement::StaticReferences,
        None => AuthenticationRequirement::None,
    };
    let transport = match object
        .get("transport")
        .or_else(|| object.get("type"))
        .and_then(serde_json::Value::as_str)
    {
        None | Some("http") | Some("remote") => PortableRemoteTransport::Http,
        Some("sse") => PortableRemoteTransport::Sse,
        Some("streamable-http") | Some("streamable_http") => {
            PortableRemoteTransport::StreamableHttp
        }
        Some(_) => {
            return Err(ManagedProjectionError::McpInvalid {
                detail: "A plugin MCP remote transport is unsupported.",
            });
        }
    };
    Ok(PortableMcpServer::Remote {
        transport,
        url: url.to_owned(),
        headers,
        authentication,
        enabled,
        timeout_ms,
        tools,
    })
}

fn optional_string(
    value: Option<&serde_json::Value>,
) -> Result<Option<&str>, ManagedProjectionError> {
    match value {
        None => Ok(None),
        Some(value) => value
            .as_str()
            .filter(|value| !value.is_empty())
            .map(Some)
            .ok_or(ManagedProjectionError::McpInvalid {
                detail: "A plugin MCP string field is invalid.",
            }),
    }
}

fn parse_strings(value: &serde_json::Value) -> Result<Vec<String>, ManagedProjectionError> {
    value
        .as_array()
        .ok_or(ManagedProjectionError::McpInvalid {
            detail: "A plugin MCP list field is invalid.",
        })?
        .iter()
        .map(|value| {
            value
                .as_str()
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .ok_or(ManagedProjectionError::McpInvalid {
                    detail: "A plugin MCP list contains a non-string.",
                })
        })
        .collect()
}

fn parse_string_set(value: &serde_json::Value) -> Result<BTreeSet<String>, ManagedProjectionError> {
    Ok(parse_strings(value)?.into_iter().collect())
}

fn parse_references(
    value: &serde_json::Value,
) -> Result<BTreeMap<String, String>, ManagedProjectionError> {
    let object = value
        .as_object()
        .ok_or(ManagedProjectionError::McpInvalid {
            detail: "A plugin MCP reference map is invalid.",
        })?;
    object
        .iter()
        .map(|(key, value)| {
            let value = value
                .as_str()
                .filter(|value| is_reference(value))
                .map(str::to_owned)
                .ok_or(ManagedProjectionError::McpInvalid {
                    detail: "Literal MCP credential material is unsupported.",
                })?;
            Ok((key.clone(), value))
        })
        .collect()
}

fn is_reference(value: &str) -> bool {
    (value.starts_with('$') && value.len() > 1)
        || (value.starts_with("${") && value.ends_with('}') && value.len() > 3)
        || (value.starts_with("{env:") && value.ends_with('}') && value.len() > 6)
}

fn reject_source_relative(value: &str) -> Result<&str, ManagedProjectionError> {
    if value.starts_with("./") || value.starts_with("../") || value.contains("PLUGIN_ROOT") {
        return Err(ManagedProjectionError::McpInvalid {
            detail: "A plugin MCP value depends on the source checkout.",
        });
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_credentials_are_references_and_oauth_is_explicit() {
        let stdio = normalize_server(&serde_json::json!({
            "command": "node",
            "args": ["server.js"],
            "env": {"TOKEN": "${TOKEN}"},
            "tools": ["read"],
        }))
        .unwrap();
        assert!(matches!(stdio, PortableMcpServer::Stdio { .. }));
        let oauth = normalize_server(&serde_json::json!({
            "url": "https://example.invalid/mcp",
            "auth": {"type": "oauth"},
        }))
        .unwrap();
        assert!(matches!(
            oauth,
            PortableMcpServer::Remote {
                authentication: AuthenticationRequirement::OAuth,
                ..
            }
        ));
        assert!(
            normalize_server(&serde_json::json!({
                "url": "https://example.invalid/mcp",
                "headers": {"Authorization": "Bearer secret"},
            }))
            .is_err()
        );
    }
}
