//! Pure project-skill layout and projection contracts.

use std::{
    fmt,
    path::{Component, Path, PathBuf},
};

use crate::{
    domain::{AbsolutePath, RelativeArtifactPath},
    instructions::{InstructionPathError, relative_symlink_target},
    runtime::RelativeSymlinkTarget,
    skill_compatibility::AgentSkillName,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProjectSkillPathError {
    InvalidProjectRoot,
    NativeRootOutsideProject,
    NativeRootHasInvalidComponent,
    DestinationInvalid,
    LinkTargetInvalid,
}

impl fmt::Display for ProjectSkillPathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidProjectRoot => "project skill layout requires an absolute project root",
            Self::NativeRootOutsideProject => "native project skill root is outside the project",
            Self::NativeRootHasInvalidComponent => {
                "native project skill root contains an invalid path component"
            }
            Self::DestinationInvalid => "native project skill destination is invalid",
            Self::LinkTargetInvalid => "project skill relative link target is invalid",
        })
    }
}

impl std::error::Error for ProjectSkillPathError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TargetProjectSkillProjection {
    Canonical { path: AbsolutePath },
    RelativeLink(ProjectSkillLinkSpec),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectSkillLinkSpec {
    pub project_root: AbsolutePath,
    pub destination: RelativeArtifactPath,
    pub canonical_path: AbsolutePath,
    pub target: RelativeSymlinkTarget,
}

impl ProjectSkillLinkSpec {
    pub fn destination_path(&self) -> AbsolutePath {
        AbsolutePath::new(format!(
            "{}/{}",
            self.project_root.as_str(),
            self.destination.as_str()
        ))
        .expect("validated project root and destination form a valid path")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectSkillLinkHealth {
    NotRequired,
    Healthy,
    Missing,
    Broken,
    Divergent,
    UnmanagedConflict,
}

/// Derive the project representation for one target from the adapter-provided
/// native root. Target identity is intentionally absent from this function:
/// path equality, not a harness-id branch, decides whether a link is needed.
pub fn project_skill_projection(
    project: &AbsolutePath,
    native_skill_root: &AbsolutePath,
    name: &AgentSkillName,
) -> Result<TargetProjectSkillProjection, ProjectSkillPathError> {
    let project_path = Path::new(project.as_str());
    if !project_path.is_absolute() {
        return Err(ProjectSkillPathError::InvalidProjectRoot);
    }
    let root_relative = native_skill_root
        .as_str()
        .strip_prefix(project.as_str())
        .and_then(|value| value.strip_prefix('/'))
        .ok_or(ProjectSkillPathError::NativeRootOutsideProject)?;
    if root_relative.is_empty() {
        // A project itself is a valid lexical root; the skill name still keeps
        // the destination a direct descendant and confined by the executor.
    }
    if Path::new(root_relative)
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(ProjectSkillPathError::NativeRootHasInvalidComponent);
    }

    let canonical_root = child(project, ".agents/skills")?;
    let canonical_path = child(&canonical_root, name.as_str())?;
    let native_path = child(native_skill_root, name.as_str())?;
    if native_path == canonical_path {
        return Ok(TargetProjectSkillProjection::Canonical {
            path: canonical_path,
        });
    }

    let destination_string = if root_relative.is_empty() {
        name.as_str().to_owned()
    } else {
        format!("{root_relative}/{}", name.as_str())
    };
    let destination = RelativeArtifactPath::new(destination_string)
        .map_err(|_| ProjectSkillPathError::DestinationInvalid)?;
    let target = relative_symlink_target(&native_path, &canonical_path)
        .map_err(|_| ProjectSkillPathError::LinkTargetInvalid)?;
    Ok(TargetProjectSkillProjection::RelativeLink(
        ProjectSkillLinkSpec {
            project_root: project.clone(),
            destination,
            canonical_path,
            target,
        },
    ))
}

fn child(parent: &AbsolutePath, child: &str) -> Result<AbsolutePath, ProjectSkillPathError> {
    let path = PathBuf::from(parent.as_str()).join(child);
    AbsolutePath::new(path.to_string_lossy().into_owned())
        .map_err(|_| ProjectSkillPathError::DestinationInvalid)
}

impl From<InstructionPathError> for ProjectSkillPathError {
    fn from(_: InstructionPathError) -> Self {
        Self::LinkTargetInvalid
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(value: &str) -> AbsolutePath {
        AbsolutePath::new(value).unwrap()
    }

    #[test]
    fn canonical_root_needs_no_projection() {
        let result = project_skill_projection(
            &path("/work/project"),
            &path("/work/project/.agents/skills"),
            &AgentSkillName::new("demo").unwrap(),
        )
        .unwrap();
        assert_eq!(
            result,
            TargetProjectSkillProjection::Canonical {
                path: path("/work/project/.agents/skills/demo")
            }
        );
    }

    #[test]
    fn native_descendant_gets_normalized_relative_link_target() {
        let result = project_skill_projection(
            &path("/work/project"),
            &path("/work/project/.claude/skills"),
            &AgentSkillName::new("demo").unwrap(),
        )
        .unwrap();
        let TargetProjectSkillProjection::RelativeLink(spec) = result else {
            panic!("Claude-style root must be projected");
        };
        assert_eq!(spec.destination.as_str(), ".claude/skills/demo");
        assert_eq!(
            spec.target.as_path(),
            Path::new("../../.agents/skills/demo")
        );
        assert_eq!(
            spec.destination_path(),
            path("/work/project/.claude/skills/demo")
        );
    }

    #[test]
    fn arbitrary_nested_roots_and_nested_projects_are_lexical() {
        let result = project_skill_projection(
            &path("/tmp/repos/app"),
            &path("/tmp/repos/app/.future/skills"),
            &AgentSkillName::new("demo").unwrap(),
        )
        .unwrap();
        let TargetProjectSkillProjection::RelativeLink(spec) = result else {
            panic!("future native root must be projected");
        };
        assert_eq!(
            spec.target.as_path(),
            Path::new("../../.agents/skills/demo")
        );
        assert_eq!(spec.destination.as_str(), ".future/skills/demo");
    }

    #[test]
    fn roots_outside_project_are_rejected() {
        let result = project_skill_projection(
            &path("/work/project"),
            &path("/work/project-other/.claude/skills"),
            &AgentSkillName::new("demo").unwrap(),
        );
        assert_eq!(result, Err(ProjectSkillPathError::NativeRootOutsideProject));
    }
}
