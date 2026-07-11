//! Native MCP projection mapping for explicit plugin sources.

use std::collections::BTreeSet;

use skilltap_core::{
    domain::{AbsolutePath, ComponentKind, HarnessId, NativeId, ResourceComponent},
    materialization::{McpProjection, McpProjectionMapper, McpTransport, ProjectionError},
    plugin_graph::ComponentProvenance,
    runtime::{
        ExternalTreeLimits, ExternalTreeObserver, ExternalTreeRequest, JsonLimits, StrictJson,
        StrictJsonDecoder, SystemExternalTreeObserver,
    },
};

/// Maps a documented MCP JSON file inside an explicit source checkout. The
/// mapper is read-only; publication belongs to the core publish boundary.
pub struct JsonMcpProjectionMapper {
    root: AbsolutePath,
    tree_limits: ExternalTreeLimits,
    json_limits: JsonLimits,
}

impl JsonMcpProjectionMapper {
    pub const fn new(
        root: AbsolutePath,
        tree_limits: ExternalTreeLimits,
        json_limits: JsonLimits,
    ) -> Self {
        Self {
            root,
            tree_limits,
            json_limits,
        }
    }
}

impl McpProjectionMapper for JsonMcpProjectionMapper {
    fn map(
        &self,
        component: &ResourceComponent,
        provenance: &ComponentProvenance,
        target: &HarnessId,
    ) -> Result<McpProjection, ProjectionError> {
        if component.kind != ComponentKind::McpServer {
            return Err(ProjectionError::ComponentKindMismatch {
                component: component.id.clone(),
            });
        }
        if !matches!(target.as_str(), "codex" | "claude") {
            return Err(ProjectionError::UnsupportedMcp {
                component: component.id.clone(),
                reason: "target has no documented MCP load path",
            });
        }
        let snapshot = SystemExternalTreeObserver
            .observe(&ExternalTreeRequest::new(
                self.root.clone(),
                self.tree_limits,
            ))
            .map_err(|_| ProjectionError::UnsupportedMcp {
                component: component.id.clone(),
                reason: "MCP source is unavailable",
            })?;
        let bytes = snapshot
            .entries()
            .iter()
            .find(|entry| entry.path() == provenance.relative_path())
            .and_then(|entry| entry.file_bytes())
            .ok_or_else(|| ProjectionError::UnsupportedMcp {
                component: component.id.clone(),
                reason: "MCP declaration file is unavailable",
            })?;
        let decoded = StrictJson.decode(bytes, self.json_limits).map_err(|_| {
            ProjectionError::UnsupportedMcp {
                component: component.id.clone(),
                reason: "MCP declaration is malformed",
            }
        })?;
        let server = decoded
            .value()
            .get("mcpServers")
            .and_then(serde_json::Value::as_object)
            .and_then(|servers| {
                provenance
                    .declared_name()
                    .and_then(|name| servers.get(name))
            })
            .and_then(serde_json::Value::as_object)
            .ok_or_else(|| ProjectionError::UnsupportedMcp {
                component: component.id.clone(),
                reason: "named MCP server is missing",
            })?;
        let has_command = server
            .get("command")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| !value.is_empty());
        let has_url = server
            .get("url")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| !value.is_empty());
        let transport = match (has_command, has_url) {
            (true, false) => McpTransport::Stdio,
            (false, true) => McpTransport::Http,
            _ => {
                return Err(ProjectionError::UnsupportedMcp {
                    component: component.id.clone(),
                    reason: "MCP transport is absent or ambiguous",
                });
            }
        };
        if let Some(declared_type) = server.get("type").and_then(serde_json::Value::as_str) {
            let type_matches = match transport {
                McpTransport::Stdio => declared_type == "stdio",
                McpTransport::Http => declared_type == "http",
            };
            if !type_matches {
                return Err(ProjectionError::UnsupportedMcp {
                    component: component.id.clone(),
                    reason: "declared MCP transport type is not a faithful equivalent",
                });
            }
        }
        let mut credential_references = BTreeSet::new();
        collect_references(server, "env", &component.id, &mut credential_references)?;
        collect_references(server, "headers", &component.id, &mut credential_references)?;
        Ok(McpProjection {
            component: component.id.clone(),
            target: target.clone(),
            destination: provenance.relative_path().clone(),
            transport,
            credential_references,
        })
    }
}

fn collect_references(
    server: &serde_json::Map<String, serde_json::Value>,
    field: &str,
    component: &skilltap_core::domain::ComponentId,
    references: &mut BTreeSet<String>,
) -> Result<(), ProjectionError> {
    let Some(values) = server.get(field) else {
        return Ok(());
    };
    let Some(values) = values.as_object() else {
        return Err(ProjectionError::UnsupportedMcp {
            component: component.clone(),
            reason: "MCP credential references are not an object",
        });
    };
    for (name, value) in values {
        if NativeId::new(name).is_err()
            || !value
                .as_str()
                .is_some_and(|value| value.starts_with('$') || value.starts_with("${"))
        {
            return Err(ProjectionError::UnsupportedMcp {
                component: component.clone(),
                reason: "literal MCP credential values are not portable",
            });
        }
        references.insert(format!("{field}:{name}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeSet, fs};

    use skilltap_core::{
        domain::{
            ComponentId, ComponentRequiredness, RelativeArtifactPath, Source, SourceKind,
            SourceLocator,
        },
        plugin_graph::{ComponentDeclaration, normalize},
        runtime::{ExternalTreeLimits, JsonLimits},
    };
    use skilltap_test_support::TempRoot;

    use super::*;

    fn source_graph(root: &std::path::Path) -> skilltap_core::plugin_graph::SourceComponentGraph {
        normalize(
            Source::new(
                SourceKind::Local,
                SourceLocator::new(root.to_str().unwrap()).unwrap(),
                None,
            )
            .unwrap(),
            [ComponentDeclaration {
                id: ComponentId::new("mcp:docs").unwrap(),
                kind: ComponentKind::McpServer,
                requiredness: ComponentRequiredness::Optional,
                dependencies: BTreeSet::new(),
                relative_path: RelativeArtifactPath::new(".mcp.json").unwrap(),
                declared_name: Some("docs".to_owned()),
            }],
        )
        .unwrap()
    }

    fn mapper(root: &std::path::Path) -> JsonMcpProjectionMapper {
        JsonMcpProjectionMapper::new(
            AbsolutePath::new(root.to_str().unwrap()).unwrap(),
            ExternalTreeLimits::new(8, 64, 8_192, 32_768, 1_024).unwrap(),
            JsonLimits::new(8_192, 16).unwrap(),
        )
    }

    #[test]
    fn named_http_mcp_server_preserves_references_without_values() {
        let root = TempRoot::new("skilltap-mcp-projection").unwrap();
        fs::write(
            root.join(".mcp.json"),
            br#"{"mcpServers":{"docs":{"url":"https://mcp.example","headers":{"Authorization":"$MCP_TOKEN"}}}}"#,
        )
        .unwrap();
        let graph = source_graph(root.path());
        let component = graph
            .components()
            .get(&ComponentId::new("mcp:docs").unwrap())
            .unwrap();
        let provenance = graph
            .provenance(&ComponentId::new("mcp:docs").unwrap())
            .unwrap();
        let projection = mapper(root.path())
            .map(component, provenance, &HarnessId::new("codex").unwrap())
            .unwrap();
        assert_eq!(projection.transport, McpTransport::Http);
        assert_eq!(
            projection.credential_references,
            ["headers:Authorization".to_owned()].into_iter().collect()
        );
    }

    #[test]
    fn literal_mcp_credentials_and_ambiguous_transport_are_blocked() {
        let root = TempRoot::new("skilltap-mcp-projection-invalid").unwrap();
        fs::write(
            root.join(".mcp.json"),
            br#"{"mcpServers":{"docs":{"command":"docs","env":{"TOKEN":"secret"}}}}"#,
        )
        .unwrap();
        let graph = source_graph(root.path());
        let component = graph
            .components()
            .get(&ComponentId::new("mcp:docs").unwrap())
            .unwrap();
        let provenance = graph
            .provenance(&ComponentId::new("mcp:docs").unwrap())
            .unwrap();
        assert!(matches!(
            mapper(root.path()).map(component, provenance, &HarnessId::new("claude").unwrap()),
            Err(ProjectionError::UnsupportedMcp { .. })
        ));
    }

    #[test]
    fn non_http_declared_transport_is_not_treated_as_http() {
        let root = TempRoot::new("skilltap-mcp-projection-transport").unwrap();
        fs::write(
            root.join(".mcp.json"),
            br#"{"mcpServers":{"docs":{"type":"sse","url":"https://mcp.example"}}}"#,
        )
        .unwrap();
        let graph = source_graph(root.path());
        let component = graph
            .components()
            .get(&ComponentId::new("mcp:docs").unwrap())
            .unwrap();
        let provenance = graph
            .provenance(&ComponentId::new("mcp:docs").unwrap())
            .unwrap();
        assert!(matches!(
            mapper(root.path()).map(component, provenance, &HarnessId::new("codex").unwrap()),
            Err(ProjectionError::UnsupportedMcp { .. })
        ));
    }
}
