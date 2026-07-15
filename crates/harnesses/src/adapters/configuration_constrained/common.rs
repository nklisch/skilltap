use std::{collections::BTreeSet, io};

use skilltap_core::{
    domain::{
        AbsolutePath, ComponentKind, ComponentRequiredness, EvidenceCode, Fingerprint,
        RelativeArtifactPath,
    },
    instructions::fingerprint_contents,
    managed_projection::{ManagedPluginWrite, ManagedProjectionError},
    plugin_graph::ComponentDeclaration,
    runtime::{ConfinedFileSystem, DirectoryIdentity, ExternalTreeLimits, RuntimeError},
    skill::ValidatedSkillTree,
    skill_compatibility::{AgentSkillName, validate_agent_skill},
    storage::{ArtifactTree, ManagedProjection},
};

use super::super::file_managed::CompleteSourcePlugin;
use super::source::SelectedPortablePlugin;
use crate::managed_projection::{ManagedProjectionContext, ManagedProjectionInput};

pub(crate) type SkillProjectionPlan = (
    Vec<ManagedPluginWrite>,
    Vec<u8>,
    Vec<u8>,
    Vec<ManagedProjection>,
);
pub(crate) type ObservedTree = (DirectoryIdentity, ArtifactTree);

/// The skill planner only needs the complete source tree and the component
/// declarations. Keeping that as a named view lets file-managed and
/// configuration-constrained adapters share drift/fingerprint planning without
/// forcing their MCP codecs or source loading into the same abstraction.
pub(crate) trait SkillProjectionSource {
    fn tree(&self) -> &ArtifactTree;
    fn declarations(&self) -> &[ComponentDeclaration];
}

impl SkillProjectionSource for SelectedPortablePlugin {
    fn tree(&self) -> &ArtifactTree {
        &self.tree
    }

    fn declarations(&self) -> &[ComponentDeclaration] {
        &self.declarations
    }
}

impl SkillProjectionSource for CompleteSourcePlugin {
    fn tree(&self) -> &ArtifactTree {
        &self.tree
    }

    fn declarations(&self) -> &[ComponentDeclaration] {
        &self.declarations
    }
}

#[derive(Clone, Copy)]
pub(crate) struct SkillProjectionPolicy {
    diagnostics: SkillProjectionDiagnostics,
    validation: SkillTreeValidation,
}

impl SkillProjectionPolicy {
    pub(crate) const fn agent_skill_contract() -> Self {
        Self {
            diagnostics: SkillProjectionDiagnostics {
                missing_declared_name: "A plugin skill has no declared name.",
                required_missing_tree: "A required plugin skill is missing its complete directory.",
                unsafe_destination: SkillProjectionDestinationError::PluginMissing {
                    detail: "A plugin skill name is not a safe destination.",
                },
                incomplete_tree: "A plugin skill is not a complete artifact tree.",
                missing_top_level_skill: "A plugin skill is missing top-level SKILL.md.",
                invalid_agent_skill_name: Some(
                    "A plugin skill name is not a valid Agent Skill name.",
                ),
                invalid_contract: Some(
                    "A plugin skill does not meet the exact Agent Skills contract.",
                ),
                observed_tree_invalid: "The managed skill tree is invalid.",
                observe_safely_failed: "The managed skill tree could not be observed safely.",
                drifted: "An owned managed skill projection is missing or was replaced.",
            },
            validation: SkillTreeValidation::AgentSkillsContract,
        }
    }

    pub(crate) const fn complete_tree(diagnostics: SkillProjectionDiagnostics) -> Self {
        Self {
            diagnostics,
            validation: SkillTreeValidation::TopLevelSkillMdOnly,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct SkillProjectionDiagnostics {
    pub(crate) missing_declared_name: &'static str,
    pub(crate) required_missing_tree: &'static str,
    pub(crate) unsafe_destination: SkillProjectionDestinationError,
    pub(crate) incomplete_tree: &'static str,
    pub(crate) missing_top_level_skill: &'static str,
    pub(crate) invalid_agent_skill_name: Option<&'static str>,
    pub(crate) invalid_contract: Option<&'static str>,
    pub(crate) observed_tree_invalid: &'static str,
    pub(crate) observe_safely_failed: &'static str,
    pub(crate) drifted: &'static str,
}

#[derive(Clone, Copy)]
pub(crate) enum SkillProjectionDestinationError {
    PluginMissing {
        detail: &'static str,
    },
    Other {
        code: &'static str,
        summary: &'static str,
    },
}

impl SkillProjectionDestinationError {
    fn into_error(self) -> ManagedProjectionError {
        match self {
            Self::PluginMissing { detail } => ManagedProjectionError::PluginMissing { detail },
            Self::Other { code, summary } => ManagedProjectionError::Other { code, summary },
        }
    }
}

#[derive(Clone, Copy)]
enum SkillTreeValidation {
    /// Exact Agent Skills validation used by configuration-constrained targets.
    AgentSkillsContract,
    /// Legacy file-managed plugin semantics: a complete skill directory is
    /// required, but frontmatter conformance remains the source reader's job.
    TopLevelSkillMdOnly,
}

pub(crate) fn plan_skills<P: SkillProjectionSource + ?Sized>(
    skill_root: &AbsolutePath,
    context: &ManagedProjectionContext<'_>,
    plugin: Option<&P>,
) -> Result<SkillProjectionPlan, ManagedProjectionError> {
    plan_skills_with_policy(
        skill_root,
        context,
        plugin,
        SkillProjectionPolicy::agent_skill_contract(),
    )
}

pub(crate) fn plan_skills_with_policy<P: SkillProjectionSource + ?Sized>(
    skill_root: &AbsolutePath,
    context: &ManagedProjectionContext<'_>,
    plugin: Option<&P>,
    policy: SkillProjectionPolicy,
) -> Result<SkillProjectionPlan, ManagedProjectionError> {
    let removal = matches!(context.input, ManagedProjectionInput::Remove);
    let declarations = plugin.map_or(&[][..], SkillProjectionSource::declarations);
    let mut names = BTreeSet::new();
    let mut manifest = Vec::new();
    for declaration in declarations {
        match declaration.kind {
            ComponentKind::Skill => {
                names.insert(declaration.declared_name.clone().ok_or(
                    ManagedProjectionError::PluginMissing {
                        detail: policy.diagnostics.missing_declared_name,
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
            .map(|plugin| skill_tree(plugin.tree(), &name, policy))
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
                detail: policy.diagnostics.required_missing_tree,
            });
        }
        let destination = RelativeArtifactPath::new(&name)
            .map_err(|_| policy.diagnostics.unsafe_destination.into_error())?;
        let current = observe_tree(context.filesystem, skill_root, &destination, policy)?;
        verify_prior_skill(context.prior, &destination, current.as_ref(), policy)?;
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
    policy: SkillProjectionPolicy,
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
        detail: policy.diagnostics.incomplete_tree,
    })?;
    match policy.validation {
        SkillTreeValidation::AgentSkillsContract => validate_agent_skill_tree(&tree, name, policy)?,
        SkillTreeValidation::TopLevelSkillMdOnly => ensure_top_level_skill_md(&tree, policy)?,
    }
    Ok(Some(tree))
}

fn validate_agent_skill_tree(
    tree: &ArtifactTree,
    name: &str,
    policy: SkillProjectionPolicy,
) -> Result<(), ManagedProjectionError> {
    let validated = ValidatedSkillTree::from_artifact_tree(tree.clone()).map_err(|_| {
        ManagedProjectionError::PluginMissing {
            detail: policy.diagnostics.missing_top_level_skill,
        }
    })?;
    let name = AgentSkillName::new(name.to_owned()).map_err(|_| {
        ManagedProjectionError::PluginMissing {
            detail: policy
                .diagnostics
                .invalid_agent_skill_name
                .expect("Agent Skills validation supplies an invalid-name diagnostic"),
        }
    })?;
    let validation = validate_agent_skill(&validated, &name);
    if !validation.loadable_shape() || !validation.is_conforming() {
        return Err(ManagedProjectionError::PluginMissing {
            detail: policy
                .diagnostics
                .invalid_contract
                .expect("Agent Skills validation supplies a contract diagnostic"),
        });
    }
    Ok(())
}

fn ensure_top_level_skill_md(
    tree: &ArtifactTree,
    policy: SkillProjectionPolicy,
) -> Result<(), ManagedProjectionError> {
    if tree
        .files()
        .contains_key(&RelativeArtifactPath::new("SKILL.md").expect("static path is valid"))
    {
        Ok(())
    } else {
        Err(ManagedProjectionError::PluginMissing {
            detail: policy.diagnostics.missing_top_level_skill,
        })
    }
}

fn observe_tree(
    filesystem: &dyn ConfinedFileSystem,
    root: &AbsolutePath,
    destination: &RelativeArtifactPath,
    policy: SkillProjectionPolicy,
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
                detail: policy.diagnostics.observed_tree_invalid,
            })?,
        ))),
        Err(RuntimeError::FileSystem { source, .. })
            if source.kind() == io::ErrorKind::NotFound =>
        {
            Ok(None)
        }
        Err(_) => Err(ManagedProjectionError::PluginUnreadable {
            detail: policy.diagnostics.observe_safely_failed,
        }),
    }
}

fn verify_prior_skill(
    prior: &[ManagedProjection],
    destination: &RelativeArtifactPath,
    current: Option<&ObservedTree>,
    policy: SkillProjectionPolicy,
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
            detail: policy.diagnostics.drifted,
        });
    }
    Ok(())
}

pub(crate) fn fingerprint_tree(
    destination: &RelativeArtifactPath,
    tree: &ArtifactTree,
) -> Fingerprint {
    let mut bytes = Vec::new();
    append_tree_fingerprint(&mut bytes, destination, tree);
    fingerprint_contents(&bytes)
}

pub(crate) fn append_tree_fingerprint(
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

pub(crate) fn evidence(code: &'static str) -> EvidenceCode {
    EvidenceCode::new(code).expect("static constrained evidence code is valid")
}

pub(crate) fn tree_limits() -> ExternalTreeLimits {
    ExternalTreeLimits::new(64, 100_000, 64 * 1024 * 1024, 1024 * 1024 * 1024, 64 * 1024)
        .expect("static constrained tree limits are valid")
}

pub(crate) fn read_optional_file(
    filesystem: &dyn ConfinedFileSystem,
    root: &AbsolutePath,
    destination: &RelativeArtifactPath,
    maximum_bytes: u64,
    detail: &'static str,
) -> Result<Option<Vec<u8>>, ManagedProjectionError> {
    match filesystem.read_regular_bounded_no_follow(root, destination, maximum_bytes) {
        Ok(bytes) => Ok(bytes),
        Err(RuntimeError::FileSystem { source, .. })
            if source.kind() == io::ErrorKind::NotFound =>
        {
            Ok(None)
        }
        Err(_) => Err(ManagedProjectionError::McpInvalid { detail }),
    }
}
