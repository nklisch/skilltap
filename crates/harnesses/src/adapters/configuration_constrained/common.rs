use std::{collections::BTreeSet, io};

use skilltap_core::{
    domain::{ComponentKind, ComponentRequiredness, RelativeArtifactPath},
    instructions::fingerprint_contents,
    managed_projection::{ManagedPluginWrite, ManagedProjectionError},
    runtime::{ConfinedFileSystem, DirectoryIdentity, ExternalTreeLimits, RuntimeError},
    skill::ValidatedSkillTree,
    skill_compatibility::{AgentSkillName, validate_agent_skill},
    storage::{ArtifactTree, ManagedProjection},
};

use super::source::SelectedPortablePlugin;
use crate::managed_projection::ManagedProjectionContext;

pub(crate) type SkillProjectionPlan = (
    Vec<ManagedPluginWrite>,
    Vec<u8>,
    Vec<u8>,
    Vec<ManagedProjection>,
);
pub(crate) type ObservedTree = (DirectoryIdentity, ArtifactTree);

pub(crate) fn plan_skills(
    skill_root: &skilltap_core::domain::AbsolutePath,
    context: &ManagedProjectionContext<'_>,
    plugin: Option<&SelectedPortablePlugin>,
    target_name: &'static str,
) -> Result<SkillProjectionPlan, ManagedProjectionError> {
    let removal = matches!(
        context.input,
        crate::managed_projection::ManagedProjectionInput::Remove
    );
    let declarations = plugin.map_or(&[][..], |plugin| plugin.declarations.as_slice());
    let mut names = BTreeSet::new();
    let mut manifest = Vec::new();
    for declaration in declarations {
        match declaration.kind {
            ComponentKind::Skill => {
                names.insert(declaration.declared_name.clone().ok_or(
                    ManagedProjectionError::PluginMissing {
                        detail: "A plugin skill has no declared name.",
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
            .map(|plugin| skill_tree(&plugin.tree, &name, target_name))
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
                detail: "A required plugin skill is missing its complete directory.",
            });
        }
        let destination = RelativeArtifactPath::new(&name).map_err(|_| {
            ManagedProjectionError::PluginMissing {
                detail: "A plugin skill name is not a safe destination.",
            }
        })?;
        let current = observe_tree(context.filesystem, skill_root, &destination, target_name)?;
        verify_prior_skill(context.prior, &destination, current.as_ref(), target_name)?;
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
    target_name: &str,
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
        detail: "A plugin skill is not a complete artifact tree.",
    })?;
    let validated = ValidatedSkillTree::from_artifact_tree(tree.clone()).map_err(|_| {
        ManagedProjectionError::PluginMissing {
            detail: "A plugin skill is missing top-level SKILL.md.",
        }
    })?;
    let name = AgentSkillName::new(name.to_owned()).map_err(|_| {
        ManagedProjectionError::PluginMissing {
            detail: "A plugin skill name is not a valid Agent Skill name.",
        }
    })?;
    let validation = validate_agent_skill(&validated, &name);
    if !validation.loadable_shape() || !validation.is_conforming() {
        let _ = target_name;
        return Err(ManagedProjectionError::PluginMissing {
            detail: "A plugin skill does not meet the exact Agent Skills contract.",
        });
    }
    Ok(Some(tree))
}

fn observe_tree(
    filesystem: &dyn ConfinedFileSystem,
    root: &skilltap_core::domain::AbsolutePath,
    destination: &RelativeArtifactPath,
    target_name: &str,
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
                detail: "The managed skill tree is invalid.",
            })?,
        ))),
        Err(RuntimeError::FileSystem { source, .. })
            if source.kind() == io::ErrorKind::NotFound =>
        {
            Ok(None)
        }
        Err(_) => {
            let _ = target_name;
            Err(ManagedProjectionError::PluginUnreadable {
                detail: "The managed skill tree could not be observed safely.",
            })
        }
    }
}

fn verify_prior_skill(
    prior: &[ManagedProjection],
    destination: &RelativeArtifactPath,
    current: Option<&ObservedTree>,
    target_name: &str,
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
        let _ = target_name;
        return Err(ManagedProjectionError::Drifted {
            detail: "An owned managed skill projection is missing or was replaced.",
        });
    }
    Ok(())
}

pub(crate) fn fingerprint_tree(
    destination: &RelativeArtifactPath,
    tree: &ArtifactTree,
) -> skilltap_core::domain::Fingerprint {
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

pub(crate) fn evidence(code: &'static str) -> skilltap_core::domain::EvidenceCode {
    skilltap_core::domain::EvidenceCode::new(code)
        .expect("static constrained evidence code is valid")
}

pub(crate) fn tree_limits() -> ExternalTreeLimits {
    ExternalTreeLimits::new(64, 100_000, 64 * 1024 * 1024, 1024 * 1024 * 1024, 64 * 1024)
        .expect("static constrained tree limits are valid")
}

pub(crate) fn read_optional_file(
    filesystem: &dyn ConfinedFileSystem,
    root: &skilltap_core::domain::AbsolutePath,
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
