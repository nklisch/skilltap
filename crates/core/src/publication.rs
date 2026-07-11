//! Pure publication batching for target materializations.

use std::collections::BTreeMap;

use crate::{
    domain::{
        Fingerprint, HarnessId, NativeId, ObservationLayer, ObservedResource, Ownership,
        Provenance, ResourceHealth, ResourceKey,
    },
    storage::{
        ArtifactPublication, ArtifactRole, ArtifactTree, ManagedArtifactError,
        ManagedArtifactRecord, ManagedArtifactRepository, ResourceState, SchemaError,
        StateDocument, Timestamp,
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
    pub fn new(
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
    verified: Vec<VerifiedTarget>,
}

impl PublicationReceipt {
    pub fn published(&self) -> &[PublishedArtifact] {
        &self.published
    }

    pub fn verified(&self) -> &[VerifiedTarget] {
        &self.verified
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedTarget {
    resource: ResourceKey,
    target: HarnessId,
    native_identity: NativeId,
}

impl VerifiedTarget {
    fn new(resource: ResourceKey, target: HarnessId, native_identity: NativeId) -> Self {
        Self {
            resource,
            target,
            native_identity,
        }
    }

    pub fn resource(&self) -> &ResourceKey {
        &self.resource
    }

    pub fn target(&self) -> &HarnessId {
        &self.target
    }

    pub fn native_identity(&self) -> &NativeId {
        &self.native_identity
    }
}

pub trait PublicationSink {
    type Error;

    fn publish(&self, entry: &PublicationEntry) -> Result<PublishedArtifact, Self::Error>;
}

pub trait LoadVerifier {
    fn verify_loaded(
        &self,
        entry: &PublicationEntry,
        artifact: &PublishedArtifact,
    ) -> Result<VerifiedTarget, LoadVerificationError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LoadVerificationError {
    ResourceMismatch,
    TargetMismatch,
    MissingEffectiveObservation,
    Unhealthy(ResourceHealth),
    MissingFingerprint,
    FingerprintMismatch,
}

impl std::fmt::Display for LoadVerificationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::ResourceMismatch => {
                "the observed resource identity does not match the publication"
            }
            Self::TargetMismatch => "the observed harness does not match the publication target",
            Self::MissingEffectiveObservation => "the target has no effective load observation",
            Self::Unhealthy(_) => "the target loaded the resource in an unhealthy state",
            Self::MissingFingerprint => "the target observation has no resource fingerprint",
            Self::FingerprintMismatch => "the target fingerprint does not match the publication",
        })
    }
}

impl std::error::Error for LoadVerificationError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PublicationVerificationError {
    MissingEntry {
        resource: ResourceKey,
        target: HarnessId,
    },
    Failed {
        resource: ResourceKey,
        target: HarnessId,
        error: LoadVerificationError,
    },
}

impl std::fmt::Display for PublicationVerificationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingEntry { resource, target } => write!(
                formatter,
                "publication receipt contains an unknown `{resource}` for `{target}`"
            ),
            Self::Failed {
                resource,
                target,
                error,
            } => write!(
                formatter,
                "load verification for `{resource}` on `{target}` failed: {error}"
            ),
        }
    }
}

impl std::error::Error for PublicationVerificationError {}

#[derive(Debug)]
pub enum PublicationStateError {
    EmptyReceipt,
    MissingVerification {
        resource: ResourceKey,
        target: HarnessId,
    },
    ConflictingArtifacts {
        resource: ResourceKey,
    },
    State(SchemaError),
}

impl std::fmt::Display for PublicationStateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyReceipt => {
                formatter.write_str("publication receipt has no verified targets")
            }
            Self::MissingVerification { resource, target } => write!(
                formatter,
                "publication `{resource}` for `{target}` has no matching verification"
            ),
            Self::ConflictingArtifacts { resource } => write!(
                formatter,
                "verified publication entries for `{resource}` disagree on managed artifact identity"
            ),
            Self::State(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for PublicationStateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::State(error) => Some(error),
            _ => None,
        }
    }
}

/// Verify all managed publications against fresh effective observations. The
/// verifier is supplied by the harness adapter and must not inspect caches.
pub fn verify_publication<V: LoadVerifier>(
    batch: &PublicationBatch,
    receipt: PublicationReceipt,
    verifier: &V,
) -> Result<PublicationReceipt, PublicationVerificationError> {
    let mut verified = Vec::with_capacity(receipt.published.len());
    for artifact in &receipt.published {
        let Some(entry) = batch.entries.iter().find(|entry| {
            entry.resource == *artifact.resource() && entry.target == *artifact.target()
        }) else {
            return Err(PublicationVerificationError::MissingEntry {
                resource: artifact.resource().clone(),
                target: artifact.target().clone(),
            });
        };
        let target = verifier.verify_loaded(entry, artifact).map_err(|error| {
            PublicationVerificationError::Failed {
                resource: entry.resource.clone(),
                target: entry.target.clone(),
                error,
            }
        })?;
        verified.push(target);
    }
    Ok(PublicationReceipt {
        published: receipt.published,
        verified,
    })
}

/// Refresh ownership, managed-artifact provenance, and verified native ids in
/// one pure state document. Callers publish the returned document through the
/// existing state repository while holding the configuration lock.
pub fn record_verified_publication(
    state: &StateDocument,
    receipt: &PublicationReceipt,
    at: Timestamp,
) -> Result<StateDocument, PublicationStateError> {
    if receipt.published.is_empty() || receipt.verified.is_empty() {
        return Err(PublicationStateError::EmptyReceipt);
    }
    let mut grouped: BTreeMap<ResourceKey, Vec<(&PublishedArtifact, &VerifiedTarget)>> =
        BTreeMap::new();
    for artifact in &receipt.published {
        let Some(verified) = receipt.verified.iter().find(|verified| {
            verified.resource() == artifact.resource() && verified.target() == artifact.target()
        }) else {
            return Err(PublicationStateError::MissingVerification {
                resource: artifact.resource().clone(),
                target: artifact.target().clone(),
            });
        };
        grouped
            .entry(artifact.resource().clone())
            .or_default()
            .push((artifact, verified));
    }
    for verified in &receipt.verified {
        if !receipt.published.iter().any(|artifact| {
            artifact.resource() == verified.resource() && artifact.target() == verified.target()
        }) {
            return Err(PublicationStateError::MissingVerification {
                resource: verified.resource().clone(),
                target: verified.target().clone(),
            });
        }
    }

    let mut updated = state.clone();
    for (resource, entries) in grouped {
        let current = state.resources().get(&resource).ok_or_else(|| {
            PublicationStateError::State(SchemaError::StateResourceNotFound {
                resource: resource.clone(),
            })
        })?;
        let first_record = entries[0].0.record();
        if entries
            .iter()
            .any(|(artifact, _)| artifact.record() != first_record)
        {
            return Err(PublicationStateError::ConflictingArtifacts { resource });
        }
        let provenance = match first_record.role() {
            ArtifactRole::MaterializedPlugin => Provenance::Materialized,
            ArtifactRole::DirectSkill => Provenance::Direct,
            ArtifactRole::Backup => {
                return Err(PublicationStateError::State(
                    SchemaError::InvalidArtifactRole {
                        resource: resource.clone(),
                    },
                ));
            }
        };
        let mut native_ids = current.native_ids().clone();
        for (_, verified) in entries {
            native_ids.insert(
                verified.target().clone(),
                verified.native_identity().clone(),
            );
        }
        let refreshed = ResourceState::new(
            resource.clone(),
            native_ids,
            provenance,
            Ownership::Skilltap,
            current.source().cloned(),
            Some(first_record.clone()),
            first_record.fingerprint().cloned(),
            current.installed_revision().cloned(),
            current.available_revision().cloned(),
            current.observed_at(),
            current.last_apply().cloned(),
        )
        .map_err(PublicationStateError::State)?;
        updated = updated
            .refresh_resource_state(refreshed)
            .map_err(PublicationStateError::State)?;
    }
    StateDocument::new(
        updated.schema(),
        updated.harnesses().values().cloned(),
        updated.resources().values().cloned(),
        updated.last_update_check(),
        updated.last_successful_observation(),
        Some(at),
    )
    .map_err(PublicationStateError::State)
}

/// Compare one fresh effective observation to the exact publication entry.
/// Harness adapters can use this after obtaining a bounded native snapshot.
pub fn verify_observed_load(
    entry: &PublicationEntry,
    artifact: &PublishedArtifact,
    observed: Option<&ObservedResource>,
) -> Result<VerifiedTarget, LoadVerificationError> {
    if artifact.resource() != &entry.resource {
        return Err(LoadVerificationError::ResourceMismatch);
    }
    if artifact.target() != &entry.target {
        return Err(LoadVerificationError::TargetMismatch);
    }
    let Some(observed) = observed else {
        return Err(LoadVerificationError::MissingEffectiveObservation);
    };
    if observed.key().resource() != &entry.resource {
        return Err(LoadVerificationError::ResourceMismatch);
    }
    if observed.key().harness() != &entry.target {
        return Err(LoadVerificationError::TargetMismatch);
    }
    if observed.key().layer() != ObservationLayer::Effective {
        return Err(LoadVerificationError::MissingEffectiveObservation);
    }
    if observed.health() != ResourceHealth::Healthy {
        return Err(LoadVerificationError::Unhealthy(observed.health()));
    }
    let Some(fingerprint) = observed.fingerprint() else {
        return Err(LoadVerificationError::MissingFingerprint);
    };
    if fingerprint != &entry.fingerprint {
        return Err(LoadVerificationError::FingerprintMismatch);
    }
    Ok(VerifiedTarget::new(
        entry.resource.clone(),
        entry.target.clone(),
        observed.native_identity().clone(),
    ))
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
    Ok(PublicationReceipt {
        published,
        verified: Vec::new(),
    })
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
    use std::collections::BTreeMap;

    use super::*;
    use crate::domain::{
        ComponentGraph, FingerprintAlgorithm, ObservationKey, Ownership, Provenance, ResourceId,
        ResourceKind, Scope,
    };

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

    fn observed(
        entry: &PublicationEntry,
        health: ResourceHealth,
        fingerprint: Option<Fingerprint>,
    ) -> ObservedResource {
        ObservedResource::new(
            ObservationKey::new(
                entry.resource.clone(),
                entry.target.clone(),
                ObservationLayer::Effective,
            ),
            ResourceKind::Plugin,
            Provenance::Materialized,
            Ownership::Skilltap,
            health,
            None,
            ComponentGraph::new([]).unwrap(),
            [].into(),
            NativeId::new("native-demo").unwrap(),
            None,
            fingerprint,
        )
    }

    #[test]
    fn effective_load_verification_requires_healthy_matching_fingerprint() {
        let entry = entry("plugin:a", "codex", 'a');
        let record = ManagedArtifactRecord::for_artifact(
            entry.resource.clone(),
            entry.role,
            entry.fingerprint.clone(),
        )
        .unwrap();
        let artifact =
            PublishedArtifact::new(entry.resource.clone(), entry.target.clone(), record, false);
        let loaded = observed(
            &entry,
            ResourceHealth::Healthy,
            Some(entry.fingerprint.clone()),
        );
        let verified = verify_observed_load(&entry, &artifact, Some(&loaded)).unwrap();
        assert_eq!(verified.resource(), &entry.resource);
        assert_eq!(verified.target(), &entry.target);
        assert_eq!(verified.native_identity().as_str(), "native-demo");

        let mismatch = observed(&entry, ResourceHealth::Healthy, Some(fingerprint('b')));
        assert_eq!(
            verify_observed_load(&entry, &artifact, Some(&mismatch)),
            Err(LoadVerificationError::FingerprintMismatch)
        );
        let unhealthy = observed(
            &entry,
            ResourceHealth::Degraded,
            Some(entry.fingerprint.clone()),
        );
        assert_eq!(
            verify_observed_load(&entry, &artifact, Some(&unhealthy)),
            Err(LoadVerificationError::Unhealthy(ResourceHealth::Degraded))
        );
    }

    #[test]
    fn verified_publication_refreshes_owned_state_after_load_verification() {
        let entry = entry("plugin:a", "codex", 'a');
        let record = ManagedArtifactRecord::for_artifact(
            entry.resource.clone(),
            entry.role,
            entry.fingerprint.clone(),
        )
        .unwrap();
        let artifact =
            PublishedArtifact::new(entry.resource.clone(), entry.target.clone(), record, false);
        let receipt = PublicationReceipt {
            published: vec![artifact],
            verified: vec![VerifiedTarget::new(
                entry.resource.clone(),
                entry.target.clone(),
                NativeId::new("native-demo").unwrap(),
            )],
        };
        let current = ResourceState::new(
            entry.resource.clone(),
            BTreeMap::new(),
            Provenance::Native,
            Ownership::Harness,
            None,
            None,
            None,
            None,
            None,
            Timestamp::new(1, 0).unwrap(),
            None,
        )
        .unwrap();
        let state = StateDocument::new(1, [], [current], None, None, None).unwrap();
        let refreshed =
            record_verified_publication(&state, &receipt, Timestamp::new(2, 0).unwrap()).unwrap();
        let updated = refreshed.resources().get(&entry.resource).unwrap();
        assert_eq!(updated.provenance(), Provenance::Materialized);
        assert_eq!(updated.ownership(), Ownership::Skilltap);
        assert_eq!(updated.native_ids()[&entry.target].as_str(), "native-demo");
        assert_eq!(
            refreshed.last_successful_application(),
            Some(Timestamp::new(2, 0).unwrap())
        );
    }
}
