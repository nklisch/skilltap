//! Validation and fingerprinting for complete standalone skill directories.

use std::{collections::BTreeMap, fmt};

use sha2::{Digest, Sha256};

use crate::{
    domain::{Fingerprint, FingerprintAlgorithm, RelativeArtifactPath},
    runtime::{ExternalTreeEntryKind, ExternalTreeSnapshot},
    storage::{ArtifactTree, ArtifactTreeError},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SkillTreeError {
    MissingSkillFile,
    SkillFileNotRegular,
    SymlinkNotAllowed { path: RelativeArtifactPath },
    Artifact(ArtifactTreeError),
    Fingerprint,
}

impl fmt::Display for SkillTreeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSkillFile => {
                formatter.write_str("skill directory must contain top-level SKILL.md")
            }
            Self::SkillFileNotRegular => {
                formatter.write_str("top-level SKILL.md must be a regular file")
            }
            Self::SymlinkNotAllowed { path } => {
                write!(formatter, "skill tree cannot contain symlink `{path}`")
            }
            Self::Artifact(error) => error.fmt(formatter),
            Self::Fingerprint => formatter.write_str("skill tree fingerprint could not be encoded"),
        }
    }
}

impl std::error::Error for SkillTreeError {}

impl From<ArtifactTreeError> for SkillTreeError {
    fn from(error: ArtifactTreeError) -> Self {
        Self::Artifact(error)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedSkillTree {
    tree: ArtifactTree,
    fingerprint: Fingerprint,
}

impl ValidatedSkillTree {
    pub fn validate(snapshot: &ExternalTreeSnapshot) -> Result<Self, SkillTreeError> {
        let mut files = BTreeMap::new();
        let mut skill_file = false;
        let mut has_regular_skill_file = false;
        for entry in snapshot.entries() {
            match entry.kind() {
                ExternalTreeEntryKind::Directory => {}
                ExternalTreeEntryKind::File => {
                    if entry.path().as_str() == "SKILL.md" {
                        skill_file = true;
                        has_regular_skill_file = true;
                    }
                    files.insert(
                        entry.path().as_str().to_owned(),
                        entry.file_bytes().unwrap_or_default().to_vec(),
                    );
                }
                ExternalTreeEntryKind::Symlink => {
                    return Err(SkillTreeError::SymlinkNotAllowed {
                        path: entry.path().clone(),
                    });
                }
            }
        }
        if !skill_file {
            return Err(SkillTreeError::MissingSkillFile);
        }
        if !has_regular_skill_file {
            return Err(SkillTreeError::SkillFileNotRegular);
        }
        let tree = ArtifactTree::new(files)?;
        let fingerprint = fingerprint(&tree)?;
        Ok(Self { tree, fingerprint })
    }

    pub const fn tree(&self) -> &ArtifactTree {
        &self.tree
    }

    pub const fn fingerprint(&self) -> &Fingerprint {
        &self.fingerprint
    }
}

fn fingerprint(tree: &ArtifactTree) -> Result<Fingerprint, SkillTreeError> {
    let mut digest = Sha256::new();
    for (path, bytes) in tree.files() {
        let path = path.as_str().as_bytes();
        let path_len = u64::try_from(path.len()).map_err(|_| SkillTreeError::Fingerprint)?;
        let byte_len = u64::try_from(bytes.len()).map_err(|_| SkillTreeError::Fingerprint)?;
        digest.update(path_len.to_be_bytes());
        digest.update(path);
        digest.update(byte_len.to_be_bytes());
        digest.update(bytes);
    }
    let hex = digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Fingerprint::new(FingerprintAlgorithm::Sha256, hex).map_err(|_| SkillTreeError::Fingerprint)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{ExternalTreeEntry, ExternalTreeLimits};

    fn limits() -> ExternalTreeLimits {
        ExternalTreeLimits::new(8, 32, 1024, 4096, 1024).unwrap()
    }

    #[test]
    fn requires_top_level_skill_file_and_keeps_siblings() {
        let snapshot = ExternalTreeSnapshot::new(
            [
                ExternalTreeEntry::directory(RelativeArtifactPath::new("docs").unwrap()),
                ExternalTreeEntry::file(
                    RelativeArtifactPath::new("SKILL.md").unwrap(),
                    b"---\nname: demo\n---\n".to_vec(),
                ),
                ExternalTreeEntry::file(
                    RelativeArtifactPath::new("docs/example.txt").unwrap(),
                    b"sibling".to_vec(),
                ),
            ],
            limits(),
        )
        .unwrap();
        let skill = ValidatedSkillTree::validate(&snapshot).unwrap();
        assert!(
            skill
                .tree()
                .files()
                .contains_key(&RelativeArtifactPath::new("docs/example.txt").unwrap())
        );
        assert_eq!(
            skill.fingerprint().algorithm(),
            FingerprintAlgorithm::Sha256
        );
    }

    #[test]
    fn rejects_missing_skill_file_and_symlink() {
        let missing = ExternalTreeSnapshot::new(
            [ExternalTreeEntry::file(
                RelativeArtifactPath::new("nested/SKILL.md").unwrap(),
                b"content".to_vec(),
            )],
            limits(),
        )
        .unwrap();
        assert_eq!(
            ValidatedSkillTree::validate(&missing),
            Err(SkillTreeError::MissingSkillFile)
        );
        let symlink = ExternalTreeSnapshot::new(
            [ExternalTreeEntry::symlink(
                RelativeArtifactPath::new("SKILL.md").unwrap(),
                b"elsewhere".to_vec(),
            )],
            limits(),
        )
        .unwrap();
        assert!(matches!(
            ValidatedSkillTree::validate(&symlink),
            Err(SkillTreeError::SymlinkNotAllowed { .. })
        ));
    }
}
