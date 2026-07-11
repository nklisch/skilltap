//! Ephemeral normalized observation snapshots and behavior ports.

use std::{collections::BTreeMap, fmt};

use serde::{Deserialize, Deserializer, Serialize};

use super::{
    CapabilityId, CapabilityProfileSelection, CapabilitySet, ConfiguredBinary, ExecutableIdentity,
    HarnessId, HarnessInstallation, HarnessReachability, NativeVersion, ObservationFinding,
    ObservationKey, ObservedResource, ProfileAuthority, Scope,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(into = "ObservationEvidenceWire")]
pub struct ObservationEvidence {
    harness: HarnessId,
    configured_binary: ConfiguredBinary,
    executable: ExecutableIdentity,
    native_version: NativeVersion,
    profile: CapabilityProfileSelection,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ObservationEvidenceWire {
    harness: HarnessId,
    configured_binary: ConfiguredBinary,
    executable: ExecutableIdentity,
    native_version: NativeVersion,
    profile: CapabilityProfileSelection,
}

impl ObservationEvidence {
    pub fn new(
        installation: &HarnessInstallation,
        profile: CapabilityProfileSelection,
    ) -> Result<Self, ObservationContractError> {
        let HarnessReachability::Reachable {
            executable,
            native_version,
        } = installation.reachability()
        else {
            return Err(ObservationContractError::InstallationUnreachable {
                harness: installation.harness().clone(),
            });
        };
        Ok(Self {
            harness: installation.harness().clone(),
            configured_binary: installation.configured_binary().clone(),
            executable: executable.clone(),
            native_version: native_version.clone(),
            profile,
        })
    }

    pub fn harness(&self) -> &HarnessId {
        &self.harness
    }
    pub const fn configured_binary(&self) -> &ConfiguredBinary {
        &self.configured_binary
    }
    pub const fn executable(&self) -> &ExecutableIdentity {
        &self.executable
    }
    pub const fn native_version(&self) -> &NativeVersion {
        &self.native_version
    }
    pub const fn profile(&self) -> &CapabilityProfileSelection {
        &self.profile
    }
    pub const fn profile_authority(&self) -> ProfileAuthority {
        self.profile.authority()
    }
    pub fn observation_capabilities(&self, scope: &Scope) -> &CapabilitySet {
        self.profile.observation_capabilities().for_scope(scope)
    }
}

impl From<ObservationEvidence> for ObservationEvidenceWire {
    fn from(value: ObservationEvidence) -> Self {
        Self {
            harness: value.harness,
            configured_binary: value.configured_binary,
            executable: value.executable,
            native_version: value.native_version,
            profile: value.profile,
        }
    }
}

impl From<ObservationEvidenceWire> for ObservationEvidence {
    fn from(value: ObservationEvidenceWire) -> Self {
        Self {
            harness: value.harness,
            configured_binary: value.configured_binary,
            executable: value.executable,
            native_version: value.native_version,
            profile: value.profile,
        }
    }
}

impl<'de> Deserialize<'de> for ObservationEvidence {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(ObservationEvidenceWire::deserialize(deserializer)?.into())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationTarget {
    harness: HarnessId,
    scope: Scope,
}

impl ObservationTarget {
    pub const fn new(harness: HarnessId, scope: Scope) -> Self {
        Self { harness, scope }
    }
    pub fn harness(&self) -> &HarnessId {
        &self.harness
    }
    pub const fn scope(&self) -> &Scope {
        &self.scope
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationRequest {
    scope: Scope,
    evidence: ObservationEvidence,
}

impl ObservationRequest {
    pub const fn new(scope: Scope, evidence: ObservationEvidence) -> Self {
        Self { scope, evidence }
    }
    pub const fn scope(&self) -> &Scope {
        &self.scope
    }
    pub const fn evidence(&self) -> &ObservationEvidence {
        &self.evidence
    }
    pub fn target(&self) -> ObservationTarget {
        ObservationTarget::new(self.evidence.harness.clone(), self.scope.clone())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(into = "HarnessObservationWire")]
pub struct HarnessObservation {
    request: ObservationRequest,
    resources: BTreeMap<ObservationKey, ObservedResource>,
    findings: Vec<ObservationFinding>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct HarnessObservationWire {
    request: ObservationRequest,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    resources: Vec<ObservedResource>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    findings: Vec<ObservationFinding>,
}

impl HarnessObservation {
    pub fn new(
        request: ObservationRequest,
        resources: impl IntoIterator<Item = ObservedResource>,
        findings: impl IntoIterator<Item = ObservationFinding>,
    ) -> Result<Self, ObservationContractError> {
        let target = request.target();
        let mut collected = BTreeMap::new();
        for resource in resources {
            let key = resource.key().clone();
            if key.harness() != target.harness() || resource.scope() != target.scope() {
                return Err(ObservationContractError::ResourceContextMismatch {
                    target: Box::new(target),
                    key: Box::new(key),
                });
            }
            if collected.insert(key.clone(), resource).is_some() {
                return Err(ObservationContractError::DuplicateObservation { key });
            }
        }
        let mut findings = findings.into_iter().collect::<Vec<_>>();
        if let Some(finding) = findings.iter().find(|finding| {
            finding.subject().harness() != target.harness()
                || finding.subject().scope() != target.scope()
        }) {
            return Err(ObservationContractError::FindingContextMismatch {
                target: Box::new(target),
                finding_target: Box::new(ObservationTarget::new(
                    finding.subject().harness().clone(),
                    finding.subject().scope().clone(),
                )),
            });
        }
        findings.sort();
        Ok(Self {
            request,
            resources: collected,
            findings,
        })
    }

    pub const fn request(&self) -> &ObservationRequest {
        &self.request
    }
    pub fn target(&self) -> ObservationTarget {
        self.request.target()
    }
    pub const fn resources(&self) -> &BTreeMap<ObservationKey, ObservedResource> {
        &self.resources
    }
    pub fn findings(&self) -> &[ObservationFinding] {
        &self.findings
    }
}

impl From<HarnessObservation> for HarnessObservationWire {
    fn from(value: HarnessObservation) -> Self {
        Self {
            request: value.request,
            resources: value.resources.into_values().collect(),
            findings: value.findings,
        }
    }
}

impl TryFrom<HarnessObservationWire> for HarnessObservation {
    type Error = ObservationContractError;
    fn try_from(value: HarnessObservationWire) -> Result<Self, Self::Error> {
        Self::new(value.request, value.resources, value.findings)
    }
}

impl<'de> Deserialize<'de> for HarnessObservation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        HarnessObservationWire::deserialize(deserializer)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObservationAdapterError {
    CapabilityUnavailable { capability: CapabilityId },
    NativeStateUnreadable {},
    NativeShapeUnsupported {},
    DeadlineExceeded {},
    ResourceLimitExceeded {},
    ExecutableChanged {},
}

impl fmt::Display for ObservationAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::CapabilityUnavailable { .. } => {
                "a required observation capability is unavailable"
            }
            Self::NativeStateUnreadable {} => "native state could not be read",
            Self::NativeShapeUnsupported {} => "the native state shape is unsupported",
            Self::DeadlineExceeded {} => "native observation exceeded its deadline",
            Self::ResourceLimitExceeded {} => "native observation exceeded a resource limit",
            Self::ExecutableChanged {} => "the observed harness executable changed",
        })
    }
}

impl std::error::Error for ObservationAdapterError {}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum HarnessObservationOutcome {
    Observed {
        observation: HarnessObservation,
    },
    Failed {
        request: ObservationRequest,
        error: ObservationAdapterError,
    },
}

impl HarnessObservationOutcome {
    pub const fn observed(observation: HarnessObservation) -> Self {
        Self::Observed { observation }
    }
    pub const fn failed(request: ObservationRequest, error: ObservationAdapterError) -> Self {
        Self::Failed { request, error }
    }
    pub fn target(&self) -> ObservationTarget {
        match self {
            Self::Observed { observation } => observation.target(),
            Self::Failed { request, .. } => request.target(),
        }
    }
    pub const fn request(&self) -> &ObservationRequest {
        match self {
            Self::Observed { observation } => observation.request(),
            Self::Failed { request, .. } => request,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(into = "ObservationBatchWire")]
pub struct ObservationBatch(BTreeMap<ObservationTarget, ObservationRequest>);

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ObservationBatchWire {
    requests: Vec<ObservationRequest>,
}

impl ObservationBatch {
    pub fn new(
        requests: impl IntoIterator<Item = ObservationRequest>,
    ) -> Result<Self, ObservationContractError> {
        let mut collected = BTreeMap::new();
        for request in requests {
            let target = request.target();
            if collected.insert(target.clone(), request).is_some() {
                return Err(ObservationContractError::DuplicateTarget { target });
            }
        }
        Ok(Self(collected))
    }
    pub fn iter(&self) -> impl ExactSizeIterator<Item = (&ObservationTarget, &ObservationRequest)> {
        self.0.iter()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn get(&self, target: &ObservationTarget) -> Option<&ObservationRequest> {
        self.0.get(target)
    }
}

impl From<ObservationBatch> for ObservationBatchWire {
    fn from(value: ObservationBatch) -> Self {
        Self {
            requests: value.0.into_values().collect(),
        }
    }
}

impl TryFrom<ObservationBatchWire> for ObservationBatch {
    type Error = ObservationContractError;
    fn try_from(value: ObservationBatchWire) -> Result<Self, Self::Error> {
        Self::new(value.requests)
    }
}

impl<'de> Deserialize<'de> for ObservationBatch {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        ObservationBatchWire::deserialize(deserializer)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(into = "ObservedEnvironmentWire")]
pub struct ObservedEnvironment {
    batch: ObservationBatch,
    outcomes: BTreeMap<ObservationTarget, HarnessObservationOutcome>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ObservedEnvironmentWire {
    batch: ObservationBatch,
    outcomes: Vec<HarnessObservationOutcome>,
}

impl ObservedEnvironment {
    pub fn new(
        batch: ObservationBatch,
        outcomes: impl IntoIterator<Item = HarnessObservationOutcome>,
    ) -> Result<Self, ObservationContractError> {
        let mut collected = BTreeMap::new();
        for outcome in outcomes {
            let target = outcome.target();
            let Some(request) = batch.get(&target) else {
                return Err(ObservationContractError::UnexpectedTarget { target });
            };
            if request != outcome.request() {
                return Err(ObservationContractError::OutcomeRequestMismatch { target });
            }
            if collected.insert(target.clone(), outcome).is_some() {
                return Err(ObservationContractError::DuplicateTarget { target });
            }
        }
        if let Some((target, _)) = batch
            .iter()
            .find(|(target, _)| !collected.contains_key(*target))
        {
            return Err(ObservationContractError::MissingTarget {
                target: target.clone(),
            });
        }
        Ok(Self {
            batch,
            outcomes: collected,
        })
    }
    pub fn get(&self, target: &ObservationTarget) -> Option<&HarnessObservationOutcome> {
        self.outcomes.get(target)
    }
    pub fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = (&ObservationTarget, &HarnessObservationOutcome)> {
        self.outcomes.iter()
    }
    pub fn is_empty(&self) -> bool {
        self.outcomes.is_empty()
    }
    pub const fn batch(&self) -> &ObservationBatch {
        &self.batch
    }
}

impl From<ObservedEnvironment> for ObservedEnvironmentWire {
    fn from(value: ObservedEnvironment) -> Self {
        Self {
            batch: value.batch,
            outcomes: value.outcomes.into_values().collect(),
        }
    }
}

impl TryFrom<ObservedEnvironmentWire> for ObservedEnvironment {
    type Error = ObservationContractError;
    fn try_from(value: ObservedEnvironmentWire) -> Result<Self, Self::Error> {
        Self::new(value.batch, value.outcomes)
    }
}

impl<'de> Deserialize<'de> for ObservedEnvironment {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        ObservedEnvironmentWire::deserialize(deserializer)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}

pub trait HarnessObservationAdapter {
    fn harness(&self) -> &HarnessId;
    fn observe(
        &self,
        request: &ObservationRequest,
    ) -> Result<HarnessObservation, ObservationAdapterError>;
}

pub trait ObservationCoordinator {
    fn observe(&self, batch: &ObservationBatch) -> ObservedEnvironment;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ObservationContractError {
    InstallationUnreachable {
        harness: HarnessId,
    },
    DuplicateObservation {
        key: ObservationKey,
    },
    ResourceContextMismatch {
        target: Box<ObservationTarget>,
        key: Box<ObservationKey>,
    },
    FindingContextMismatch {
        target: Box<ObservationTarget>,
        finding_target: Box<ObservationTarget>,
    },
    DuplicateTarget {
        target: ObservationTarget,
    },
    UnexpectedTarget {
        target: ObservationTarget,
    },
    MissingTarget {
        target: ObservationTarget,
    },
    OutcomeRequestMismatch {
        target: ObservationTarget,
    },
}

impl fmt::Display for ObservationContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InstallationUnreachable { harness } => {
                write!(
                    formatter,
                    "harness `{harness}` is not reachable for observation"
                )
            }
            Self::DuplicateObservation { key } => {
                write!(formatter, "duplicate observation key {key:?}")
            }
            Self::ResourceContextMismatch { target, key } => write!(
                formatter,
                "observation key {key:?} does not match target {target:?}"
            ),
            Self::FindingContextMismatch {
                target,
                finding_target,
            } => write!(
                formatter,
                "finding target {finding_target:?} does not match target {target:?}"
            ),
            Self::DuplicateTarget { target } => {
                write!(formatter, "duplicate observation target {target:?}")
            }
            Self::UnexpectedTarget { target } => {
                write!(formatter, "unexpected observation target {target:?}")
            }
            Self::MissingTarget { target } => {
                write!(formatter, "missing observation target {target:?}")
            }
            Self::OutcomeRequestMismatch { target } => {
                write!(
                    formatter,
                    "observation request mismatch for target {target:?}"
                )
            }
        }
    }
}

impl std::error::Error for ObservationContractError {}

#[cfg(test)]
mod tests;
