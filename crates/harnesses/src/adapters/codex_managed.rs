use std::{collections::BTreeSet, io};

use skilltap_core::{
    domain::{AbsolutePath, ArtifactFile, ComponentKind, NativeId, RelativeArtifactPath},
    instructions::fingerprint_contents,
    managed_projection::{
        ManagedFileWrite, ManagedPluginWrite, ManagedProjectionError, ManagedProjectionPlan,
    },
    plugin_graph::{ComponentDeclaration, PluginGraphReader},
    runtime::{
        ConfinedFileSystem, ExternalTreeLimits, ExternalTreeObserver, ExternalTreeRequest,
        JsonLimits, SystemExternalTreeObserver,
    },
    storage::{ArtifactTree, ManagedProjection},
};

use crate::{
    CodexPluginGraphReader,
    managed_codex_project::ManagedCodexCatalog,
    managed_projection::{ManagedProjectionContext, ManagedProjectionInput, ManagedProjectionPort},
};

pub(crate) const CODEX_CATALOG_DESTINATIONS: &[&str] = &[
    ".agents/plugins/marketplace.json",
    ".claude-plugin/marketplace.json",
];
pub(crate) const CODEX_MCP_DESTINATION: &str = ".codex/config.toml";

const MCP_TABLE_NAME: &str = "mcp_servers";
const PLUGIN_ROOT_RELATIVE_MCP_OMITTED: &str = "plugin_root_relative_mcp_omitted";

static MANAGED_PROJECTION: CodexManagedProjection = CodexManagedProjection;

/// Codex's managed project projection codec and documented destination rules.
pub struct CodexManagedProjection;

impl CodexManagedProjection {
    pub fn static_ref() -> &'static dyn ManagedProjectionPort {
        &MANAGED_PROJECTION
    }
}

impl ManagedProjectionPort for CodexManagedProjection {
    fn plan(
        &self,
        context: &ManagedProjectionContext<'_>,
    ) -> Result<ManagedProjectionPlan, ManagedProjectionError> {
        match context.resource_kind {
            skilltap_core::domain::ResourceKind::Marketplace => plan_marketplace(context),
            skilltap_core::domain::ResourceKind::Plugin => plan_plugin(context),
            _ => Err(adapter_error(
                "managed_project_resource_invalid",
                "Only Codex project marketplace and plugin resources can use this managed lifecycle.",
            )),
        }
    }
}

fn plan_marketplace(
    context: &ManagedProjectionContext<'_>,
) -> Result<ManagedProjectionPlan, ManagedProjectionError> {
    let destination = relative_path(
        CODEX_CATALOG_DESTINATIONS[0],
        "managed_project_path_invalid",
        "The managed Codex project catalog path is invalid.",
    )?;
    let current = context
        .filesystem
        .read_regular_bounded_no_follow(context.project, &destination, context.json_limits.bytes())
        .map_err(|_| {
            adapter_error(
                "managed_project_catalog_unreadable",
                "The Codex project catalog could not be read safely.",
            )
        })?;
    let desired = match &context.input {
        ManagedProjectionInput::Apply { checkout } => {
            let catalog = read_codex_catalog_at_root(
                context.filesystem,
                checkout.root(),
                context.json_limits,
            )?;
            Some(
                catalog
                    .into_bytes()
                    .map_err(|_| ManagedProjectionError::CatalogInvalid {
                        detail: "The selected Codex marketplace is invalid.",
                    })?,
            )
        }
        ManagedProjectionInput::Remove => None,
    };

    Ok(ManagedProjectionPlan {
        trees: Vec::new(),
        files: vec![ManagedFileWrite {
            root: context.project.clone(),
            destination,
            expected: current.clone(),
            desired: desired.clone(),
        }],
        manifest: Vec::new(),
        current_fingerprint: current.as_deref().map(fingerprint_contents),
        desired_fingerprint: desired.as_deref().map(fingerprint_contents),
    })
}

fn plan_plugin(
    context: &ManagedProjectionContext<'_>,
) -> Result<ManagedProjectionPlan, ManagedProjectionError> {
    let plugin = match &context.input {
        ManagedProjectionInput::Apply { checkout } => {
            let selector =
                skilltap_core::marketplace::PluginSelector::parse(context.request.name.as_str())
                    .map_err(|_| {
                        adapter_error(
                            "invalid_plugin_selector",
                            "The managed project plugin selector is invalid.",
                        )
                    })?;
            let catalog = read_codex_catalog_at_root(
                context.filesystem,
                checkout.root(),
                context.json_limits,
            )?;
            let plugin_root = catalog
                .plugin_source(selector.plugin(), checkout.root())
                .map_err(|_| ManagedProjectionError::PluginSourceInvalid {
                    detail:
                        "The selected plugin source is not a contained local marketplace entry.",
                })?;
            Some(read_complete_codex_plugin(
                &plugin_root,
                checkout.source(),
                context.json_limits,
            )?)
        }
        ManagedProjectionInput::Remove => None,
    };
    let removal = matches!(context.input, ManagedProjectionInput::Remove);
    let (tree, declarations) = plugin.as_ref().map_or((None, &[][..]), |plugin| {
        (Some(&plugin.tree), plugin.declarations.as_slice())
    });
    plan_codex_component_projections(
        context.project,
        context.filesystem,
        tree,
        declarations,
        context.prior,
        removal,
        context.acknowledged,
    )
}

fn read_codex_catalog_at_root(
    filesystem: &dyn ConfinedFileSystem,
    root: &AbsolutePath,
    limits: JsonLimits,
) -> Result<ManagedCodexCatalog, ManagedProjectionError> {
    for relative in CODEX_CATALOG_DESTINATIONS {
        let destination = relative_path(
            relative,
            "managed_project_source_invalid",
            "The marketplace document path is invalid.",
        )?;
        if let Some(bytes) = filesystem
            .read_regular_bounded_no_follow(root, &destination, limits.bytes())
            .map_err(|_| {
                adapter_error(
                    "managed_project_source_unreadable",
                    "The selected marketplace document could not be read safely.",
                )
            })?
        {
            return ManagedCodexCatalog::parse(&bytes, limits).map_err(|_| {
                ManagedProjectionError::CatalogInvalid {
                    detail: "The selected marketplace document is invalid.",
                }
            });
        }
    }
    Err(ManagedProjectionError::CatalogMissing)
}

fn plan_codex_component_projections(
    project: &AbsolutePath,
    filesystem: &dyn ConfinedFileSystem,
    plugin: Option<&ArtifactTree>,
    declarations: &[ComponentDeclaration],
    prior: &[ManagedProjection],
    removal: bool,
    acknowledged: bool,
) -> Result<ManagedProjectionPlan, ManagedProjectionError> {
    let mut skill_names = BTreeSet::new();
    let mut unsupported_optional = BTreeSet::new();
    for declaration in declarations {
        match declaration.kind {
            ComponentKind::Skill => {
                if let Some(name) = &declaration.declared_name {
                    skill_names.insert(name.clone());
                }
            }
            ComponentKind::McpServer => {}
            _ => {
                unsupported_optional.insert(declaration.id.clone());
            }
        }
    }
    for projection in prior {
        if let ManagedProjection::Skill { id, .. } = projection {
            skill_names.insert(id.as_str().to_owned());
        }
    }
    if !unsupported_optional.is_empty() && !acknowledged && !removal {
        return Err(adapter_error(
            "partial_operation_requires_acknowledgment",
            "The plugin has optional components outside Codex project skill/MCP load paths; rerun with `--yes` to accept their omission.",
        ));
    }

    let root = AbsolutePath::new(format!("{}/.agents/skills", project.as_str())).map_err(|_| {
        adapter_error(
            "managed_project_plugin_path_invalid",
            "The project skill root is invalid.",
        )
    })?;
    let mut trees = Vec::new();
    let mut current_parts = Vec::new();
    let mut desired_parts = Vec::new();
    for name in skill_names {
        let prefix = format!("skills/{name}/");
        let desired_files = plugin
            .into_iter()
            .flat_map(ArtifactTree::files)
            .filter_map(|(path, file)| {
                path.as_str()
                    .strip_prefix(&prefix)
                    .map(|relative| (relative.to_owned(), file.clone()))
            })
            .collect::<Vec<_>>();
        let desired_tree = if desired_files.is_empty() {
            None
        } else {
            Some(ArtifactTree::new(desired_files).map_err(|_| {
                ManagedProjectionError::PluginMissing {
                    detail: "A plugin skill is not a complete artifact tree.",
                }
            })?)
        };
        if desired_tree.as_ref().is_some_and(|tree| {
            !tree
                .files()
                .contains_key(&RelativeArtifactPath::new("SKILL.md").expect("static path is valid"))
        }) {
            return Err(ManagedProjectionError::PluginMissing {
                detail: "A required plugin skill is missing top-level SKILL.md.",
            });
        }
        let destination = RelativeArtifactPath::new(name).map_err(|_| {
            adapter_error(
                "managed_project_plugin_path_invalid",
                "A plugin skill name is not a safe destination.",
            )
        })?;
        let current = observe_managed_project_tree(filesystem, &root, &destination)?.and_then(
            |(identity, files)| {
                ArtifactTree::new(
                    files
                        .into_iter()
                        .map(|(path, file)| (path.as_str().to_owned(), file)),
                )
                .ok()
                .map(|tree| (identity, tree))
            },
        );
        if let Some(expected) = prior.iter().find_map(|projection| match projection {
            ManagedProjection::Skill { id, fingerprint } if id == &destination => Some(fingerprint),
            _ => None,
        }) {
            let observed = current.as_ref().map(|(_, tree)| {
                let mut bytes = Vec::new();
                append_projection_tree(&mut bytes, &destination, tree);
                fingerprint_contents(&bytes)
            });
            if observed.as_ref() != Some(expected) {
                return Err(ManagedProjectionError::Drifted {
                    detail: "An owned project skill projection is missing or was replaced.",
                });
            }
        }
        if let Some((_, tree)) = &current {
            append_projection_tree(&mut current_parts, &destination, tree);
        }
        if !removal && let Some(tree) = &desired_tree {
            append_projection_tree(&mut desired_parts, &destination, tree);
        }
        trees.push(ManagedPluginWrite {
            root: root.clone(),
            destination,
            desired_tree: (!removal).then_some(desired_tree).flatten(),
            expected_tree: current.as_ref().map(|(_, tree)| tree.clone()),
            expected_identity: current.map(|(identity, _)| identity),
        });
    }

    let (mcp_write, mcp_manifest) = plan_codex_mcp_config(
        project,
        filesystem,
        plugin,
        prior,
        removal,
        acknowledged,
        (&mut current_parts, &mut desired_parts),
    )?;
    if trees.is_empty() && mcp_write.is_none() {
        return Err(adapter_error(
            "managed_project_plugin_unsupported",
            "The plugin has no faithful project skill or MCP projection.",
        ));
    }
    let mut manifest = if removal {
        Vec::new()
    } else {
        let mut manifest = managed_projection_manifest(&trees, &mcp_manifest);
        manifest.extend(unsupported_optional.into_iter().map(|id| {
            ManagedProjection::Omitted {
                id,
                consequence: skilltap_core::domain::EvidenceCode::new(
                    "unsupported_optional_component_omitted",
                )
                .expect("static evidence code is valid"),
            }
        }));
        manifest
    };
    manifest.sort();
    manifest.dedup();

    Ok(ManagedProjectionPlan {
        trees,
        files: mcp_write.into_iter().collect(),
        manifest,
        current_fingerprint: (!current_parts.is_empty())
            .then(|| fingerprint_contents(&current_parts)),
        desired_fingerprint: (!removal).then(|| fingerprint_contents(&desired_parts)),
    })
}

type ObservedManagedProjectTree = (
    skilltap_core::runtime::DirectoryIdentity,
    std::collections::BTreeMap<RelativeArtifactPath, ArtifactFile>,
);

fn observe_managed_project_tree(
    filesystem: &dyn ConfinedFileSystem,
    root: &AbsolutePath,
    destination: &RelativeArtifactPath,
) -> Result<Option<ObservedManagedProjectTree>, ManagedProjectionError> {
    match filesystem.load_tree_bounded_no_follow(
        root,
        destination,
        managed_project_tree_observation_limits(),
    ) {
        Ok(tree) => Ok(Some(tree)),
        Err(skilltap_core::runtime::RuntimeError::FileSystem { source, .. })
            if source.kind() == io::ErrorKind::NotFound =>
        {
            Ok(None)
        }
        Err(_) => Err(ManagedProjectionError::PluginUnreadable {
            detail: "The managed project skill tree could not be observed within its safety limits.",
        }),
    }
}

fn managed_project_tree_observation_limits() -> ExternalTreeLimits {
    ExternalTreeLimits::new(64, 100_000, 64 * 1024 * 1024, 1024 * 1024 * 1024, 64 * 1024)
        .expect("bounded project tree limits are valid")
}

fn managed_projection_manifest(
    trees: &[ManagedPluginWrite],
    mcp: &[ManagedProjection],
) -> Vec<ManagedProjection> {
    let mut manifest = mcp.to_vec();
    for tree in trees {
        if let Some(desired) = &tree.desired_tree {
            let mut bytes = Vec::new();
            append_projection_tree(&mut bytes, &tree.destination, desired);
            manifest.push(ManagedProjection::Skill {
                id: tree.destination.clone(),
                fingerprint: fingerprint_contents(&bytes),
            });
        }
    }
    manifest
}

fn append_projection_tree(
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

fn plan_codex_mcp_config(
    project: &AbsolutePath,
    filesystem: &dyn ConfinedFileSystem,
    plugin: Option<&ArtifactTree>,
    prior: &[ManagedProjection],
    removal: bool,
    acknowledged: bool,
    fingerprint_parts: (&mut Vec<u8>, &mut Vec<u8>),
) -> Result<(Option<ManagedFileWrite>, Vec<ManagedProjection>), ManagedProjectionError> {
    let (current_parts, desired_parts) = fingerprint_parts;
    let mcp_file = [".codex-plugin/mcp.json", ".mcp.json", "mcp.json"]
        .iter()
        .find_map(|path| plugin?.files().get(&RelativeArtifactPath::new(*path).ok()?));
    let value: serde_json::Value = mcp_file
        .map_or_else(
            || Ok(serde_json::json!({"mcpServers": {}})),
            |mcp_file| serde_json::from_slice(mcp_file.contents()),
        )
        .map_err(|_| ManagedProjectionError::McpInvalid {
            detail: "The plugin MCP declaration is invalid JSON.",
        })?;
    let servers = value
        .get("mcpServers")
        .and_then(serde_json::Value::as_object)
        .ok_or(ManagedProjectionError::McpInvalid {
            detail: "The plugin MCP declaration has no mcpServers object.",
        })?;
    let destination = relative_path(
        CODEX_MCP_DESTINATION,
        "managed_project_mcp_path_invalid",
        "The project MCP config path is invalid.",
    )?;
    let expected = filesystem
        .read_regular_bounded_no_follow(project, &destination, 256 * 1024)
        .map_err(|_| {
            adapter_error(
                "managed_project_mcp_unreadable",
                "The project MCP config could not be read safely.",
            )
        })?;
    let mut document = match expected.as_deref() {
        Some(bytes) => std::str::from_utf8(bytes)
            .ok()
            .and_then(|text| text.parse::<toml::Table>().ok())
            .ok_or(ManagedProjectionError::McpInvalid {
                detail: "The existing project config is not valid TOML.",
            })?,
        None => toml::Table::new(),
    };
    let mcp_servers = document
        .entry(MCP_TABLE_NAME)
        .or_insert_with(|| toml::Value::Table(toml::Table::new()))
        .as_table_mut()
        .ok_or(ManagedProjectionError::McpConflict)?;
    let mut compatible_servers = 0usize;
    let mut names = servers.keys().cloned().collect::<BTreeSet<_>>();
    names.extend(prior.iter().filter_map(|projection| match projection {
        ManagedProjection::Mcp { id, .. } => Some(id.as_str().to_owned()),
        _ => None,
    }));
    let mut manifest = Vec::new();
    for name in names {
        let server = servers.get(&name);
        let native_id = NativeId::new(&name).map_err(|_| ManagedProjectionError::McpInvalid {
            detail: "An MCP server name is invalid.",
        })?;
        if !removal && server.is_some_and(mcp_depends_on_plugin_root) {
            if !acknowledged {
                return Err(adapter_error(
                    "partial_operation_requires_acknowledgment",
                    "An MCP server depends on a plugin-root-relative executable that cannot be projected faithfully; rerun with `--yes` to omit it.",
                ));
            }
            if let Ok(id) = skilltap_core::domain::ComponentId::new(format!("mcp:{name}")) {
                manifest.push(ManagedProjection::Omitted {
                    id,
                    consequence: skilltap_core::domain::EvidenceCode::new(
                        PLUGIN_ROOT_RELATIVE_MCP_OMITTED,
                    )
                    .expect("static evidence code is valid"),
                });
            }
            continue;
        }
        compatible_servers += 1;
        if let Some(current) = mcp_servers.get(&name) {
            current_parts.extend(toml::to_string(current).unwrap_or_default().into_bytes());
        }
        if let Some(expected_projection) = prior.iter().find_map(|projection| match projection {
            ManagedProjection::Mcp { id, fingerprint } if id.as_str() == name => Some(fingerprint),
            _ => None,
        }) {
            let observed = mcp_servers.get(&name).map(|current| {
                fingerprint_contents(toml::to_string(current).unwrap_or_default().as_bytes())
            });
            if observed.as_ref() != Some(expected_projection) {
                return Err(ManagedProjectionError::Drifted {
                    detail: "An owned project MCP projection is missing or was replaced.",
                });
            }
        }
        if removal {
            mcp_servers.remove(&name);
        } else if let Some(server) = server {
            let mapped = json_to_toml(server).ok_or(ManagedProjectionError::McpInvalid {
                detail: "An MCP server contains an unsupported value.",
            })?;
            desired_parts.extend(toml::to_string(&mapped).unwrap_or_default().into_bytes());
            manifest.push(ManagedProjection::Mcp {
                id: native_id,
                fingerprint: fingerprint_contents(
                    toml::to_string(&mapped).unwrap_or_default().as_bytes(),
                ),
            });
            mcp_servers.insert(name, mapped);
        } else {
            mcp_servers.remove(&name);
        }
    }
    if compatible_servers == 0 {
        return Ok((None, manifest));
    }
    if mcp_servers.is_empty() {
        document.remove(MCP_TABLE_NAME);
    }
    let desired = if document.is_empty() {
        None
    } else {
        Some(
            toml::to_string_pretty(&document)
                .map_err(|_| ManagedProjectionError::McpInvalid {
                    detail: "The project MCP config could not be encoded.",
                })?
                .into_bytes(),
        )
    };
    Ok((
        Some(ManagedFileWrite {
            root: project.clone(),
            destination,
            expected,
            desired,
        }),
        manifest,
    ))
}

fn mcp_depends_on_plugin_root(server: &serde_json::Value) -> bool {
    let has_placeholder = |value: &str| {
        value.contains("PLUGIN_ROOT")
            || value.contains("${CLAUDE_PLUGIN_ROOT}")
            || value.contains("${CODEX_PLUGIN_ROOT}")
    };
    server
        .get("command")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|value| {
            has_placeholder(value) || value.starts_with("./") || value.starts_with("../")
        })
        || server
            .get("args")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|values| {
                values
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .any(|value| {
                        has_placeholder(value)
                            || value.starts_with("./")
                            || value.starts_with("../")
                    })
            })
}

fn json_to_toml(value: &serde_json::Value) -> Option<toml::Value> {
    Some(match value {
        serde_json::Value::String(value) => toml::Value::String(value.clone()),
        serde_json::Value::Bool(value) => toml::Value::Boolean(*value),
        serde_json::Value::Number(value) => toml::Value::Integer(value.as_i64()?),
        serde_json::Value::Array(values) => toml::Value::Array(
            values
                .iter()
                .map(json_to_toml)
                .collect::<Option<Vec<_>>>()?,
        ),
        serde_json::Value::Object(values) => toml::Value::Table(
            values
                .iter()
                .map(|(key, value)| Some((key.clone(), json_to_toml(value)?)))
                .collect::<Option<toml::Table>>()?,
        ),
        serde_json::Value::Null => return None,
    })
}

fn read_complete_codex_plugin(
    root: &AbsolutePath,
    source: &skilltap_core::domain::Source,
    json_limits: JsonLimits,
) -> Result<CompleteCodexPlugin, ManagedProjectionError> {
    let tree_limits =
        ExternalTreeLimits::new(64, 100_000, 64 * 1024 * 1024, 1024 * 1024 * 1024, 64 * 1024)
            .expect("bounded plugin tree limits are valid");
    let declarations = CodexPluginGraphReader::new(root.clone(), tree_limits, json_limits)
        .read(source)
        .map_err(|_| ManagedProjectionError::PluginMissing {
            detail:
                "The selected plugin does not contain a valid Codex manifest and complete component graph.",
        })?;
    let snapshot = SystemExternalTreeObserver
        .observe(&ExternalTreeRequest::new(root.clone(), tree_limits))
        .map_err(|_| ManagedProjectionError::PluginUnreadable {
            detail: "The selected plugin tree could not be read safely.",
        })?;
    let mut files = Vec::new();
    for entry in snapshot.entries() {
        match entry.kind() {
            skilltap_core::runtime::ExternalTreeEntryKind::Directory => {}
            skilltap_core::runtime::ExternalTreeEntryKind::File => files.push((
                entry.path().as_str().to_owned(),
                ArtifactFile::new(
                    entry.file_bytes().unwrap_or_default().to_vec(),
                    entry.file_executable().unwrap_or(false),
                ),
            )),
            skilltap_core::runtime::ExternalTreeEntryKind::Symlink => {
                return Err(adapter_error(
                    "managed_project_plugin_symlink",
                    "Managed project plugins cannot contain symlinks.",
                ));
            }
        }
    }
    let tree = ArtifactTree::new(files).map_err(|_| ManagedProjectionError::PluginMissing {
        detail: "The selected plugin tree is invalid.",
    })?;
    Ok(CompleteCodexPlugin { tree, declarations })
}

struct CompleteCodexPlugin {
    tree: ArtifactTree,
    declarations: Vec<ComponentDeclaration>,
}

fn relative_path(
    value: &str,
    code: &'static str,
    summary: &'static str,
) -> Result<RelativeArtifactPath, ManagedProjectionError> {
    RelativeArtifactPath::new(value).map_err(|_| adapter_error(code, summary))
}

fn adapter_error(code: &'static str, summary: &'static str) -> ManagedProjectionError {
    ManagedProjectionError::Other { code, summary }
}
