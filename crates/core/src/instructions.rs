//! Pure instruction bridge specification and health model.

use std::path::{Component, Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::{
    domain::{AbsolutePath, Fingerprint, FingerprintAlgorithm},
    runtime::RelativeSymlinkTarget,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InstructionPathError {
    BridgeHasNoParent,
    InvalidRelativeTarget,
    AbsoluteObservedTarget,
    ObservedTargetEscapesRoot,
    InvalidEffectiveTarget,
}

/// Compute the normalized relative target from a bridge's parent to the
/// canonical instruction file. The path relationship, rather than the scope
/// label, is authoritative.
pub fn relative_symlink_target(
    bridge: &AbsolutePath,
    canonical: &AbsolutePath,
) -> Result<RelativeSymlinkTarget, InstructionPathError> {
    let bridge_parent = Path::new(bridge.as_str())
        .parent()
        .ok_or(InstructionPathError::BridgeHasNoParent)?;
    let parent_components = bridge_parent.components().collect::<Vec<_>>();
    let canonical_components = Path::new(canonical.as_str())
        .components()
        .collect::<Vec<_>>();
    let common = parent_components
        .iter()
        .zip(&canonical_components)
        .take_while(|(left, right)| left == right)
        .count();

    let mut relative = PathBuf::new();
    for _ in common..parent_components.len() {
        relative.push("..");
    }
    for component in &canonical_components[common..] {
        relative.push(component.as_os_str());
    }
    let relative = relative
        .to_str()
        .ok_or(InstructionPathError::InvalidRelativeTarget)?;
    RelativeSymlinkTarget::new(relative.to_owned())
        .map_err(|_| InstructionPathError::InvalidRelativeTarget)
}

/// Resolve an observed relative link lexically without touching the
/// filesystem. Absolute targets and paths that escape the filesystem root are
/// rejected before they can be treated as managed evidence.
pub fn resolve_symlink_target(
    bridge: &AbsolutePath,
    observed: &Path,
) -> Result<AbsolutePath, InstructionPathError> {
    if observed.is_absolute() {
        return Err(InstructionPathError::AbsoluteObservedTarget);
    }
    let parent = Path::new(bridge.as_str())
        .parent()
        .ok_or(InstructionPathError::BridgeHasNoParent)?;
    let mut resolved = parent
        .components()
        .filter_map(|component| match component {
            Component::RootDir | Component::Normal(_) | Component::Prefix(_) => {
                Some(component.as_os_str().to_owned())
            }
            Component::CurDir | Component::ParentDir => None,
        })
        .collect::<Vec<_>>();

    for component in observed.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if resolved.len() <= 1 {
                    return Err(InstructionPathError::ObservedTargetEscapesRoot);
                }
                resolved.pop();
            }
            Component::Normal(value) => resolved.push(value.to_owned()),
            Component::RootDir | Component::Prefix(_) => {
                return Err(InstructionPathError::AbsoluteObservedTarget);
            }
        }
    }

    let mut effective = PathBuf::new();
    for component in resolved {
        effective.push(component);
    }
    AbsolutePath::new(effective.to_string_lossy().into_owned())
        .map_err(|_| InstructionPathError::InvalidEffectiveTarget)
}

/// Fingerprint instruction bytes using the same stable SHA-256 representation
/// as other managed text resources.
pub fn fingerprint_contents(contents: &[u8]) -> Fingerprint {
    let digest = Sha256::digest(contents);
    let hex = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Fingerprint::new(FingerprintAlgorithm::Sha256, hex)
        .expect("SHA-256 digest always has the required length")
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstructionPaths {
    pub canonical: AbsolutePath,
    pub codex_bridge: AbsolutePath,
    pub claude_bridge: AbsolutePath,
}

pub fn global_paths(
    home: &AbsolutePath,
    codex_home: &AbsolutePath,
    claude_home: &AbsolutePath,
) -> Option<InstructionPaths> {
    Some(InstructionPaths {
        canonical: child(home, "AGENTS.md")?,
        codex_bridge: child(codex_home, "AGENTS.md")?,
        claude_bridge: child(claude_home, "CLAUDE.md")?,
    })
}

pub fn project_paths(project: &AbsolutePath) -> Option<(AbsolutePath, AbsolutePath)> {
    Some((child(project, "AGENTS.md")?, child(project, "CLAUDE.md")?))
}

fn child(root: &AbsolutePath, name: &str) -> Option<AbsolutePath> {
    AbsolutePath::new(format!("{}/{}", root.as_str(), name)).ok()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InstructionBridgeMode {
    Symlink,
    Import,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InstructionBridgeRepresentation {
    Symlink(RelativeSymlinkTarget),
    Import(Vec<u8>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstructionBridgeSpec {
    pub canonical: AbsolutePath,
    pub bridge: AbsolutePath,
    pub mode: InstructionBridgeMode,
    pub representation: InstructionBridgeRepresentation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ObservedInstructionBridge {
    Missing,
    Symlink {
        effective_target: Option<AbsolutePath>,
        target_exists: bool,
        target_is_regular: bool,
    },
    RegularFile {
        fingerprint: Fingerprint,
    },
    Other,
}

pub fn classify_bridge(
    spec: &InstructionBridgeSpec,
    observed: &ObservedInstructionBridge,
) -> InstructionHealth {
    match (observed, &spec.representation) {
        (ObservedInstructionBridge::Missing, _) => InstructionHealth::Missing,
        (
            ObservedInstructionBridge::Symlink {
                effective_target: Some(effective),
                target_exists: true,
                target_is_regular: true,
            },
            InstructionBridgeRepresentation::Symlink(_),
        ) if effective == &spec.canonical => InstructionHealth::Managed,
        (
            ObservedInstructionBridge::Symlink {
                effective_target: None,
                ..
            },
            _,
        )
        | (
            ObservedInstructionBridge::Symlink {
                target_exists: false,
                ..
            },
            _,
        )
        | (
            ObservedInstructionBridge::Symlink {
                target_is_regular: false,
                ..
            },
            _,
        )
        | (ObservedInstructionBridge::Other, _) => InstructionHealth::Broken,
        (
            ObservedInstructionBridge::RegularFile { fingerprint },
            InstructionBridgeRepresentation::Import(contents),
        ) if fingerprint == &fingerprint_contents(contents) => InstructionHealth::Managed,
        (ObservedInstructionBridge::Symlink { .. }, _)
        | (ObservedInstructionBridge::RegularFile { .. }, _) => InstructionHealth::Divergent,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InstructionHealth {
    Missing,
    Managed,
    Divergent,
    Broken,
    Duplicate,
    Unmanaged,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InstructionRepairAction {
    Create,
    Repair,
    NoOp,
    BlockedConflict,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InstructionRepairPlan {
    pub action: InstructionRepairAction,
    pub backup_required: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstructionLocation {
    pub canonical: AbsolutePath,
    pub bridge: AbsolutePath,
    pub mode: InstructionBridgeMode,
    pub expected: Option<Fingerprint>,
    pub observed: Option<Fingerprint>,
    pub bridge_exists: bool,
}

pub fn classify(location: &InstructionLocation) -> InstructionHealth {
    if !location.bridge_exists {
        return InstructionHealth::Missing;
    }
    if location.expected.is_none() || location.observed.is_none() {
        return InstructionHealth::Unmanaged;
    }
    if location.expected != location.observed {
        return InstructionHealth::Divergent;
    }
    InstructionHealth::Managed
}

pub fn repair_plan(
    location: &InstructionLocation,
    acknowledge_divergence: bool,
) -> InstructionRepairPlan {
    match classify(location) {
        InstructionHealth::Missing => InstructionRepairPlan {
            action: InstructionRepairAction::Create,
            backup_required: false,
        },
        InstructionHealth::Managed => InstructionRepairPlan {
            action: InstructionRepairAction::NoOp,
            backup_required: false,
        },
        InstructionHealth::Divergent | InstructionHealth::Duplicate => {
            if acknowledge_divergence {
                InstructionRepairPlan {
                    action: InstructionRepairAction::Repair,
                    backup_required: true,
                }
            } else {
                InstructionRepairPlan {
                    action: InstructionRepairAction::BlockedConflict,
                    backup_required: false,
                }
            }
        }
        InstructionHealth::Broken | InstructionHealth::Unmanaged => InstructionRepairPlan {
            action: InstructionRepairAction::BlockedConflict,
            backup_required: false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Fingerprint, FingerprintAlgorithm};

    fn location(
        expected: Option<char>,
        observed: Option<char>,
        exists: bool,
    ) -> InstructionLocation {
        let fingerprint = |value: char| {
            Fingerprint::new(FingerprintAlgorithm::Sha256, value.to_string().repeat(64)).unwrap()
        };
        InstructionLocation {
            canonical: AbsolutePath::new("/home/user/AGENTS.md").unwrap(),
            bridge: AbsolutePath::new("/home/user/CLAUDE.md").unwrap(),
            mode: InstructionBridgeMode::Symlink,
            expected: expected.map(fingerprint),
            observed: observed.map(fingerprint),
            bridge_exists: exists,
        }
    }

    #[test]
    fn canonical_and_bridge_health_is_explicit() {
        assert_eq!(
            classify(&location(None, None, false)),
            InstructionHealth::Missing
        );
        assert_eq!(
            classify(&location(Some('a'), Some('b'), true)),
            InstructionHealth::Divergent
        );
        assert_eq!(
            classify(&location(Some('a'), Some('a'), true)),
            InstructionHealth::Managed
        );
        assert_eq!(
            repair_plan(&location(Some('a'), Some('b'), true), false).action,
            InstructionRepairAction::BlockedConflict
        );
        assert!(repair_plan(&location(Some('a'), Some('b'), true), true).backup_required);
    }

    #[test]
    fn global_and_project_paths_keep_canonical_home_separate_from_codex_home() {
        let paths = global_paths(
            &AbsolutePath::new("/home/user").unwrap(),
            &AbsolutePath::new("/home/user/.codex").unwrap(),
            &AbsolutePath::new("/home/user/.claude").unwrap(),
        )
        .unwrap();
        assert_eq!(paths.canonical.as_str(), "/home/user/AGENTS.md");
        assert_eq!(paths.codex_bridge.as_str(), "/home/user/.codex/AGENTS.md");
        assert_eq!(
            project_paths(&AbsolutePath::new("/tmp/project").unwrap())
                .unwrap()
                .0
                .as_str(),
            "/tmp/project/AGENTS.md"
        );
    }

    #[test]
    fn relative_bridge_targets_follow_actual_path_relationships() {
        for (bridge, canonical, expected) in [
            (
                "/home/user/.codex/AGENTS.md",
                "/home/user/AGENTS.md",
                "../AGENTS.md",
            ),
            (
                "/opt/codex/AGENTS.md",
                "/home/user/AGENTS.md",
                "../../home/user/AGENTS.md",
            ),
            (
                "/home/user/deep/codex/AGENTS.md",
                "/home/user/AGENTS.md",
                "../../AGENTS.md",
            ),
            (
                "/work/project/CLAUDE.md",
                "/work/project/AGENTS.md",
                "AGENTS.md",
            ),
        ] {
            assert_eq!(
                relative_symlink_target(
                    &AbsolutePath::new(bridge).unwrap(),
                    &AbsolutePath::new(canonical).unwrap(),
                )
                .unwrap()
                .as_path(),
                Path::new(expected),
            );
        }
    }

    #[test]
    fn observed_targets_are_resolved_lexically_and_fail_closed() {
        let bridge = AbsolutePath::new("/home/user/.codex/AGENTS.md").unwrap();
        assert_eq!(
            resolve_symlink_target(&bridge, Path::new("./../AGENTS.md"))
                .unwrap()
                .as_str(),
            "/home/user/AGENTS.md"
        );
        assert_eq!(
            resolve_symlink_target(&bridge, Path::new("../../other/AGENTS.md"))
                .unwrap()
                .as_str(),
            "/home/other/AGENTS.md"
        );
        assert!(resolve_symlink_target(&bridge, Path::new("/home/user/AGENTS.md")).is_err());
        assert!(resolve_symlink_target(&bridge, Path::new("../../../../escape")).is_err());
    }

    #[test]
    fn bridge_health_requires_exact_live_regular_canonical_target() {
        let canonical = AbsolutePath::new("/home/user/AGENTS.md").unwrap();
        let bridge = AbsolutePath::new("/home/user/.codex/AGENTS.md").unwrap();
        let spec = InstructionBridgeSpec {
            canonical: canonical.clone(),
            bridge,
            mode: InstructionBridgeMode::Symlink,
            representation: InstructionBridgeRepresentation::Symlink(
                RelativeSymlinkTarget::new("../AGENTS.md").unwrap(),
            ),
        };
        let observed = |effective_target, target_exists, target_is_regular| {
            ObservedInstructionBridge::Symlink {
                effective_target,
                target_exists,
                target_is_regular,
            }
        };
        assert_eq!(
            classify_bridge(&spec, &observed(Some(canonical.clone()), true, true)),
            InstructionHealth::Managed
        );
        assert_eq!(
            classify_bridge(&spec, &observed(Some(canonical), false, false)),
            InstructionHealth::Broken
        );
        assert_eq!(
            classify_bridge(
                &spec,
                &observed(
                    Some(AbsolutePath::new("/home/other/AGENTS.md").unwrap()),
                    true,
                    true,
                )
            ),
            InstructionHealth::Divergent
        );
    }
}
