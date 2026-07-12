//! Validation and fingerprinting for complete standalone skill directories.

use std::{collections::BTreeMap, fmt};

use sha2::{Digest, Sha256};

use crate::{
    domain::{ArtifactFile, Fingerprint, FingerprintAlgorithm, NativeId, RelativeArtifactPath},
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
                        ArtifactFile::new(
                            entry.file_bytes().unwrap_or_default().to_vec(),
                            entry.file_executable().unwrap_or(false),
                        ),
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

    /// Return the declared frontmatter name when it is a valid native
    /// identifier. Callers use this only as an assertion; the directory name
    /// remains the resource identity and the source is never rewritten.
    pub fn declared_name(&self) -> Option<NativeId> {
        let bytes = self
            .tree
            .files()
            .get(&RelativeArtifactPath::new("SKILL.md").expect("static skill path is valid"))?;
        let text = std::str::from_utf8(bytes.contents()).ok()?;
        let mut lines = text.lines();
        if lines.next() != Some("---") {
            return None;
        }
        for line in lines {
            if line == "---" {
                return None;
            }
            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            if key.trim() == "name" {
                return NativeId::new(value.trim()).ok();
            }
        }
        None
    }
}

fn fingerprint(tree: &ArtifactTree) -> Result<Fingerprint, SkillTreeError> {
    let mut digest = Sha256::new();
    for (path, file) in tree.files() {
        let path = path.as_str().as_bytes();
        let path_len = u64::try_from(path.len()).map_err(|_| SkillTreeError::Fingerprint)?;
        let byte_len =
            u64::try_from(file.contents().len()).map_err(|_| SkillTreeError::Fingerprint)?;
        digest.update(path_len.to_be_bytes());
        digest.update(path);
        digest.update([u8::from(file.is_executable())]);
        digest.update(byte_len.to_be_bytes());
        digest.update(file.contents());
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
                    false,
                ),
                ExternalTreeEntry::file(
                    RelativeArtifactPath::new("docs/example.txt").unwrap(),
                    b"sibling".to_vec(),
                    false,
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
        assert_eq!(skill.declared_name().unwrap().as_str(), "demo");
    }

    #[test]
    fn rejects_missing_skill_file_and_symlink() {
        let missing = ExternalTreeSnapshot::new(
            [ExternalTreeEntry::file(
                RelativeArtifactPath::new("nested/SKILL.md").unwrap(),
                b"content".to_vec(),
                false,
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

    #[test]
    fn executable_intent_changes_fingerprint_without_changing_contents() {
        let make = |executable| {
            ExternalTreeSnapshot::new(
                [ExternalTreeEntry::file(
                    RelativeArtifactPath::new("SKILL.md").unwrap(),
                    b"---\nname: demo\n---\n".to_vec(),
                    executable,
                )],
                limits(),
            )
            .unwrap()
        };
        let plain = ValidatedSkillTree::validate(&make(false)).unwrap();
        let executable = ValidatedSkillTree::validate(&make(true)).unwrap();

        assert_ne!(plain.fingerprint(), executable.fingerprint());
        assert_eq!(plain.tree().files().len(), executable.tree().files().len());
    }
}
