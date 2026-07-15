//! Native plugin source readers for the normalized core graph contract.

use std::collections::BTreeSet;

use skilltap_core::{
    domain::{
        AbsolutePath, ComponentId, ComponentKind, ComponentRequiredness, RelativeArtifactPath,
        Source, SourceKind,
    },
    plugin_graph::{ComponentDeclaration, PluginGraphReadError, PluginGraphReader},
    runtime::{
        ExternalTreeLimits, ExternalTreeObserver, ExternalTreeRequest, JsonLimits, StrictJson,
        StrictJsonDecoder, SystemExternalTreeObserver,
    },
};

/// Reads a Codex plugin from the explicitly selected source checkout.
pub struct CodexPluginGraphReader {
    root: AbsolutePath,
    tree_limits: ExternalTreeLimits,
    json_limits: JsonLimits,
}

impl CodexPluginGraphReader {
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

impl PluginGraphReader for CodexPluginGraphReader {
    fn read(&self, source: &Source) -> Result<Vec<ComponentDeclaration>, PluginGraphReadError> {
        NativePluginGraphReader {
            root: &self.root,
            manifest: ".codex-plugin/plugin.json",
            tree_limits: self.tree_limits,
            json_limits: self.json_limits,
        }
        .read(source)
    }
}

/// Reads a Claude Code plugin from the explicitly selected source checkout.
pub struct ClaudePluginGraphReader {
    root: AbsolutePath,
    tree_limits: ExternalTreeLimits,
    json_limits: JsonLimits,
}

impl ClaudePluginGraphReader {
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

impl PluginGraphReader for ClaudePluginGraphReader {
    fn read(&self, source: &Source) -> Result<Vec<ComponentDeclaration>, PluginGraphReadError> {
        NativePluginGraphReader {
            root: &self.root,
            manifest: ".claude-plugin/plugin.json",
            tree_limits: self.tree_limits,
            json_limits: self.json_limits,
        }
        .read(source)
    }
}

struct NativePluginGraphReader<'a> {
    root: &'a AbsolutePath,
    manifest: &'static str,
    tree_limits: ExternalTreeLimits,
    json_limits: JsonLimits,
}

impl PluginGraphReader for NativePluginGraphReader<'_> {
    fn read(&self, source: &Source) -> Result<Vec<ComponentDeclaration>, PluginGraphReadError> {
        if source.kind() == SourceKind::RemoteCatalog {
            return Err(PluginGraphReadError::UnsupportedSourceKind(
                SourceKind::RemoteCatalog,
            ));
        }
        let snapshot = SystemExternalTreeObserver
            .observe(&ExternalTreeRequest::new(
                self.root.clone(),
                self.tree_limits,
            ))
            .map_err(|_| PluginGraphReadError::SourceUnavailable)?;
        let manifest = snapshot
            .entries()
            .iter()
            .find(|entry| entry.path().as_str() == self.manifest)
            .and_then(|entry| entry.file_bytes())
            .ok_or(PluginGraphReadError::MalformedManifest)?;
        let decoded = StrictJson
            .decode(manifest, self.json_limits)
            .map_err(|_| PluginGraphReadError::MalformedManifest)?;
        if !decoded.value().is_object() {
            return Err(PluginGraphReadError::MalformedManifest);
        }
        declarations_from_snapshot(snapshot.entries(), self.json_limits)
    }
}

pub(crate) fn declarations_from_snapshot(
    entries: &[skilltap_core::runtime::ExternalTreeEntry],
    json_limits: JsonLimits,
) -> Result<Vec<ComponentDeclaration>, PluginGraphReadError> {
    let mut declarations = Vec::new();
    let mut seen = BTreeSet::new();
    append_mcp_declarations(entries, json_limits, &mut declarations, &mut seen)?;
    for entry in entries {
        let path = entry.path().as_str();
        let Some((root, name)) = one_child(path) else {
            continue;
        };
        let Some(kind) = component_kind(root) else {
            continue;
        };
        if root == "skills" {
            if entry.kind() != skilltap_core::runtime::ExternalTreeEntryKind::Directory {
                continue;
            }
            let skill_file = format!("skills/{name}/SKILL.md");
            if !entries
                .iter()
                .any(|candidate| candidate.path().as_str() == skill_file)
            {
                return Err(PluginGraphReadError::MalformedManifest);
            }
        }
        let id = ComponentId::new(format!("{}:{name}", component_prefix(&kind)))
            .map_err(|_| PluginGraphReadError::MalformedManifest)?;
        if !seen.insert(id.clone()) {
            return Err(PluginGraphReadError::MalformedManifest);
        }
        let requiredness = if kind == ComponentKind::Skill {
            ComponentRequiredness::Required
        } else {
            ComponentRequiredness::Optional
        };
        declarations.push(ComponentDeclaration {
            id,
            kind,
            requiredness,
            dependencies: BTreeSet::new(),
            relative_path: RelativeArtifactPath::new(path)
                .map_err(|_| PluginGraphReadError::MalformedManifest)?,
            declared_name: Some(name.to_owned()),
        });
    }
    Ok(declarations)
}

fn append_mcp_declarations(
    entries: &[skilltap_core::runtime::ExternalTreeEntry],
    json_limits: JsonLimits,
    declarations: &mut Vec<ComponentDeclaration>,
    seen: &mut BTreeSet<ComponentId>,
) -> Result<(), PluginGraphReadError> {
    for entry in entries.iter().filter(|entry| {
        matches!(
            entry.path().as_str(),
            "mcp.json"
                | ".mcp.json"
                | ".factory-plugin/mcp.json"
                | ".claude-plugin/mcp.json"
                | ".codex-plugin/mcp.json"
        )
    }) {
        let bytes = entry
            .file_bytes()
            .ok_or(PluginGraphReadError::MalformedManifest)?;
        let decoded = StrictJson
            .decode(bytes, json_limits)
            .map_err(|_| PluginGraphReadError::MalformedManifest)?;
        let Some(object) = decoded.value().as_object() else {
            return Err(PluginGraphReadError::MalformedManifest);
        };
        let names = object
            .get("mcpServers")
            .and_then(serde_json::Value::as_object)
            .map(|servers| servers.keys().map(String::as_str).collect::<Vec<_>>())
            .unwrap_or_else(|| vec!["default"]);
        for name in names {
            let id = ComponentId::new(format!("mcp:{name}"))
                .map_err(|_| PluginGraphReadError::MalformedManifest)?;
            if !seen.insert(id.clone()) {
                return Err(PluginGraphReadError::MalformedManifest);
            }
            declarations.push(ComponentDeclaration {
                id,
                kind: ComponentKind::McpServer,
                requiredness: ComponentRequiredness::Optional,
                dependencies: BTreeSet::new(),
                relative_path: entry.path().clone(),
                declared_name: Some(name.to_owned()),
            });
        }
    }
    Ok(())
}

fn one_child(path: &str) -> Option<(&str, &str)> {
    let (root, remainder) = path.split_once('/')?;
    if remainder.is_empty() || remainder.contains('/') {
        return None;
    }
    Some((root, remainder))
}

fn component_kind(root: &str) -> Option<ComponentKind> {
    Some(match root {
        "skills" => ComponentKind::Skill,
        "hooks" => ComponentKind::Hook,
        "agents" => ComponentKind::Agent,
        "commands" => ComponentKind::Command,
        "apps" => ComponentKind::App,
        "connectors" => ComponentKind::Connector,
        "lsp" | "lsp-servers" => ComponentKind::LspServer,
        "output-styles" | "output_styles" => ComponentKind::OutputStyle,
        "themes" => ComponentKind::Theme,
        "monitors" => ComponentKind::Monitor,
        "executables" => ComponentKind::Executable,
        "settings" => ComponentKind::Settings,
        _ => return None,
    })
}

fn component_prefix(kind: &ComponentKind) -> &'static str {
    match kind {
        ComponentKind::Skill => "skill",
        ComponentKind::McpServer => "mcp",
        ComponentKind::Hook => "hook",
        ComponentKind::Agent => "agent",
        ComponentKind::App => "app",
        ComponentKind::Connector => "connector",
        ComponentKind::LspServer => "lsp",
        ComponentKind::Command => "command",
        ComponentKind::OutputStyle => "output-style",
        ComponentKind::Theme => "theme",
        ComponentKind::Monitor => "monitor",
        ComponentKind::Executable => "executable",
        ComponentKind::Settings => "settings",
        ComponentKind::HarnessSpecific(_) => "harness",
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use skilltap_core::{
        domain::{SourceKind, SourceLocator},
        runtime::{ExternalTreeLimits, JsonLimits},
    };
    use skilltap_test_support::TempRoot;

    use super::*;

    fn limits() -> (ExternalTreeLimits, JsonLimits) {
        (
            ExternalTreeLimits::new(12, 128, 8_192, 32_768, 1_024).unwrap(),
            JsonLimits::new(8_192, 16).unwrap(),
        )
    }

    fn source(root: &Path) -> Source {
        Source::new(
            SourceKind::Local,
            SourceLocator::new(root.to_str().unwrap()).unwrap(),
            None,
        )
        .unwrap()
    }

    #[test]
    fn claude_reader_normalizes_complete_skills_and_known_components() {
        let root = TempRoot::new("skilltap-claude-plugin-graph").unwrap();
        fs::create_dir_all(root.join(".claude-plugin")).unwrap();
        fs::write(
            root.join(".claude-plugin/plugin.json"),
            br#"{"name":"demo","future":true}"#,
        )
        .unwrap();
        fs::create_dir_all(root.join("skills/example")).unwrap();
        fs::write(
            root.join("skills/example/SKILL.md"),
            b"---\nname: example\n---\n",
        )
        .unwrap();
        fs::create_dir_all(root.join("hooks")).unwrap();
        fs::write(root.join("hooks/notify.sh"), b"#!/bin/sh\n").unwrap();
        fs::write(
            root.join(".mcp.json"),
            br#"{"mcpServers":{"docs":{"command":"docs"}}}"#,
        )
        .unwrap();
        let (tree_limits, json_limits) = limits();
        let reader = ClaudePluginGraphReader::new(
            AbsolutePath::new(root.path().to_str().unwrap()).unwrap(),
            tree_limits,
            json_limits,
        );
        let declarations = reader.read(&source(root.path())).unwrap();
        assert!(declarations.iter().any(|value| {
            value.id.as_str() == "skill:example"
                && value.requiredness == ComponentRequiredness::Required
                && value.relative_path.as_str() == "skills/example"
        }));
        assert!(declarations.iter().any(|value| {
            value.id.as_str() == "hook:notify.sh"
                && value.requiredness == ComponentRequiredness::Optional
        }));
        assert!(declarations.iter().any(|value| {
            value.id.as_str() == "mcp:docs"
                && value.kind == ComponentKind::McpServer
                && value.relative_path.as_str() == ".mcp.json"
        }));
    }

    #[test]
    fn codex_reader_rejects_incomplete_skills_and_remote_catalogs() {
        let root = TempRoot::new("skilltap-codex-plugin-graph").unwrap();
        fs::create_dir_all(root.join(".codex-plugin")).unwrap();
        fs::write(
            root.join(".codex-plugin/plugin.json"),
            br#"{"name":"demo"}"#,
        )
        .unwrap();
        fs::create_dir_all(root.join("skills/missing-entry")).unwrap();
        let (tree_limits, json_limits) = limits();
        let reader = CodexPluginGraphReader::new(
            AbsolutePath::new(root.path().to_str().unwrap()).unwrap(),
            tree_limits,
            json_limits,
        );
        assert_eq!(
            reader.read(&source(root.path())),
            Err(PluginGraphReadError::MalformedManifest)
        );
        let remote = Source::new(
            SourceKind::RemoteCatalog,
            SourceLocator::new("https://example.test/catalog.json").unwrap(),
            None,
        )
        .unwrap();
        assert_eq!(
            reader.read(&remote),
            Err(PluginGraphReadError::UnsupportedSourceKind(
                SourceKind::RemoteCatalog
            ))
        );
    }
}
