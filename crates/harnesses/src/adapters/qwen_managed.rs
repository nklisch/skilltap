use std::{
    collections::{BTreeMap, BTreeSet},
    io,
};

use skilltap_core::{
    domain::{
        AbsolutePath, ArtifactFile, ComponentId, ComponentKind, ComponentRequiredness, NativeId,
        RelativeArtifactPath, ResourceKind, Source, SourceKind,
    },
    instructions::fingerprint_contents,
    managed_projection::{
        ManagedFileWrite, ManagedPluginWrite, ManagedProjectionError, ManagedProjectionPlan,
    },
    plugin_graph::{ComponentDeclaration, PluginGraphReadError, PluginGraphReader},
    runtime::{
        ConfinedFileSystem, ExternalTreeEntryKind, ExternalTreeLimits, ExternalTreeObserver,
        ExternalTreeRequest, JsonLimits, StrictJson, StrictJsonDecoder, SystemExternalTreeObserver,
    },
    storage::{ArtifactTree, ManagedProjection},
};

use crate::{
    managed_codex_project::ManagedCodexCatalog,
    managed_projection::{ManagedProjectionContext, ManagedProjectionInput, ManagedProjectionPort},
    plugin_graph::{ClaudePluginGraphReader, declarations_from_snapshot},
};

const MCP_CONTAINER: &str = "mcpServers";
const MCP_SOURCE_DOCUMENTS: &[&str] = &[
    ".mcp.json",
    "mcp.json",
    ".claude-plugin/mcp.json",
    ".codex-plugin/mcp.json",
];
const MARKETPLACE_DOCUMENTS: &[&str] = &[
    ".agents/plugins/marketplace.json",
    ".claude-plugin/marketplace.json",
];
static PROJECTION: QwenManagedProjection = QwenManagedProjection;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum QwenSourceFlavor {
    Qwen,
    Claude,
    Gemini,
}

#[derive(Debug)]
pub(crate) struct QwenSourcePlugin {
    pub(crate) plugin: CompleteSourcePlugin,
    pub(crate) flavor: QwenSourceFlavor,
}

#[derive(Debug)]
pub(crate) struct CompleteSourcePlugin {
    pub(crate) tree: ArtifactTree,
    pub(crate) declarations: Vec<ComponentDeclaration>,
}

/// Read only the explicitly selected source shapes Qwen documents. The
/// checkout is already resolved by the caller; this function never invokes
/// Qwen, downloads a package, or treats an arbitrary directory as native.
pub(crate) fn read_qwen_source_plugin(
    _filesystem: &dyn ConfinedFileSystem,
    root: &AbsolutePath,
    source: &Source,
    limits: JsonLimits,
) -> Result<QwenSourcePlugin, crate::NativeDistributionError> {
    if !source_locator_is_admitted(source) {
        return Err(crate::NativeDistributionError::UnsupportedSource);
    }
    let tree_limits =
        ExternalTreeLimits::new(64, 100_000, 64 * 1024 * 1024, 1024 * 1024 * 1024, 64 * 1024)
            .expect("bounded Qwen source limits are valid");
    let snapshot = SystemExternalTreeObserver
        .observe(&ExternalTreeRequest::new(root.clone(), tree_limits))
        .map_err(|_| crate::NativeDistributionError::SourceUnavailable)?;
    let (manifest, flavor) = [
        ("qwen-extension.json", QwenSourceFlavor::Qwen),
        ("gemini-extension.json", QwenSourceFlavor::Gemini),
        (".claude-plugin/plugin.json", QwenSourceFlavor::Claude),
        ("package.json", QwenSourceFlavor::Qwen),
    ]
    .into_iter()
    .find(|(path, _)| {
        snapshot
            .entries()
            .iter()
            .any(|entry| entry.path().as_str() == *path)
    })
    .ok_or(crate::NativeDistributionError::UnsupportedSource)?;
    let manifest_bytes = snapshot
        .entries()
        .iter()
        .find(|entry| entry.path().as_str() == manifest)
        .and_then(|entry| entry.file_bytes())
        .ok_or(crate::NativeDistributionError::MalformedSource)?;
    if !StrictJson
        .decode(manifest_bytes, limits)
        .map_err(|_| crate::NativeDistributionError::MalformedSource)?
        .value()
        .is_object()
    {
        return Err(crate::NativeDistributionError::MalformedSource);
    }
    let mut declarations = if flavor == QwenSourceFlavor::Claude {
        ClaudePluginGraphReader::new(root.clone(), tree_limits, limits)
            .read(source)
            .map_err(|error| match error {
                PluginGraphReadError::SourceUnavailable => {
                    crate::NativeDistributionError::SourceUnavailable
                }
                _ => crate::NativeDistributionError::MalformedSource,
            })?
    } else {
        declarations_from_snapshot(snapshot.entries(), limits)
            .map_err(|_| crate::NativeDistributionError::MalformedSource)?
    };
    append_context_declarations(snapshot.entries(), &mut declarations)?;
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
                Some(Err(crate::NativeDistributionError::MalformedSource))
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    let tree =
        ArtifactTree::new(files).map_err(|_| crate::NativeDistributionError::MalformedSource)?;
    Ok(QwenSourcePlugin {
        plugin: CompleteSourcePlugin { tree, declarations },
        flavor,
    })
}

fn source_locator_is_admitted(source: &Source) -> bool {
    if source.kind() != SourceKind::RemoteCatalog {
        return true;
    }
    let locator = source.locator().as_str().to_ascii_lowercase();
    locator.starts_with("npm:")
        || locator.starts_with("npm://")
        || locator.ends_with(".tgz")
        || locator.ends_with(".tar.gz")
        || locator.ends_with(".zip")
        || locator.ends_with(".tar")
}

fn append_context_declarations(
    entries: &[skilltap_core::runtime::ExternalTreeEntry],
    declarations: &mut Vec<ComponentDeclaration>,
) -> Result<(), crate::NativeDistributionError> {
    let mut names = BTreeSet::new();
    for entry in entries {
        let Some(remainder) = entry.path().as_str().strip_prefix("context/") else {
            continue;
        };
        let Some(name) = remainder.split('/').next() else {
            continue;
        };
        if name.is_empty()
            || entry.kind() != ExternalTreeEntryKind::File
            || !names.insert(name.to_owned())
        {
            continue;
        }
        let id = ComponentId::new(format!("context:{name}"))
            .map_err(|_| crate::NativeDistributionError::MalformedSource)?;
        declarations.push(ComponentDeclaration {
            id,
            kind: ComponentKind::HarnessSpecific(
                NativeId::new("context").expect("static component kind is valid"),
            ),
            requiredness: ComponentRequiredness::Optional,
            dependencies: BTreeSet::new(),
            relative_path: RelativeArtifactPath::new(format!("context/{name}"))
                .map_err(|_| crate::NativeDistributionError::MalformedSource)?,
            declared_name: Some(name.to_owned()),
        });
    }
    Ok(())
}

pub struct QwenManagedProjection;

impl QwenManagedProjection {
    pub fn static_ref() -> &'static dyn ManagedProjectionPort {
        &PROJECTION
    }
}

impl ManagedProjectionPort for QwenManagedProjection {
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
            summary: "The plugin has no faithful Qwen skill or MCP projection.",
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
    let (skill_root, config_root, config_destination) = match context.scope {
        skilltap_core::domain::Scope::Global => (
            AbsolutePath::new(format!("{}/.qwen/skills", context.paths.home().as_str())),
            context.paths.home().clone(),
            RelativeArtifactPath::new(".qwen/settings.json"),
        ),
        skilltap_core::domain::Scope::Project(project) => (
            AbsolutePath::new(format!("{}/.qwen/skills", project.as_str())),
            project.clone(),
            RelativeArtifactPath::new(".qwen/settings.json"),
        ),
    };
    let skill_root = skill_root.map_err(|_| ManagedProjectionError::Other {
        code: "managed_destination_invalid",
        summary: "The Qwen managed skill destination is invalid.",
    })?;
    let config_destination =
        config_destination.map_err(|_| ManagedProjectionError::McpInvalid {
            detail: "The Qwen settings path is invalid.",
        })?;
    Ok((skill_root, config_root, config_destination))
}

fn read_selected_plugin(
    context: &ManagedProjectionContext<'_>,
    checkout: &skilltap_core::managed_projection::ResolvedSourceCheckout,
) -> Result<CompleteSourcePlugin, ManagedProjectionError> {
    let selector = skilltap_core::marketplace::PluginSelector::parse(context.request.name.as_str())
        .map_err(|_| ManagedProjectionError::PluginSourceInvalid {
            detail: "The selected Qwen plugin selector is invalid.",
        })?;
    let catalog =
        read_marketplace_catalog(context.filesystem, checkout.root(), context.json_limits)?;
    let plugin_root = catalog
        .plugin_source(selector.plugin(), checkout.root())
        .map_err(|_| ManagedProjectionError::PluginSourceInvalid {
            detail: "The selected Qwen plugin source is not a contained local marketplace entry.",
        })?;
    read_qwen_source_plugin(
        context.filesystem,
        &plugin_root,
        checkout.source(),
        context.json_limits,
    )
    .map(|source| source.plugin)
    .map_err(|_| ManagedProjectionError::PluginSourceInvalid {
        detail: "The selected Qwen plugin source is not a supported complete directory.",
    })
}

fn read_marketplace_catalog(
    filesystem: &dyn ConfinedFileSystem,
    root: &AbsolutePath,
    limits: JsonLimits,
) -> Result<ManagedCodexCatalog, ManagedProjectionError> {
    for path in MARKETPLACE_DOCUMENTS {
        let destination = RelativeArtifactPath::new(*path).map_err(|_| {
            ManagedProjectionError::CatalogInvalid {
                detail: "The selected Qwen marketplace document path is invalid.",
            }
        })?;
        let bytes = filesystem
            .read_regular_bounded_no_follow(root, &destination, limits.bytes())
            .map_err(|_| ManagedProjectionError::CatalogInvalid {
                detail: "The selected Qwen marketplace document could not be read safely.",
            })?;
        if let Some(bytes) = bytes {
            return ManagedCodexCatalog::parse(&bytes, limits).map_err(|_| {
                ManagedProjectionError::CatalogInvalid {
                    detail: "The selected Qwen marketplace document is invalid.",
                }
            });
        }
    }
    Err(ManagedProjectionError::CatalogMissing)
}

type SkillPlan = (
    Vec<ManagedPluginWrite>,
    Vec<u8>,
    Vec<u8>,
    Vec<ManagedProjection>,
);
type ObservedTree = (skilltap_core::runtime::DirectoryIdentity, ArtifactTree);

fn plan_skills(
    skill_root: &AbsolutePath,
    context: &ManagedProjectionContext<'_>,
    plugin: Option<&CompleteSourcePlugin>,
) -> Result<SkillPlan, ManagedProjectionError> {
    let removal = matches!(context.input, ManagedProjectionInput::Remove);
    let declarations = plugin.map_or(&[][..], |plugin| plugin.declarations.as_slice());
    let mut names = BTreeSet::new();
    let mut omitted = Vec::new();
    for declaration in declarations {
        match declaration.kind {
            ComponentKind::Skill => names.insert(declaration.declared_name.clone().ok_or(
                ManagedProjectionError::PluginMissing {
                    detail: "A Qwen plugin skill has no declared name.",
                },
            )?),
            ComponentKind::McpServer => false,
            _ if declaration.requiredness == ComponentRequiredness::Required => {
                return Err(ManagedProjectionError::RequiredUnsupported);
            }
            _ => {
                omitted.push(ManagedProjection::Omitted {
                    id: declaration.id.clone(),
                    consequence: evidence("unsupported_optional_component_omitted"),
                });
                false
            }
        };
    }
    for projection in context.prior {
        if let ManagedProjection::Skill { id, .. } = projection {
            names.insert(id.as_str().to_owned());
        }
    }
    if !omitted.is_empty() && !context.acknowledged && !removal {
        return Err(ManagedProjectionError::Other {
            code: "partial_operation_requires_acknowledgment",
            summary: "The plugin contains optional components outside Qwen skill/MCP load paths; rerun with `--yes` to accept their omission.",
        });
    }
    let mut trees = Vec::new();
    let mut current_parts = Vec::new();
    let mut desired_parts = Vec::new();
    let mut manifest = omitted;
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
                detail: "A required Qwen plugin skill is missing its complete directory.",
            });
        }
        let destination = RelativeArtifactPath::new(&name).map_err(|_| {
            ManagedProjectionError::PluginMissing {
                detail: "A Qwen plugin skill name is not a safe destination.",
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
        detail: "A Qwen plugin skill is not a complete artifact tree.",
    })?;
    if !tree
        .files()
        .contains_key(&RelativeArtifactPath::new("SKILL.md").expect("static path is valid"))
    {
        return Err(ManagedProjectionError::PluginMissing {
            detail: "A Qwen plugin skill is missing top-level SKILL.md.",
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
                detail: "The Qwen managed skill tree is invalid.",
            })?,
        ))),
        Err(skilltap_core::runtime::RuntimeError::FileSystem { source, .. })
            if source.kind() == io::ErrorKind::NotFound =>
        {
            Ok(None)
        }
        Err(_) => Err(ManagedProjectionError::PluginUnreadable {
            detail: "The Qwen managed skill tree could not be observed safely.",
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
            detail: "An owned Qwen skill projection is missing or was replaced.",
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
            detail: "The Qwen settings document could not be read safely.",
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
            detail: "A Qwen MCP server name is invalid.",
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
                    detail: "An owned Qwen MCP server is missing or was replaced.",
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
        let mapped = match QwenMcpCodec::encode_server(source) {
            Ok(value) => value,
            Err(_) if is_required_mcp(declarations, &name) => {
                return Err(ManagedProjectionError::RequiredUnsupported);
            }
            Err(_) if !context.acknowledged && !removal => {
                return Err(ManagedProjectionError::Other {
                    code: "partial_operation_requires_acknowledgment",
                    summary: "The plugin contains an MCP server that cannot be projected faithfully to Qwen; rerun with `--yes` to accept its omission.",
                });
            }
            Err(_) => {
                manifest.push(ManagedProjection::Omitted {
                    id: ComponentId::new(format!("mcp:{name}")).map_err(|_| {
                        ManagedProjectionError::McpInvalid {
                            detail: "An omitted Qwen MCP server name is invalid.",
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
                    detail: "The Qwen settings document could not be encoded.",
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
            detail: "The selected Qwen MCP declaration is invalid JSON.",
        }
    })?;
    let servers = value
        .value()
        .get(MCP_CONTAINER)
        .and_then(serde_json::Value::as_object)
        .ok_or(ManagedProjectionError::McpInvalid {
            detail: "The selected Qwen MCP declaration has no mcpServers object.",
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
            detail: "The existing Qwen settings document is invalid JSON.",
        })?
        .value()
        .as_object()
        .cloned()
        .ok_or(ManagedProjectionError::McpInvalid {
            detail: "The existing Qwen settings document must be a JSON object.",
        })
}

fn is_required_mcp(declarations: &[ComponentDeclaration], name: &str) -> bool {
    declarations.iter().any(|declaration| {
        declaration.kind == ComponentKind::McpServer
            && declaration.declared_name.as_deref() == Some(name)
            && declaration.requiredness == ComponentRequiredness::Required
    })
}

pub(crate) struct QwenMcpCodec;
impl QwenMcpCodec {
    fn encode_server(value: &serde_json::Value) -> Result<serde_json::Value, &'static str> {
        let server = value.as_object().ok_or("MCP server must be an object")?;
        let command = non_empty_string(server.get("command"))?;
        let url = non_empty_string(server.get("url"))?;
        if usize::from(command.is_some()) + usize::from(url.is_some()) != 1 {
            return Err("MCP transport is absent or ambiguous");
        }
        if command.is_some_and(path_depends_on_source) || url.is_some_and(path_depends_on_source) {
            return Err("MCP endpoint depends on the source plugin root");
        }
        validate_string_array(server.get("args"))?;
        if server.get("args").is_some_and(array_depends_on_source)
            || server.get("cwd").is_some_and(string_depends_on_source)
        {
            return Err("MCP path depends on the source plugin root");
        }
        validate_reference_map(server.get("env"))?;
        validate_reference_map(server.get("headers"))?;
        if let Some(kind) = server.get("type") {
            let kind = kind.as_str().ok_or("MCP type is not a string")?;
            let valid = if command.is_some() {
                matches!(kind, "stdio" | "local")
            } else {
                matches!(kind, "http" | "sse" | "remote")
            };
            if !valid {
                return Err("MCP transport type is not faithful");
            }
        }
        if server
            .get("oauth")
            .is_some_and(|value| value != &serde_json::Value::Bool(false))
            || server
                .get("auth")
                .is_some_and(|value| !value.as_str().is_some_and(is_reference))
        {
            return Err("MCP authentication requires a native credential reference");
        }
        Ok(value.clone())
    }
}

fn non_empty_string(value: Option<&serde_json::Value>) -> Result<Option<&str>, &'static str> {
    match value {
        None => Ok(None),
        Some(value) => value
            .as_str()
            .filter(|value| !value.is_empty())
            .map(Some)
            .ok_or("MCP transport value must be a non-empty string"),
    }
}
fn validate_string_array(value: Option<&serde_json::Value>) -> Result<(), &'static str> {
    if let Some(value) = value {
        let values = value.as_array().ok_or("MCP list value is not an array")?;
        if !values.iter().all(serde_json::Value::is_string) {
            return Err("MCP list contains a non-string");
        }
    }
    Ok(())
}
fn validate_reference_map(value: Option<&serde_json::Value>) -> Result<(), &'static str> {
    if let Some(value) = value {
        let object = value
            .as_object()
            .ok_or("MCP reference map is not an object")?;
        if object
            .values()
            .any(|value| !value.as_str().is_some_and(is_reference))
        {
            return Err("MCP reference map contains literal credential material");
        }
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
        || value.contains("PLUGIN_ROOT")
        || value.contains("EXTENSION_PATH")
}
fn string_depends_on_source(value: &serde_json::Value) -> bool {
    value.as_str().is_some_and(path_depends_on_source)
}
fn array_depends_on_source(value: &serde_json::Value) -> bool {
    value
        .as_array()
        .is_some_and(|values| values.iter().any(string_depends_on_source))
}
fn evidence(code: &'static str) -> skilltap_core::domain::EvidenceCode {
    skilltap_core::domain::EvidenceCode::new(code).expect("static Qwen evidence code is valid")
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
        .expect("static Qwen tree limits are valid")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qwen_codec_preserves_unknown_fields_and_all_faithful_transports() {
        for value in [
            serde_json::json!({"type":"stdio","command":"node","args":["server.mjs"],"env":{"TOKEN":"${MCP_TOKEN}"},"future":{"keep":true}}),
            serde_json::json!({"type":"http","url":"https://example.invalid/mcp","headers":{"Authorization":"$MCP_AUTH"}}),
            serde_json::json!({"type":"sse","url":"https://example.invalid/events"}),
        ] {
            assert_eq!(QwenMcpCodec::encode_server(&value).unwrap(), value);
        }
    }

    #[test]
    fn qwen_codec_rejects_ambiguous_source_relative_literal_and_oauth_shapes() {
        for value in [
            serde_json::json!({"command":"node","url":"https://example.invalid"}),
            serde_json::json!({"command":"./server"}),
            serde_json::json!({"command":"node","env":{"TOKEN":"literal"}}),
            serde_json::json!({"url":"https://example.invalid","oauth":{}}),
            serde_json::json!({"url":"https://example.invalid","type":"websocket"}),
        ] {
            assert!(
                QwenMcpCodec::encode_server(&value).is_err(),
                "accepted {value}"
            );
        }
    }

    #[test]
    fn qwen_settings_codec_reads_only_mcp_servers() {
        let settings = parse_json_object(
            br#"{"future":{"enabled":true},"mcpServers":{"unmanaged":{"command":"keep"}}}"#,
            JsonLimits::new(4096, 16).unwrap(),
        )
        .unwrap();
        assert_eq!(settings["future"]["enabled"], true);
        assert_eq!(settings[MCP_CONTAINER]["unmanaged"]["command"], "keep");
    }
}
