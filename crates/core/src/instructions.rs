//! Pure instruction bridge health model.

use sha2::{Digest, Sha256};

use crate::domain::{AbsolutePath, Fingerprint, FingerprintAlgorithm};

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
}
