use std::{
    collections::BTreeMap,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Deserializer, Serialize};

use super::{ArtifactRole, ManagedArtifactRecord, STATE_SCHEMA_VERSION, SchemaError};
use crate::domain::{
    ComponentId, EvidenceCode, Fingerprint, HarnessId, NativeId, OperationId, OperationResult,
    Ownership, Provenance, RelativeArtifactPath, ResolvedRevision, ResourceKey, Source,
};

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "id")]
pub enum ManagedProjection {
    Skill {
        id: RelativeArtifactPath,
        fingerprint: Fingerprint,
    },
    Mcp {
        id: NativeId,
        fingerprint: Fingerprint,
    },
    Omitted {
        id: ComponentId,
        consequence: EvidenceCode,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(try_from = "TimestampWire", into = "TimestampWire")]
pub struct Timestamp {
    seconds: u64,
    nanoseconds: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct TimestampWire {
    seconds: u64,
    nanoseconds: u32,
}

impl Timestamp {
    pub fn new(seconds: u64, nanoseconds: u32) -> Result<Self, SchemaError> {
        if nanoseconds >= 1_000_000_000 {
            return Err(SchemaError::InvalidNanoseconds { nanoseconds });
        }
        let timestamp = Self {
            seconds,
            nanoseconds,
        };
        UNIX_EPOCH
            .checked_add(Duration::new(seconds, nanoseconds))
            .ok_or(SchemaError::TimestampOutOfRange)?;
        Ok(timestamp)
    }

    pub const fn seconds(self) -> u64 {
        self.seconds
    }
    pub const fn nanoseconds(self) -> u32 {
        self.nanoseconds
    }

    pub fn from_system_time(value: SystemTime) -> Result<Self, SchemaError> {
        let duration = value
            .duration_since(UNIX_EPOCH)
            .map_err(|_| SchemaError::TimestampBeforeEpoch)?;
        Self::new(duration.as_secs(), duration.subsec_nanos())
    }

    pub fn to_system_time(self) -> Result<SystemTime, SchemaError> {
        UNIX_EPOCH
            .checked_add(Duration::new(self.seconds, self.nanoseconds))
            .ok_or(SchemaError::TimestampOutOfRange)
    }
}

impl From<Timestamp> for TimestampWire {
    fn from(value: Timestamp) -> Self {
        Self {
            seconds: value.seconds,
            nanoseconds: value.nanoseconds,
        }
    }
}

impl TryFrom<TimestampWire> for Timestamp {
    type Error = SchemaError;
    fn try_from(value: TimestampWire) -> Result<Self, Self::Error> {
        Self::new(value.seconds, value.nanoseconds)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(into = "ApplyRecordWire")]
pub struct ApplyRecord {
    at: Timestamp,
    operations: BTreeMap<OperationId, OperationResult>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ApplyRecordWire {
    at: Timestamp,
    operations: Vec<OperationResult>,
}

impl ApplyRecord {
    pub fn new(
        at: Timestamp,
        operations: impl IntoIterator<Item = OperationResult>,
    ) -> Result<Self, SchemaError> {
        let mut collected = BTreeMap::new();
        for operation in operations {
            let id = operation.operation_id().clone();
            if collected.insert(id.clone(), operation).is_some() {
                return Err(SchemaError::DuplicateOperation { operation: id });
            }
        }
        Ok(Self {
            at,
            operations: collected,
        })
    }
    pub const fn at(&self) -> Timestamp {
        self.at
    }
    pub const fn operations(&self) -> &BTreeMap<OperationId, OperationResult> {
        &self.operations
    }
}

impl From<ApplyRecord> for ApplyRecordWire {
    fn from(value: ApplyRecord) -> Self {
        Self {
            at: value.at,
            operations: value.operations.into_values().collect(),
        }
    }
}

impl TryFrom<ApplyRecordWire> for ApplyRecord {
    type Error = SchemaError;
    fn try_from(value: ApplyRecordWire) -> Result<Self, Self::Error> {
        Self::new(value.at, value.operations)
    }
}

impl<'de> Deserialize<'de> for ApplyRecord {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        ApplyRecordWire::deserialize(deserializer)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DaemonRunResult {
    Completed,
    Pending,
    Contended,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(into = "DaemonRunRecordWire")]
pub struct DaemonRunRecord {
    at: Timestamp,
    result: DaemonRunResult,
    safe_operations: u64,
    pending_operations: u64,
    failure_code: Option<EvidenceCode>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct DaemonRunRecordWire {
    at: Timestamp,
    result: DaemonRunResult,
    safe_operations: u64,
    pending_operations: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    failure_code: Option<EvidenceCode>,
}

impl DaemonRunRecord {
    pub fn new(
        at: Timestamp,
        result: DaemonRunResult,
        safe_operations: u64,
        pending_operations: u64,
        failure_code: Option<EvidenceCode>,
    ) -> Result<Self, SchemaError> {
        if let Some(code) = &failure_code
            && !matches!(
                code.as_str(),
                "daemon.lock_contended"
                    | "daemon.source_unreachable"
                    | "daemon.update_failed"
                    | "daemon.manager_unavailable"
            )
        {
            return Err(SchemaError::InvalidDaemonFailureCode);
        }
        if matches!(result, DaemonRunResult::Completed) && failure_code.is_some() {
            return Err(SchemaError::InvalidDaemonFailureCode);
        }
        Ok(Self {
            at,
            result,
            safe_operations,
            pending_operations,
            failure_code,
        })
    }

    pub const fn at(&self) -> Timestamp {
        self.at
    }

    pub const fn result(&self) -> DaemonRunResult {
        self.result
    }

    pub const fn safe_operations(&self) -> u64 {
        self.safe_operations
    }

    pub const fn pending_operations(&self) -> u64 {
        self.pending_operations
    }

    pub const fn failure_code(&self) -> Option<&EvidenceCode> {
        self.failure_code.as_ref()
    }
}

impl From<DaemonRunRecord> for DaemonRunRecordWire {
    fn from(value: DaemonRunRecord) -> Self {
        Self {
            at: value.at,
            result: value.result,
            safe_operations: value.safe_operations,
            pending_operations: value.pending_operations,
            failure_code: value.failure_code,
        }
    }
}

impl TryFrom<DaemonRunRecordWire> for DaemonRunRecord {
    type Error = SchemaError;

    fn try_from(value: DaemonRunRecordWire) -> Result<Self, Self::Error> {
        Self::new(
            value.at,
            value.result,
            value.safe_operations,
            value.pending_operations,
            value.failure_code,
        )
    }
}

impl<'de> Deserialize<'de> for DaemonRunRecord {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        DaemonRunRecordWire::deserialize(deserializer)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HarnessState {
    pub harness: HarnessId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_version: Option<NativeId>,
    pub observed_at: Timestamp,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(into = "TargetResourceStateWire")]
pub struct TargetResourceState {
    harness: HarnessId,
    #[serde(skip_serializing_if = "Option::is_none")]
    native_id: Option<NativeId>,
    provenance: Provenance,
    ownership: Ownership,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<Source>,
    #[serde(skip_serializing_if = "Option::is_none")]
    managed_artifact: Option<ManagedArtifactRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fingerprint: Option<Fingerprint>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    managed_projections: Vec<ManagedProjection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pending_managed_attempt: Option<PendingManagedAttempt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    installed_revision: Option<ResolvedRevision>,
    #[serde(skip_serializing_if = "Option::is_none")]
    available_revision: Option<ResolvedRevision>,
    observed_at: Timestamp,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_apply: Option<ApplyRecord>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct TargetResourceStateWire {
    harness: HarnessId,
    #[serde(skip_serializing_if = "Option::is_none")]
    native_id: Option<NativeId>,
    provenance: Provenance,
    ownership: Ownership,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<Source>,
    #[serde(skip_serializing_if = "Option::is_none")]
    managed_artifact: Option<ManagedArtifactRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fingerprint: Option<Fingerprint>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    managed_projections: Vec<ManagedProjection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pending_managed_attempt: Option<PendingManagedAttempt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    installed_revision: Option<ResolvedRevision>,
    #[serde(skip_serializing_if = "Option::is_none")]
    available_revision: Option<ResolvedRevision>,
    observed_at: Timestamp,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_apply: Option<ApplyRecord>,
}

impl TargetResourceState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        harness: HarnessId,
        native_id: Option<NativeId>,
        provenance: Provenance,
        ownership: Ownership,
        source: Option<Source>,
        managed_artifact: Option<ManagedArtifactRecord>,
        fingerprint: Option<Fingerprint>,
        installed_revision: Option<ResolvedRevision>,
        available_revision: Option<ResolvedRevision>,
        observed_at: Timestamp,
        last_apply: Option<ApplyRecord>,
    ) -> Result<Self, SchemaError> {
        let ownership_valid = matches!(
            (provenance, ownership),
            (
                Provenance::Direct | Provenance::Materialized,
                Ownership::Skilltap
            ) | (
                Provenance::Native | Provenance::Adopted,
                Ownership::Harness | Ownership::Unmanaged
            )
        );
        if !ownership_valid {
            return Err(SchemaError::InvalidTargetOwnership { harness });
        }
        if let Some(artifact) = &managed_artifact {
            let role_valid = matches!(
                (artifact.role(), provenance),
                (ArtifactRole::MaterializedPlugin, Provenance::Materialized)
                    | (ArtifactRole::DirectSkill, Provenance::Direct)
                    | (
                        ArtifactRole::Backup,
                        Provenance::Direct | Provenance::Materialized
                    )
            );
            if !role_valid {
                return Err(SchemaError::InvalidTargetArtifactRole { harness });
            }
        }
        Ok(Self {
            harness,
            native_id,
            provenance,
            ownership,
            source,
            managed_artifact,
            fingerprint,
            managed_projections: Vec::new(),
            pending_managed_attempt: None,
            installed_revision,
            available_revision,
            observed_at,
            last_apply,
        })
    }
    pub const fn harness(&self) -> &HarnessId {
        &self.harness
    }
    pub const fn native_id(&self) -> Option<&NativeId> {
        self.native_id.as_ref()
    }
    pub const fn provenance(&self) -> Provenance {
        self.provenance
    }
    pub const fn ownership(&self) -> Ownership {
        self.ownership
    }
    pub const fn managed_artifact(&self) -> Option<&ManagedArtifactRecord> {
        self.managed_artifact.as_ref()
    }
    pub const fn source(&self) -> Option<&Source> {
        self.source.as_ref()
    }
    pub const fn fingerprint(&self) -> Option<&Fingerprint> {
        self.fingerprint.as_ref()
    }
    pub fn managed_projections(&self) -> &[ManagedProjection] {
        &self.managed_projections
    }
    pub fn with_managed_projections(
        mut self,
        projections: impl IntoIterator<Item = ManagedProjection>,
    ) -> Self {
        self.managed_projections = projections.into_iter().collect();
        self.managed_projections.sort();
        self.managed_projections.dedup();
        self
    }
    pub const fn pending_managed_attempt(&self) -> Option<&PendingManagedAttempt> {
        self.pending_managed_attempt.as_ref()
    }
    pub fn with_pending_managed_attempt(mut self, attempt: PendingManagedAttempt) -> Self {
        self.pending_managed_attempt = Some(attempt);
        self
    }
    fn with_pending_managed_attempt_option(
        mut self,
        attempt: Option<PendingManagedAttempt>,
    ) -> Self {
        self.pending_managed_attempt = attempt;
        self
    }
    pub const fn installed_revision(&self) -> Option<&ResolvedRevision> {
        self.installed_revision.as_ref()
    }
    pub const fn available_revision(&self) -> Option<&ResolvedRevision> {
        self.available_revision.as_ref()
    }
    pub const fn observed_at(&self) -> Timestamp {
        self.observed_at
    }
    pub const fn last_apply(&self) -> Option<&ApplyRecord> {
        self.last_apply.as_ref()
    }
}

impl From<TargetResourceState> for TargetResourceStateWire {
    fn from(value: TargetResourceState) -> Self {
        Self {
            harness: value.harness,
            native_id: value.native_id,
            provenance: value.provenance,
            ownership: value.ownership,
            source: value.source,
            managed_artifact: value.managed_artifact,
            fingerprint: value.fingerprint,
            managed_projections: value.managed_projections,
            pending_managed_attempt: value.pending_managed_attempt,
            installed_revision: value.installed_revision,
            available_revision: value.available_revision,
            observed_at: value.observed_at,
            last_apply: value.last_apply,
        }
    }
}

impl TryFrom<TargetResourceStateWire> for TargetResourceState {
    type Error = SchemaError;

    fn try_from(value: TargetResourceStateWire) -> Result<Self, Self::Error> {
        validate_managed_projections(&value.managed_projections)?;
        if let Some(attempt) = &value.pending_managed_attempt {
            validate_managed_projections(attempt.managed_projections())?;
        }
        Self::new(
            value.harness,
            value.native_id,
            value.provenance,
            value.ownership,
            value.source,
            value.managed_artifact,
            value.fingerprint,
            value.installed_revision,
            value.available_revision,
            value.observed_at,
            value.last_apply,
        )
        .map(|state| state.with_managed_projections(value.managed_projections))
        .map(|state| match value.pending_managed_attempt {
            Some(attempt) => state.with_pending_managed_attempt(attempt),
            None => state,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PendingManagedAttempt {
    operation_id: OperationId,
    fingerprint: Fingerprint,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    managed_projections: Vec<ManagedProjection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    installed_revision: Option<ResolvedRevision>,
}

impl PendingManagedAttempt {
    pub fn new(
        operation_id: OperationId,
        fingerprint: Fingerprint,
        projections: impl IntoIterator<Item = ManagedProjection>,
        installed_revision: Option<ResolvedRevision>,
    ) -> Result<Self, SchemaError> {
        let mut managed_projections = projections.into_iter().collect::<Vec<_>>();
        validate_managed_projections(&managed_projections)?;
        managed_projections.sort();
        managed_projections.dedup();
        Ok(Self {
            operation_id,
            fingerprint,
            managed_projections,
            installed_revision,
        })
    }
    pub const fn operation_id(&self) -> &OperationId {
        &self.operation_id
    }
    pub const fn fingerprint(&self) -> &Fingerprint {
        &self.fingerprint
    }
    pub fn managed_projections(&self) -> &[ManagedProjection] {
        &self.managed_projections
    }
    pub const fn installed_revision(&self) -> Option<&ResolvedRevision> {
        self.installed_revision.as_ref()
    }
}

fn validate_managed_projections(projections: &[ManagedProjection]) -> Result<(), SchemaError> {
    let mut identities = BTreeMap::<String, &ManagedProjection>::new();
    for projection in projections {
        let identity = match projection {
            ManagedProjection::Skill { id, .. } => format!("skill:{}", id.as_str()),
            ManagedProjection::Mcp { id, .. } => format!("mcp:{}", id.as_str()),
            ManagedProjection::Omitted { id, .. } => format!("omitted:{}", id.as_str()),
        };
        if let Some(previous) = identities.insert(identity, projection)
            && previous != projection
        {
            return Err(SchemaError::ConflictingManagedProjection);
        }
    }
    Ok(())
}

impl<'de> Deserialize<'de> for TargetResourceState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        TargetResourceStateWire::deserialize(deserializer)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(into = "ResourceStateWire")]
pub struct ResourceState {
    key: ResourceKey,
    targets: BTreeMap<HarnessId, TargetResourceState>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ResourceStateWire {
    key: ResourceKey,
    targets: Vec<TargetBindingWire>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct TargetBindingWire {
    target: HarnessId,
    binding: TargetResourceState,
}

impl ResourceState {
    pub fn new(
        key: ResourceKey,
        targets: impl IntoIterator<Item = TargetResourceState>,
    ) -> Result<Self, SchemaError> {
        let mut target_map = BTreeMap::new();
        for target in targets {
            let harness = target.harness().clone();
            if let Some(artifact) = target.managed_artifact() {
                artifact.validate_for_owner(&key)?;
            }
            if target_map.insert(harness.clone(), target).is_some() {
                return Err(SchemaError::DuplicateTargetBinding {
                    resource: key,
                    harness,
                });
            }
        }
        if target_map.is_empty() {
            return Err(SchemaError::EmptyTargetBindings { resource: key });
        }
        Ok(Self {
            key,
            targets: target_map,
        })
    }

    pub const fn key(&self) -> &ResourceKey {
        &self.key
    }

    pub const fn targets(&self) -> &BTreeMap<HarnessId, TargetResourceState> {
        &self.targets
    }

    pub fn target(&self, harness: &HarnessId) -> Option<&TargetResourceState> {
        self.targets.get(harness)
    }

    pub fn with_target(&self, target: TargetResourceState) -> Result<Self, SchemaError> {
        let mut targets = self.targets.clone();
        targets.insert(target.harness().clone(), target);
        Self::new(self.key.clone(), targets.into_values())
    }

    pub fn without_targets(
        &self,
        selected: &crate::domain::HarnessSet,
    ) -> Result<Option<Self>, SchemaError> {
        let remaining = self
            .targets
            .iter()
            .filter(|(harness, _)| !selected.contains(harness))
            .map(|(_, target)| target.clone())
            .collect::<Vec<_>>();
        if remaining.is_empty() {
            Ok(None)
        } else {
            Self::new(self.key.clone(), remaining).map(Some)
        }
    }
}

impl From<ResourceState> for ResourceStateWire {
    fn from(value: ResourceState) -> Self {
        Self {
            key: value.key,
            targets: value
                .targets
                .into_iter()
                .map(|(target, binding)| TargetBindingWire { target, binding })
                .collect(),
        }
    }
}

impl TryFrom<ResourceStateWire> for ResourceState {
    type Error = SchemaError;

    fn try_from(value: ResourceStateWire) -> Result<Self, Self::Error> {
        for target in &value.targets {
            if target.target != *target.binding.harness() {
                return Err(SchemaError::TargetBindingMismatch {
                    resource: value.key,
                    map_key: target.target.clone(),
                    binding: target.binding.harness().clone(),
                });
            }
        }
        Self::new(
            value.key,
            value.targets.into_iter().map(|target| target.binding),
        )
    }
}

impl<'de> Deserialize<'de> for ResourceState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        ResourceStateWire::deserialize(deserializer)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(into = "StateWire")]
pub struct StateDocument {
    harnesses: BTreeMap<HarnessId, HarnessState>,
    resources: BTreeMap<ResourceKey, ResourceState>,
    last_update_check: Option<Timestamp>,
    last_successful_observation: Option<Timestamp>,
    last_successful_application: Option<Timestamp>,
    daemon_run: Option<DaemonRunRecord>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct StateWire {
    schema: u32,
    harnesses: Vec<HarnessState>,
    resources: Vec<ResourceState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_update_check: Option<Timestamp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_successful_observation: Option<Timestamp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_successful_application: Option<Timestamp>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    daemon_run: Option<DaemonRunRecord>,
}

impl StateDocument {
    pub const fn schema(&self) -> u32 {
        STATE_SCHEMA_VERSION
    }

    pub fn new(
        schema: u32,
        harnesses: impl IntoIterator<Item = HarnessState>,
        resources: impl IntoIterator<Item = ResourceState>,
        last_update_check: Option<Timestamp>,
        last_successful_observation: Option<Timestamp>,
        last_successful_application: Option<Timestamp>,
    ) -> Result<Self, SchemaError> {
        if schema != STATE_SCHEMA_VERSION {
            return Err(SchemaError::UnsupportedVersion {
                document: "state",
                version: schema,
            });
        }
        let mut harness_map = BTreeMap::new();
        for state in harnesses {
            let id = state.harness.clone();
            if harness_map.insert(id.clone(), state).is_some() {
                return Err(SchemaError::DuplicateHarness { harness: id });
            }
        }
        let mut resource_map = BTreeMap::new();
        let mut managed_paths = BTreeMap::new();
        for state in resources {
            let id = state.key.clone();
            for target in state.targets().values() {
                if let Some(artifact) = target.managed_artifact()
                    && managed_paths
                        .insert(artifact.path().clone(), id.clone())
                        .is_some_and(|owner| owner != id)
                {
                    return Err(SchemaError::DuplicateManagedPath {
                        path: artifact.path().clone(),
                    });
                }
            }
            if resource_map.insert(id.clone(), state).is_some() {
                return Err(SchemaError::DuplicateStateResource { resource: id });
            }
        }
        Ok(Self {
            harnesses: harness_map,
            resources: resource_map,
            last_update_check,
            last_successful_observation,
            last_successful_application,
            daemon_run: None,
        })
    }
    pub const fn harnesses(&self) -> &BTreeMap<HarnessId, HarnessState> {
        &self.harnesses
    }
    pub const fn resources(&self) -> &BTreeMap<ResourceKey, ResourceState> {
        &self.resources
    }
    pub const fn last_update_check(&self) -> Option<Timestamp> {
        self.last_update_check
    }
    pub const fn last_successful_observation(&self) -> Option<Timestamp> {
        self.last_successful_observation
    }
    pub const fn last_successful_application(&self) -> Option<Timestamp> {
        self.last_successful_application
    }

    pub const fn daemon_run(&self) -> Option<&DaemonRunRecord> {
        self.daemon_run.as_ref()
    }

    /// Return a copy with one typed daemon result attached. Callers publish
    /// the returned document through `StateRepository`.
    pub fn with_daemon_run(&self, record: DaemonRunRecord) -> Result<Self, SchemaError> {
        let mut next = Self::new(
            STATE_SCHEMA_VERSION,
            self.harnesses.values().cloned(),
            self.resources.values().cloned(),
            self.last_update_check,
            self.last_successful_observation,
            self.last_successful_application,
        )?;
        next.daemon_run = Some(record);
        Ok(next)
    }

    fn preserve_daemon_run(&self, next: Self) -> Result<Self, SchemaError> {
        match self.daemon_run.clone() {
            Some(record) => next.with_daemon_run(record),
            None => Ok(next),
        }
    }

    /// Return a copy with one operation result atomically attached to its
    /// exact resource record. Callers publish the returned document through
    /// `StateRepository`; this method performs no I/O.
    pub fn with_operation_result(
        &self,
        resource: &ResourceKey,
        target: &HarnessId,
        at: Timestamp,
        operation: OperationResult,
    ) -> Result<Self, SchemaError> {
        let current =
            self.resources
                .get(resource)
                .ok_or_else(|| SchemaError::StateResourceNotFound {
                    resource: resource.clone(),
                })?;
        let current_target =
            current
                .target(target)
                .ok_or_else(|| SchemaError::TargetBindingNotFound {
                    resource: resource.clone(),
                    harness: target.clone(),
                })?;
        let mut operations = current_target
            .last_apply()
            .map(|record| record.operations().clone())
            .unwrap_or_default();
        operations.insert(operation.operation_id().clone(), operation);
        let apply = ApplyRecord::new(at, operations.into_values())?;
        let updated_target = TargetResourceState::new(
            current_target.harness().clone(),
            current_target.native_id().cloned(),
            current_target.provenance(),
            current_target.ownership(),
            current_target.source().cloned(),
            current_target.managed_artifact().cloned(),
            current_target.fingerprint().cloned(),
            current_target.installed_revision().cloned(),
            current_target.available_revision().cloned(),
            current_target.observed_at(),
            Some(apply),
        )?
        .with_managed_projections(current_target.managed_projections().iter().cloned())
        .with_pending_managed_attempt_option(current_target.pending_managed_attempt().cloned());
        let updated = current.with_target(updated_target)?;
        let mut resources = self.resources.values().cloned().collect::<Vec<_>>();
        resources.retain(|value| value.key() != resource);
        resources.push(updated);
        self.preserve_daemon_run(Self::new(
            STATE_SCHEMA_VERSION,
            self.harnesses.values().cloned(),
            resources,
            self.last_update_check,
            self.last_successful_observation,
            Some(at),
        )?)
    }

    /// Cache one freshly resolved available revision while preserving the
    /// installed resource, ownership, fingerprint, and operation journal.
    /// Callers can compose several returned documents and publish once through
    /// `StateRepository` for an atomic multi-resource check.
    pub fn with_available_revision(
        &self,
        resource: &ResourceKey,
        target: &HarnessId,
        available: Option<ResolvedRevision>,
        checked_at: Timestamp,
    ) -> Result<Self, SchemaError> {
        let current =
            self.resources
                .get(resource)
                .ok_or_else(|| SchemaError::StateResourceNotFound {
                    resource: resource.clone(),
                })?;
        let current_target =
            current
                .target(target)
                .ok_or_else(|| SchemaError::TargetBindingNotFound {
                    resource: resource.clone(),
                    harness: target.clone(),
                })?;
        let updated_target = TargetResourceState::new(
            current_target.harness().clone(),
            current_target.native_id().cloned(),
            current_target.provenance(),
            current_target.ownership(),
            current_target.source().cloned(),
            current_target.managed_artifact().cloned(),
            current_target.fingerprint().cloned(),
            current_target.installed_revision().cloned(),
            available,
            current_target.observed_at(),
            current_target.last_apply().cloned(),
        )?
        .with_managed_projections(current_target.managed_projections().iter().cloned())
        .with_pending_managed_attempt_option(current_target.pending_managed_attempt().cloned());
        let updated = current.with_target(updated_target)?;
        let mut resources = self.resources.values().cloned().collect::<Vec<_>>();
        resources.retain(|value| value.key() != resource);
        resources.push(updated);
        self.preserve_daemon_run(Self::new(
            STATE_SCHEMA_VERSION,
            self.harnesses.values().cloned(),
            resources,
            Some(checked_at),
            self.last_successful_observation,
            self.last_successful_application,
        )?)
    }

    /// Return a copy with a resource seed added idempotently.
    pub fn with_resource_state(&self, resource: ResourceState) -> Result<Self, SchemaError> {
        if let Some(existing) = self.resources.get(resource.key()) {
            if existing == &resource {
                return Ok(self.clone());
            }
            return Err(SchemaError::StateResourceConflict {
                resource: resource.key().clone(),
            });
        }
        self.preserve_daemon_run(Self::new(
            STATE_SCHEMA_VERSION,
            self.harnesses.values().cloned(),
            self.resources
                .values()
                .cloned()
                .chain(std::iter::once(resource)),
            self.last_update_check,
            self.last_successful_observation,
            self.last_successful_application,
        )?)
    }

    /// Merge refreshed target bindings into an already-known logical resource.
    /// Unselected siblings are preserved, and an existing target journal is
    /// retained until an exact-target operation result replaces it.
    pub fn refresh_resource_state(&self, resource: ResourceState) -> Result<Self, SchemaError> {
        let Some(existing) = self.resources.get(resource.key()) else {
            return self.with_resource_state(resource);
        };
        let mut refreshed = existing.clone();
        for incoming in resource.targets().values() {
            let incoming = if incoming.last_apply().is_none() {
                if let Some(previous) = existing.target(incoming.harness()) {
                    TargetResourceState::new(
                        incoming.harness().clone(),
                        incoming.native_id().cloned(),
                        incoming.provenance(),
                        incoming.ownership(),
                        incoming.source().cloned(),
                        incoming.managed_artifact().cloned(),
                        incoming.fingerprint().cloned(),
                        incoming.installed_revision().cloned(),
                        incoming.available_revision().cloned(),
                        incoming.observed_at(),
                        previous.last_apply().cloned(),
                    )?
                    .with_managed_projections(incoming.managed_projections().iter().cloned())
                    .with_pending_managed_attempt_option(
                        incoming.pending_managed_attempt().cloned(),
                    )
                } else {
                    incoming.clone()
                }
            } else {
                incoming.clone()
            };
            refreshed = refreshed.with_target(incoming)?;
        }
        let mut resources = self.resources.values().cloned().collect::<Vec<_>>();
        resources.retain(|value| value.key() != resource.key());
        resources.push(refreshed);
        self.preserve_daemon_run(Self::new(
            STATE_SCHEMA_VERSION,
            self.harnesses.values().cloned(),
            resources,
            self.last_update_check,
            self.last_successful_observation,
            self.last_successful_application,
        )?)
    }

    /// Return a copy without a resource state record. Removing an absent
    /// record is idempotent and preserves the daemon journal.
    pub fn without_resource(&self, key: &ResourceKey) -> Result<Self, SchemaError> {
        if !self.resources.contains_key(key) {
            return Ok(self.clone());
        }
        let resources = self
            .resources
            .iter()
            .filter(|(resource_key, _)| *resource_key != key)
            .map(|(_, resource)| resource.clone())
            .collect::<Vec<_>>();
        self.preserve_daemon_run(Self::new(
            STATE_SCHEMA_VERSION,
            self.harnesses.values().cloned(),
            resources,
            self.last_update_check,
            self.last_successful_observation,
            self.last_successful_application,
        )?)
    }
}

impl From<StateDocument> for StateWire {
    fn from(value: StateDocument) -> Self {
        Self {
            schema: STATE_SCHEMA_VERSION,
            harnesses: value.harnesses.into_values().collect(),
            resources: value.resources.into_values().collect(),
            last_update_check: value.last_update_check,
            last_successful_observation: value.last_successful_observation,
            last_successful_application: value.last_successful_application,
            daemon_run: value.daemon_run,
        }
    }
}

impl TryFrom<StateWire> for StateDocument {
    type Error = SchemaError;
    fn try_from(value: StateWire) -> Result<Self, Self::Error> {
        Self::new(
            value.schema,
            value.harnesses,
            value.resources,
            value.last_update_check,
            value.last_successful_observation,
            value.last_successful_application,
        )
        .and_then(|state| match value.daemon_run {
            Some(record) => state.with_daemon_run(record),
            None => Ok(state),
        })
    }
}

impl<'de> Deserialize<'de> for StateDocument {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        StateWire::deserialize(deserializer)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}
