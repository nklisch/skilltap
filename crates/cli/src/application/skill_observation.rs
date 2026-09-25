//! Read-only comparison shared by standalone skill planning and global status.
use super::*;
use skilltap_core::runtime::{ConfinedFileSystem, RuntimeError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SkillCondition {
    Satisfied,
    Missing,
    Drifted,
    Invalid,
    Unrecorded,
}

impl SkillCondition {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Satisfied => "satisfied",
            Self::Missing => "missing",
            Self::Drifted => "drifted",
            Self::Invalid => "invalid",
            Self::Unrecorded => "unrecorded",
        }
    }
}

pub(super) fn compare_skill(
    registry: &skilltap_harnesses::TargetRegistry,
    paths: &PlatformPaths,
    resource: &DesiredResource,
    state: Option<&ResourceState>,
    target: &HarnessId,
) -> SkillCondition {
    let expected = state
        .and_then(|state| state.target(target))
        .and_then(|binding| binding.fingerprint());
    let targets = HarnessSet::new([target.clone()]).expect("one skill target is valid");
    let limits =
        ExternalTreeLimits::new(64, 100_000, 64 * 1024 * 1024, 1024 * 1024 * 1024, 64 * 1024)
            .expect("bounded skill comparison limits are valid");
    if matches!(resource.scope(), Scope::Project(_)) {
        use super::project_skills::CanonicalProjectSkillObservation;
        use skilltap_core::project_skill::ProjectSkillLinkHealth;
        let Ok(observation) = super::project_skills::observe_project_skill(
            registry,
            &SystemFileSystem,
            paths,
            resource,
            state,
            &targets,
            limits,
        ) else {
            return SkillCondition::Invalid;
        };
        let fingerprint = match &observation.canonical {
            CanonicalProjectSkillObservation::Missing => return SkillCondition::Missing,
            CanonicalProjectSkillObservation::Invalid { .. } => return SkillCondition::Invalid,
            CanonicalProjectSkillObservation::Present { fingerprint, .. } => fingerprint,
        };
        if expected.is_some_and(|expected| expected != fingerprint) {
            return SkillCondition::Drifted;
        }
        return match observation
            .targets
            .get(target)
            .map(|target| target.projection)
        {
            Some(ProjectSkillLinkHealth::Healthy | ProjectSkillLinkHealth::NotRequired) => {
                if expected.is_some() {
                    SkillCondition::Satisfied
                } else {
                    SkillCondition::Unrecorded
                }
            }
            Some(ProjectSkillLinkHealth::Missing | ProjectSkillLinkHealth::Broken) => {
                SkillCondition::Missing
            }
            Some(ProjectSkillLinkHealth::Divergent | ProjectSkillLinkHealth::UnmanagedConflict)
            | None => SkillCondition::Invalid,
        };
    }
    let Some(destination) = resource
        .id()
        .as_str()
        .strip_prefix("skill:")
        .and_then(|name| NativeId::new(name).ok())
        .and_then(|name| skill_relative_destination(&name))
    else {
        return SkillCondition::Invalid;
    };
    let Some(destinations) =
        skill_destinations(registry, paths, resource.scope(), &targets, &destination)
    else {
        return SkillCondition::Invalid;
    };
    let mut missing = false;
    for entry in destinations {
        let files =
            match SystemFileSystem.load_tree_bounded_no_follow(&entry.root, &destination, limits) {
                Ok((_, files)) => files,
                Err(RuntimeError::FileSystem { source, .. })
                    if source.kind() == std::io::ErrorKind::NotFound =>
                {
                    missing = true;
                    continue;
                }
                Err(_) => return SkillCondition::Invalid,
            };
        let tree = ArtifactTree::new(
            files
                .into_iter()
                .map(|(path, file)| (path.as_str().to_owned(), file)),
        );
        let Some(skill) = tree
            .ok()
            .and_then(|tree| ValidatedSkillTree::from_artifact_tree(tree).ok())
        else {
            return SkillCondition::Invalid;
        };
        if expected.is_some_and(|expected| expected != skill.fingerprint()) {
            return SkillCondition::Drifted;
        }
    }
    if missing {
        SkillCondition::Missing
    } else if expected.is_none() {
        SkillCondition::Unrecorded
    } else {
        SkillCondition::Satisfied
    }
}
