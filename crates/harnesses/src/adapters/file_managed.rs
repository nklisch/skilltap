use skilltap_core::{
    domain::{AbsolutePath, ArtifactFile, Source},
    managed_projection::ManagedProjectionError,
    plugin_graph::PluginGraphReader,
    runtime::{
        ExternalTreeEntryKind, ExternalTreeLimits, ExternalTreeObserver, ExternalTreeRequest,
        JsonLimits, SystemExternalTreeObserver,
    },
    storage::ArtifactTree,
};

use crate::{ClaudePluginGraphReader, CodexPluginGraphReader};

/// A complete, explicitly selected source plugin normalized for destination
/// adapters. Native destination paths and MCP encodings remain outside this
/// shared reader.
#[derive(Debug)]
pub(crate) struct CompleteSourcePlugin {
    pub(crate) tree: ArtifactTree,
    pub(crate) declarations: Vec<skilltap_core::plugin_graph::ComponentDeclaration>,
}

/// Read one selected source checkout using either attested plugin manifest.
///
/// The source root is observed as one bounded, no-symlink tree after the
/// manifest reader validates the complete component graph. This keeps source
/// acquisition and completeness shared without flattening target-native
/// destination codecs.
pub(crate) fn read_complete_source_plugin(
    root: &AbsolutePath,
    source: &Source,
    json_limits: JsonLimits,
) -> Result<CompleteSourcePlugin, ManagedProjectionError> {
    let tree_limits =
        ExternalTreeLimits::new(64, 100_000, 64 * 1024 * 1024, 1024 * 1024 * 1024, 64 * 1024)
            .expect("bounded source tree limits are valid");
    let declarations = CodexPluginGraphReader::new(root.clone(), tree_limits, json_limits)
        .read(source)
        .or_else(|_| {
            ClaudePluginGraphReader::new(root.clone(), tree_limits, json_limits).read(source)
        })
        .map_err(|_| ManagedProjectionError::PluginMissing {
            detail: "The selected source does not contain a valid supported plugin manifest.",
        })?;
    let snapshot = SystemExternalTreeObserver
        .observe(&ExternalTreeRequest::new(root.clone(), tree_limits))
        .map_err(|_| ManagedProjectionError::PluginUnreadable {
            detail: "The selected plugin tree could not be read safely.",
        })?;
    let files = snapshot
        .entries()
        .iter()
        .filter_map(|entry| match entry.kind() {
            ExternalTreeEntryKind::Directory => None,
            ExternalTreeEntryKind::File => Some(Ok((
                entry.path().as_str().to_owned(),
                ArtifactFile::new(
                    entry.file_bytes().unwrap_or_default().to_vec(),
                    entry.file_executable().unwrap_or(false),
                ),
            ))),
            ExternalTreeEntryKind::Symlink => {
                Some(Err(ManagedProjectionError::PluginSourceInvalid {
                    detail: "Managed plugin projections cannot contain symlinks.",
                }))
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    let tree = ArtifactTree::new(files).map_err(|_| ManagedProjectionError::PluginMissing {
        detail: "The selected plugin tree is invalid.",
    })?;
    Ok(CompleteSourcePlugin { tree, declarations })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use skilltap_core::domain::{SourceKind, SourceLocator};
    use skilltap_test_support::TempRoot;

    use super::*;

    fn limits() -> JsonLimits {
        JsonLimits::new(8_192, 16).unwrap()
    }

    fn source(root: &TempRoot) -> Source {
        Source::new(
            SourceKind::Local,
            SourceLocator::new(root.path().to_str().unwrap()).unwrap(),
            None,
        )
        .unwrap()
    }

    #[test]
    fn reads_complete_skill_tree_and_mcp_from_codex_source() {
        let root = TempRoot::new("skilltap-file-managed-source").unwrap();
        fs::create_dir_all(root.join(".codex-plugin")).unwrap();
        fs::write(
            root.join(".codex-plugin/plugin.json"),
            br#"{"name":"demo"}"#,
        )
        .unwrap();
        fs::write(
            root.join(".codex-plugin/mcp.json"),
            br#"{"mcpServers":{"docs":{"command":"docs"}}}"#,
        )
        .unwrap();
        fs::create_dir_all(root.join("skills/demo/scripts")).unwrap();
        fs::write(root.join("skills/demo/SKILL.md"), b"---\nname: demo\n---\n").unwrap();
        fs::write(root.join("skills/demo/scripts/run.sh"), b"#!/bin/sh\n").unwrap();

        let plugin = read_complete_source_plugin(
            &AbsolutePath::new(root.path().to_str().unwrap()).unwrap(),
            &source(&root),
            limits(),
        )
        .unwrap();

        assert!(
            plugin
                .declarations
                .iter()
                .any(|declaration| declaration.id.as_str() == "skill:demo")
        );
        assert!(
            plugin
                .declarations
                .iter()
                .any(|declaration| declaration.id.as_str() == "mcp:docs")
        );
        assert!(plugin.tree.files().contains_key(
            &skilltap_core::domain::RelativeArtifactPath::new("skills/demo/SKILL.md").unwrap()
        ));
        assert!(
            plugin
                .tree
                .files()
                .get(
                    &skilltap_core::domain::RelativeArtifactPath::new("skills/demo/scripts/run.sh")
                        .unwrap()
                )
                .is_some_and(|file| !file.is_executable())
        );
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinks_in_the_selected_source_tree() {
        use std::os::unix::fs::symlink;

        let root = TempRoot::new("skilltap-file-managed-symlink").unwrap();
        fs::create_dir_all(root.join(".claude-plugin")).unwrap();
        fs::write(
            root.join(".claude-plugin/plugin.json"),
            br#"{"name":"demo"}"#,
        )
        .unwrap();
        fs::create_dir_all(root.join("skills/demo")).unwrap();
        fs::write(root.join("skills/demo/SKILL.md"), b"---\nname: demo\n---\n").unwrap();
        fs::write(root.join("outside"), b"outside").unwrap();
        symlink(root.join("outside"), root.join("skills/demo/link")).unwrap();

        let error = read_complete_source_plugin(
            &AbsolutePath::new(root.path().to_str().unwrap()).unwrap(),
            &source(&root),
            limits(),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            ManagedProjectionError::PluginSourceInvalid { .. }
        ));
    }
}
