//! Pure publication batching for target materializations.

use crate::{
    domain::{Fingerprint, HarnessId, ResourceKey},
    storage::{
        ArtifactPublication, ArtifactRole, ArtifactTree, ManagedArtifactError,
        ManagedArtifactRecord, ManagedArtifactRepository,
    },
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublishedArtifact {
    resource: ResourceKey,
    target: HarnessId,
    record: ManagedArtifactRecord,
    reused: bool,
}

impl PublishedArtifact {
    fn new(
        resource: ResourceKey,
        target: HarnessId,
        record: ManagedArtifactRecord,
        reused: bool,
    ) -> Self {
        Self {
            resource,
            target,
            record,
            reused,
        }
    }

    pub fn resource(&self) -> &ResourceKey {
        &self.resource
    }

    pub fn target(&self) -> &HarnessId {
        &self.target
    }

    pub fn record(&self) -> &ManagedArtifactRecord {
        &self.record
    }

    pub const fn reused(&self) -> bool {
        self.reused
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicationReceipt {
    published: Vec<PublishedArtifact>,
}

impl PublicationReceipt {
    pub fn published(&self) -> &[PublishedArtifact] {
        &self.published
    }
}

pub trait PublicationSink {
    type Error;

    fn publish(&self, entry: &PublicationEntry) -> Result<PublishedArtifact, Self::Error>;
}

#[derive(Debug)]
pub enum PublicationApplyError<E> {
    Failed {
        resource: ResourceKey,
        target: HarnessId,
        error: E,
    },
    Partial {
        published: Vec<PublishedArtifact>,
        resource: ResourceKey,
        target: HarnessId,
        error: E,
    },
}

impl<E: std::fmt::Display> std::fmt::Display for PublicationApplyError<E> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed {
                resource,
                target,
                error,
            } => write!(
                formatter,
                "publication of `{resource}` for `{target}` failed: {error}"
            ),
            Self::Partial {
                published,
                resource,
                target,
                error,
            } => write!(
                formatter,
                "publication partially completed ({} entries) before `{resource}` for `{target}` failed: {error}",
                published.len()
            ),
        }
    }
}

impl<E: std::fmt::Debug + std::fmt::Display> std::error::Error for PublicationApplyError<E> {}

/// Apply a validated batch in deterministic order. The caller owns the
/// configuration lock and state publication boundary; this function only
/// invokes the supplied sink and retains exact completed-entry context.
pub fn apply_publication<S: PublicationSink>(
    batch: &PublicationBatch,
    sink: &S,
) -> Result<PublicationReceipt, PublicationApplyError<S::Error>> {
    let mut published = Vec::with_capacity(batch.entries.len());
    for entry in &batch.entries {
        match sink.publish(entry) {
            Ok(artifact) => published.push(artifact),
            Err(error) if published.is_empty() => {
                return Err(PublicationApplyError::Failed {
                    resource: entry.resource.clone(),
                    target: entry.target.clone(),
                    error,
                });
            }
            Err(error) => {
                return Err(PublicationApplyError::Partial {
                    published,
                    resource: entry.resource.clone(),
                    target: entry.target.clone(),
                    error,
                });
            }
        }
    }
    Ok(PublicationReceipt { published })
}

/// Adapter that stores each complete tree through the existing managed
/// artifact repository. Target-specific projection and native registration
/// remain separate operations owned by the harness/application layer.
pub struct ManagedPublicationSink<'a, R: ManagedArtifactRepository + ?Sized> {
    repository: &'a R,
}

impl<'a, R: ManagedArtifactRepository + ?Sized> ManagedPublicationSink<'a, R> {
    pub const fn new(repository: &'a R) -> Self {
        Self { repository }
    }
}

impl<R: ManagedArtifactRepository + ?Sized> PublicationSink for ManagedPublicationSink<'_, R> {
    type Error = ManagedArtifactError;

    fn publish(&self, entry: &PublicationEntry) -> Result<PublishedArtifact, Self::Error> {
        let publication = self.repository.publish(
            &entry.resource,
            entry.role,
            &entry.fingerprint,
            &entry.tree,
        )?;
        let (handle, reused) = match publication {
            ArtifactPublication::Published(handle) => (handle, false),
            ArtifactPublication::Existing(handle) => (handle, true),
        };
        Ok(PublishedArtifact::new(
            entry.resource.clone(),
            entry.target.clone(),
            handle.record().clone(),
            reused,
        ))
    }
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
    use std::cell::RefCell;

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

    struct FakeSink {
        fail_target: Option<String>,
        calls: RefCell<Vec<(ResourceKey, HarnessId)>>,
    }

    impl PublicationSink for FakeSink {
        type Error = &'static str;

        fn publish(&self, entry: &PublicationEntry) -> Result<PublishedArtifact, Self::Error> {
            self.calls
                .borrow_mut()
                .push((entry.resource.clone(), entry.target.clone()));
            if self
                .fail_target
                .as_deref()
                .is_some_and(|target| target == entry.target.as_str())
            {
                return Err("forced publication failure");
            }
            let record = ManagedArtifactRecord::for_artifact(
                entry.resource.clone(),
                entry.role,
                entry.fingerprint.clone(),
            )
            .unwrap();
            Ok(PublishedArtifact::new(
                entry.resource.clone(),
                entry.target.clone(),
                record,
                false,
            ))
        }
    }

    #[test]
    fn publication_failure_keeps_exact_completed_entries_and_target() {
        let batch = plan_publication([
            entry("plugin:a", "codex", 'a'),
            entry("plugin:a", "claude", 'b'),
        ])
        .unwrap();
        let sink = FakeSink {
            fail_target: Some("claude".to_owned()),
            calls: RefCell::new(Vec::new()),
        };
        let error = apply_publication(&batch, &sink).unwrap_err();
        assert!(matches!(
            error,
            PublicationApplyError::Failed { ref target, .. } if target.as_str() == "claude"
        ));

        let sink = FakeSink {
            fail_target: Some("codex".to_owned()),
            calls: RefCell::new(Vec::new()),
        };
        let batch = plan_publication([
            entry("plugin:a", "claude", 'a'),
            entry("plugin:a", "codex", 'b'),
        ])
        .unwrap();
        let error = apply_publication(&batch, &sink).unwrap_err();
        assert!(matches!(
            error,
            PublicationApplyError::Partial {
                ref published,
                ref target,
                ..
            } if published.len() == 1 && target.as_str() == "codex"
        ));
        assert_eq!(sink.calls.borrow().len(), 2);
    }
}
