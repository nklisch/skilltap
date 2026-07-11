//! Pure publication batching for target materializations.

use crate::{
    domain::{Fingerprint, HarnessId, ResourceKey},
    storage::{ArtifactRole, ArtifactTree},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicationEntry {
    pub resource: ResourceKey,
    pub target: HarnessId,
    pub role: ArtifactRole,
    pub fingerprint: Fingerprint,
    pub tree: ArtifactTree,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicationBatch {
    entries: Vec<PublicationEntry>,
}

impl PublicationBatch {
    pub fn entries(&self) -> &[PublicationEntry] {
        &self.entries
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PublicationPlanError {
    Empty,
    BackupRole {
        resource: ResourceKey,
        target: HarnessId,
    },
    DuplicateTarget {
        resource: ResourceKey,
        target: HarnessId,
    },
}

impl std::fmt::Display for PublicationPlanError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => formatter.write_str("publication batch must contain an entry"),
            Self::BackupRole { resource, target } => write!(
                formatter,
                "publication entry `{resource}` for `{target}` cannot use the backup role"
            ),
            Self::DuplicateTarget { resource, target } => write!(
                formatter,
                "publication batch contains duplicate target `{target}` for `{resource}`"
            ),
        }
    }
}

impl std::error::Error for PublicationPlanError {}

/// Validate and deterministically order a set of target publication entries.
/// No filesystem, native harness, or state operation occurs here.
pub fn plan_publication(
    entries: impl IntoIterator<Item = PublicationEntry>,
) -> Result<PublicationBatch, PublicationPlanError> {
    let mut entries = entries.into_iter().collect::<Vec<_>>();
    if entries.is_empty() {
        return Err(PublicationPlanError::Empty);
    }
    entries.sort_by(|left, right| {
        left.resource
            .cmp(&right.resource)
            .then(left.target.cmp(&right.target))
    });
    let mut seen = std::collections::BTreeSet::new();
    for entry in &entries {
        if entry.role == ArtifactRole::Backup {
            return Err(PublicationPlanError::BackupRole {
                resource: entry.resource.clone(),
                target: entry.target.clone(),
            });
        }
        let key = (entry.resource.clone(), entry.target.clone());
        if !seen.insert(key) {
            return Err(PublicationPlanError::DuplicateTarget {
                resource: entry.resource.clone(),
                target: entry.target.clone(),
            });
        }
    }
    Ok(PublicationBatch { entries })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{FingerprintAlgorithm, ResourceId, Scope};

    fn resource(value: &str) -> ResourceKey {
        ResourceKey::new(ResourceId::new(value).unwrap(), Scope::Global)
    }

    fn fingerprint(value: char) -> Fingerprint {
        Fingerprint::new(FingerprintAlgorithm::Sha256, value.to_string().repeat(64)).unwrap()
    }

    fn entry(resource_name: &str, target: &str, value: char) -> PublicationEntry {
        PublicationEntry {
            resource: resource(resource_name),
            target: HarnessId::new(target).unwrap(),
            role: ArtifactRole::MaterializedPlugin,
            fingerprint: fingerprint(value),
            tree: ArtifactTree::new([("skills/demo/SKILL.md", b"---\nname: demo\n---\n".to_vec())])
                .unwrap(),
        }
    }

    #[test]
    fn publication_batch_is_sorted_by_resource_then_target() {
        let batch = plan_publication([
            entry("plugin:z", "codex", 'a'),
            entry("plugin:a", "claude", 'b'),
            entry("plugin:a", "codex", 'c'),
        ])
        .unwrap();
        let keys = batch
            .entries()
            .iter()
            .map(|entry| (entry.resource.clone(), entry.target.clone()))
            .collect::<Vec<_>>();
        assert_eq!(
            keys[0],
            (resource("plugin:a"), HarnessId::new("claude").unwrap())
        );
        assert_eq!(
            keys[1],
            (resource("plugin:a"), HarnessId::new("codex").unwrap())
        );
        assert_eq!(
            keys[2],
            (resource("plugin:z"), HarnessId::new("codex").unwrap())
        );
    }

    #[test]
    fn duplicate_targets_and_backup_roles_fail_before_publication() {
        let duplicate = plan_publication([
            entry("plugin:a", "codex", 'a'),
            entry("plugin:a", "codex", 'b'),
        ]);
        assert!(matches!(
            duplicate,
            Err(PublicationPlanError::DuplicateTarget { .. })
        ));

        let mut backup = entry("plugin:a", "codex", 'a');
        backup.role = ArtifactRole::Backup;
        assert!(matches!(
            plan_publication([backup]),
            Err(PublicationPlanError::BackupRole { .. })
        ));
    }

    #[test]
    fn empty_batches_are_rejected() {
        assert_eq!(plan_publication([]), Err(PublicationPlanError::Empty));
    }
}
