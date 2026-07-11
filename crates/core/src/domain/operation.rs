//! Validated reconciliation plan and apply-result contracts.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{
    ComponentId, EvidenceCode, EvidenceDetail, HarnessId, MaterialConsequence, OperationId,
    ResourceId,
};

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OperationSelector {
    Resource {
        resource_id: ResourceId,
    },
    Component {
        resource_id: ResourceId,
        component_id: ComponentId,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OperationDependency {
    operation_id: OperationId,
}

impl OperationDependency {
    pub fn new(operation_id: OperationId) -> Self {
        Self { operation_id }
    }

    pub fn operation_id(&self) -> &OperationId {
        &self.operation_id
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationClass {
    SafeNative,
    SafeFaithfulEquivalent,
    SafeMaterialization,
    Partial,
    Unsupported,
    Conflict,
    NoOp,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Reversibility {
    Reversible,
    Compensatable,
    Irreversible,
    NotApplicable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AttentionKind {
    AcknowledgmentRequired,
    Unsupported,
    Conflict,
    OperationFailed,
    DependencyBlocked,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttentionReason(AttentionReasonWire);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum AttentionReasonWire {
    AcknowledgmentRequired {
        selectors: BTreeSet<OperationSelector>,
        consequences: BTreeSet<MaterialConsequence>,
    },
    Unsupported {
        code: EvidenceCode,
        detail: EvidenceDetail,
    },
    Conflict {
        code: EvidenceCode,
        detail: EvidenceDetail,
    },
    OperationFailed {
        code: EvidenceCode,
        detail: EvidenceDetail,
    },
    DependencyBlocked {
        dependencies: BTreeSet<OperationId>,
    },
}

impl AttentionReason {
    pub fn acknowledgment_required(
        selectors: impl IntoIterator<Item = OperationSelector>,
        consequences: impl IntoIterator<Item = MaterialConsequence>,
    ) -> Result<Self, OperationContractError> {
        Self::from_wire(AttentionReasonWire::AcknowledgmentRequired {
            selectors: selectors.into_iter().collect(),
            consequences: consequences.into_iter().collect(),
        })
    }

    pub fn unsupported(code: EvidenceCode, detail: EvidenceDetail) -> Self {
        Self(AttentionReasonWire::Unsupported { code, detail })
    }

    pub fn conflict(code: EvidenceCode, detail: EvidenceDetail) -> Self {
        Self(AttentionReasonWire::Conflict { code, detail })
    }

    pub fn operation_failed(code: EvidenceCode, detail: EvidenceDetail) -> Self {
        Self(AttentionReasonWire::OperationFailed { code, detail })
    }

    pub fn dependency_blocked(
        dependencies: impl IntoIterator<Item = OperationId>,
    ) -> Result<Self, OperationContractError> {
        Self::from_wire(AttentionReasonWire::DependencyBlocked {
            dependencies: dependencies.into_iter().collect(),
        })
    }

    pub const fn kind(&self) -> AttentionKind {
        match &self.0 {
            AttentionReasonWire::AcknowledgmentRequired { .. } => {
                AttentionKind::AcknowledgmentRequired
            }
            AttentionReasonWire::Unsupported { .. } => AttentionKind::Unsupported,
            AttentionReasonWire::Conflict { .. } => AttentionKind::Conflict,
            AttentionReasonWire::OperationFailed { .. } => AttentionKind::OperationFailed,
            AttentionReasonWire::DependencyBlocked { .. } => AttentionKind::DependencyBlocked,
        }
    }

    pub fn selectors(&self) -> Option<&BTreeSet<OperationSelector>> {
        match &self.0 {
            AttentionReasonWire::AcknowledgmentRequired { selectors, .. } => Some(selectors),
            _ => None,
        }
    }

    pub fn consequences(&self) -> Option<&BTreeSet<MaterialConsequence>> {
        match &self.0 {
            AttentionReasonWire::AcknowledgmentRequired { consequences, .. } => Some(consequences),
            _ => None,
        }
    }

    pub fn code(&self) -> Option<&EvidenceCode> {
        match &self.0 {
            AttentionReasonWire::Unsupported { code, .. }
            | AttentionReasonWire::Conflict { code, .. }
            | AttentionReasonWire::OperationFailed { code, .. } => Some(code),
            _ => None,
        }
    }

    pub fn detail(&self) -> Option<&EvidenceDetail> {
        match &self.0 {
            AttentionReasonWire::Unsupported { detail, .. }
            | AttentionReasonWire::Conflict { detail, .. }
            | AttentionReasonWire::OperationFailed { detail, .. } => Some(detail),
            _ => None,
        }
    }

    pub fn dependencies(&self) -> Option<&BTreeSet<OperationId>> {
        match &self.0 {
            AttentionReasonWire::DependencyBlocked { dependencies } => Some(dependencies),
            _ => None,
        }
    }

    fn from_wire(wire: AttentionReasonWire) -> Result<Self, OperationContractError> {
        match &wire {
            AttentionReasonWire::AcknowledgmentRequired {
                selectors,
                consequences,
            } => validate_acknowledgment_parts(selectors, consequences)?,
            AttentionReasonWire::DependencyBlocked { dependencies } if dependencies.is_empty() => {
                return Err(OperationContractError::EmptyDependencyBlockers);
            }
            _ => {}
        }
        Ok(Self(wire))
    }
}

impl Serialize for AttentionReason {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for AttentionReason {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = AttentionReasonWire::deserialize(deserializer)?;
        Self::from_wire(wire).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AcknowledgmentRequirement(AcknowledgmentRequirementWire);

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum AcknowledgmentKind {
    NotRequired,
    Required,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct AcknowledgmentRequirementWire {
    kind: AcknowledgmentKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    selectors: Option<BTreeSet<OperationSelector>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    consequences: Option<BTreeSet<MaterialConsequence>>,
}

impl AcknowledgmentRequirement {
    pub const fn not_required() -> Self {
        Self(AcknowledgmentRequirementWire {
            kind: AcknowledgmentKind::NotRequired,
            selectors: None,
            consequences: None,
        })
    }

    pub fn required(
        selectors: impl IntoIterator<Item = OperationSelector>,
        consequences: impl IntoIterator<Item = MaterialConsequence>,
    ) -> Result<Self, OperationContractError> {
        let selectors = selectors.into_iter().collect();
        let consequences = consequences.into_iter().collect();
        validate_acknowledgment_parts(&selectors, &consequences)?;
        Ok(Self(AcknowledgmentRequirementWire {
            kind: AcknowledgmentKind::Required,
            selectors: Some(selectors),
            consequences: Some(consequences),
        }))
    }

    pub const fn is_required(&self) -> bool {
        matches!(self.0.kind, AcknowledgmentKind::Required)
    }

    pub fn selectors(&self) -> Option<&BTreeSet<OperationSelector>> {
        self.0.selectors.as_ref()
    }

    pub fn consequences(&self) -> Option<&BTreeSet<MaterialConsequence>> {
        self.0.consequences.as_ref()
    }
}

impl Serialize for AcknowledgmentRequirement {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for AcknowledgmentRequirement {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = AcknowledgmentRequirementWire::deserialize(deserializer)?;
        match (wire.kind, wire.selectors, wire.consequences) {
            (AcknowledgmentKind::NotRequired, None, None) => Ok(Self::not_required()),
            (AcknowledgmentKind::Required, Some(selectors), Some(consequences)) => {
                Self::required(selectors, consequences).map_err(serde::de::Error::custom)
            }
            _ => Err(serde::de::Error::custom(
                OperationContractError::InvalidAcknowledgmentShape,
            )),
        }
    }
}

fn validate_acknowledgment_parts(
    selectors: &BTreeSet<OperationSelector>,
    consequences: &BTreeSet<MaterialConsequence>,
) -> Result<(), OperationContractError> {
    if selectors.is_empty() {
        return Err(OperationContractError::EmptyAcknowledgmentSelectors);
    }
    if consequences.is_empty() {
        return Err(OperationContractError::EmptyAcknowledgmentConsequences);
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(into = "OperationWire")]
pub struct Operation {
    id: OperationId,
    target: HarnessId,
    selector: OperationSelector,
    class: OperationClass,
    reversibility: Reversibility,
    dependencies: BTreeSet<OperationDependency>,
    acknowledgment: AcknowledgmentRequirement,
    attention: Option<AttentionReason>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct OperationWire {
    id: OperationId,
    target: HarnessId,
    selector: OperationSelector,
    class: OperationClass,
    reversibility: Reversibility,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    dependencies: BTreeSet<OperationDependency>,
    acknowledgment: AcknowledgmentRequirement,
    #[serde(skip_serializing_if = "Option::is_none")]
    attention: Option<AttentionReason>,
}

impl Operation {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: OperationId,
        target: HarnessId,
        selector: OperationSelector,
        class: OperationClass,
        reversibility: Reversibility,
        dependencies: impl IntoIterator<Item = OperationDependency>,
        acknowledgment: AcknowledgmentRequirement,
        attention: Option<AttentionReason>,
    ) -> Result<Self, OperationContractError> {
        let dependencies = dependencies.into_iter().collect::<BTreeSet<_>>();
        let acknowledgment_required = acknowledgment.is_required();
        let attention_kind = attention.as_ref().map(AttentionReason::kind);

        let valid = match class {
            OperationClass::Partial => {
                acknowledgment_required
                    && attention_kind == Some(AttentionKind::AcknowledgmentRequired)
            }
            OperationClass::Unsupported => {
                !acknowledgment_required && attention_kind == Some(AttentionKind::Unsupported)
            }
            OperationClass::Conflict => {
                !acknowledgment_required && attention_kind == Some(AttentionKind::Conflict)
            }
            OperationClass::SafeNative
            | OperationClass::SafeFaithfulEquivalent
            | OperationClass::SafeMaterialization
            | OperationClass::NoOp => !acknowledgment_required && attention.is_none(),
        };
        if !valid {
            return Err(OperationContractError::InvalidOperationClassification {
                id,
                class,
                acknowledgment_required,
                attention: attention_kind,
            });
        }

        let executable = matches!(
            class,
            OperationClass::SafeNative
                | OperationClass::SafeFaithfulEquivalent
                | OperationClass::SafeMaterialization
                | OperationClass::Partial
        );
        if executable == (reversibility == Reversibility::NotApplicable) {
            return Err(OperationContractError::InvalidReversibility { id, class });
        }

        if let (Some(selectors), Some(attention)) = (acknowledgment.selectors(), attention.as_ref())
            && let AttentionReasonWire::AcknowledgmentRequired {
                selectors: attention_selectors,
                consequences: attention_consequences,
            } = &attention.0
            && (selectors != attention_selectors
                || acknowledgment.consequences() != Some(attention_consequences))
        {
            return Err(OperationContractError::AcknowledgmentAttentionMismatch { id });
        }

        if let Some(selectors) = acknowledgment.selectors()
            && let Some(outside) = selectors
                .iter()
                .find(|candidate| !selector_contains(&selector, candidate))
        {
            return Err(
                OperationContractError::AcknowledgmentSelectorOutsideOperation {
                    id,
                    selector: outside.clone(),
                },
            );
        }

        Ok(Self {
            id,
            target,
            selector,
            class,
            reversibility,
            dependencies,
            acknowledgment,
            attention,
        })
    }

    pub fn id(&self) -> &OperationId {
        &self.id
    }

    pub fn target(&self) -> &HarnessId {
        &self.target
    }

    pub const fn selector(&self) -> &OperationSelector {
        &self.selector
    }

    pub const fn class(&self) -> OperationClass {
        self.class
    }

    pub const fn reversibility(&self) -> Reversibility {
        self.reversibility
    }

    pub fn dependencies(&self) -> &BTreeSet<OperationDependency> {
        &self.dependencies
    }

    pub const fn acknowledgment(&self) -> &AcknowledgmentRequirement {
        &self.acknowledgment
    }

    pub fn attention(&self) -> Option<&AttentionReason> {
        self.attention.as_ref()
    }
}

fn selector_contains(operation: &OperationSelector, candidate: &OperationSelector) -> bool {
    match (operation, candidate) {
        (
            OperationSelector::Resource { resource_id },
            OperationSelector::Resource {
                resource_id: candidate_resource,
            }
            | OperationSelector::Component {
                resource_id: candidate_resource,
                ..
            },
        ) => resource_id == candidate_resource,
        (
            OperationSelector::Component {
                resource_id,
                component_id,
            },
            OperationSelector::Component {
                resource_id: candidate_resource,
                component_id: candidate_component,
            },
        ) => resource_id == candidate_resource && component_id == candidate_component,
        (OperationSelector::Component { .. }, OperationSelector::Resource { .. }) => false,
    }
}

impl From<Operation> for OperationWire {
    fn from(value: Operation) -> Self {
        Self {
            id: value.id,
            target: value.target,
            selector: value.selector,
            class: value.class,
            reversibility: value.reversibility,
            dependencies: value.dependencies,
            acknowledgment: value.acknowledgment,
            attention: value.attention,
        }
    }
}

impl TryFrom<OperationWire> for Operation {
    type Error = OperationContractError;

    fn try_from(value: OperationWire) -> Result<Self, Self::Error> {
        Self::new(
            value.id,
            value.target,
            value.selector,
            value.class,
            value.reversibility,
            value.dependencies,
            value.acknowledgment,
            value.attention,
        )
    }
}

impl<'de> Deserialize<'de> for Operation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        OperationWire::deserialize(deserializer)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(into = "PlanWire")]
pub struct Plan(BTreeMap<OperationId, Operation>);

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PlanWire {
    operations: Vec<Operation>,
}

impl Plan {
    pub fn new(
        operations: impl IntoIterator<Item = Operation>,
    ) -> Result<Self, OperationContractError> {
        let mut collected = BTreeMap::new();
        for operation in operations {
            let id = operation.id.clone();
            if collected.insert(id.clone(), operation).is_some() {
                return Err(OperationContractError::DuplicateOperation { id });
            }
        }
        validate_operation_graph(&collected)?;
        Ok(Self(collected))
    }

    pub fn get(&self, id: &OperationId) -> Option<&Operation> {
        self.0.get(id)
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = (&OperationId, &Operation)> {
        self.0.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl From<Plan> for PlanWire {
    fn from(value: Plan) -> Self {
        Self {
            operations: value.0.into_values().collect(),
        }
    }
}

impl<'de> Deserialize<'de> for Plan {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = PlanWire::deserialize(deserializer)?;
        Self::new(wire.operations).map_err(serde::de::Error::custom)
    }
}

fn validate_operation_graph(
    operations: &BTreeMap<OperationId, Operation>,
) -> Result<(), OperationContractError> {
    let mut remaining = BTreeMap::new();
    let mut dependents: BTreeMap<&OperationId, BTreeSet<&OperationId>> = BTreeMap::new();
    for (id, operation) in operations {
        for dependency in &operation.dependencies {
            let dependency_id = dependency.operation_id();
            if dependency_id == id {
                return Err(OperationContractError::SelfDependency { id: id.clone() });
            }
            if !operations.contains_key(dependency_id) {
                return Err(OperationContractError::UnknownDependency {
                    operation: id.clone(),
                    dependency: dependency_id.clone(),
                });
            }
            dependents.entry(dependency_id).or_default().insert(id);
        }
        remaining.insert(id, operation.dependencies.len());
    }

    let mut ready = remaining
        .iter()
        .filter_map(|(&id, &count)| (count == 0).then_some(id))
        .collect::<BTreeSet<_>>();
    let mut visited = 0;
    while let Some(operation) = ready.pop_first() {
        visited += 1;
        if let Some(children) = dependents.get(operation) {
            for child in children {
                let count = remaining
                    .get_mut(child)
                    .expect("validated dependent belongs to operation graph");
                *count -= 1;
                if *count == 0 {
                    ready.insert(child);
                }
            }
        }
    }
    if visited != operations.len() {
        return Err(OperationContractError::DependencyCycle {
            operations: remaining
                .into_iter()
                .filter(|(_, count)| *count > 0)
                .map(|(id, _)| id.clone())
                .collect(),
        });
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OperationOutcome {
    Applied,
    NoChange,
    Failed { reason: AttentionReason },
    Blocked { reason: AttentionReason },
    SkippedDependency { dependencies: BTreeSet<OperationId> },
    Pending,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum OperationStatus {
    Applied,
    NoChange,
    Failed,
    Blocked,
    SkippedDependency,
    Pending,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct OperationOutcomeWire {
    status: OperationStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<AttentionReason>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dependencies: Option<BTreeSet<OperationId>>,
}

impl OperationOutcome {
    fn from_wire(wire: OperationOutcomeWire) -> Result<Self, OperationContractError> {
        Ok(match (wire.status, wire.reason, wire.dependencies) {
            (OperationStatus::Applied, None, None) => Self::Applied,
            (OperationStatus::NoChange, None, None) => Self::NoChange,
            (OperationStatus::Failed, Some(reason), None) => Self::Failed { reason },
            (OperationStatus::Blocked, Some(reason), None) => Self::Blocked { reason },
            (OperationStatus::SkippedDependency, None, Some(dependencies)) => {
                if dependencies.is_empty() {
                    return Err(OperationContractError::EmptySkippedDependencies);
                }
                Self::SkippedDependency { dependencies }
            }
            (OperationStatus::Pending, None, None) => Self::Pending,
            _ => return Err(OperationContractError::InvalidOperationOutcomeShape),
        })
    }
}

impl From<OperationOutcome> for OperationOutcomeWire {
    fn from(value: OperationOutcome) -> Self {
        match value {
            OperationOutcome::Applied => Self {
                status: OperationStatus::Applied,
                reason: None,
                dependencies: None,
            },
            OperationOutcome::NoChange => Self {
                status: OperationStatus::NoChange,
                reason: None,
                dependencies: None,
            },
            OperationOutcome::Failed { reason } => Self {
                status: OperationStatus::Failed,
                reason: Some(reason),
                dependencies: None,
            },
            OperationOutcome::Blocked { reason } => Self {
                status: OperationStatus::Blocked,
                reason: Some(reason),
                dependencies: None,
            },
            OperationOutcome::SkippedDependency { dependencies } => Self {
                status: OperationStatus::SkippedDependency,
                reason: None,
                dependencies: Some(dependencies),
            },
            OperationOutcome::Pending => Self {
                status: OperationStatus::Pending,
                reason: None,
                dependencies: None,
            },
        }
    }
}

impl Serialize for OperationOutcome {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        OperationOutcomeWire::from(self.clone()).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for OperationOutcome {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        OperationOutcome::from_wire(OperationOutcomeWire::deserialize(deserializer)?)
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(into = "OperationResultWire")]
pub struct OperationResult {
    operation_id: OperationId,
    outcome: OperationOutcome,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct OperationResultWire {
    operation_id: OperationId,
    outcome: OperationOutcome,
}

impl OperationResult {
    pub fn new(
        operation_id: OperationId,
        outcome: OperationOutcome,
    ) -> Result<Self, OperationContractError> {
        match &outcome {
            OperationOutcome::Failed { reason }
                if reason.kind() != AttentionKind::OperationFailed =>
            {
                return Err(OperationContractError::InvalidFailedAttention {
                    id: operation_id,
                    attention: reason.kind(),
                });
            }
            OperationOutcome::Blocked { reason }
                if !matches!(
                    reason.kind(),
                    AttentionKind::AcknowledgmentRequired
                        | AttentionKind::Unsupported
                        | AttentionKind::Conflict
                        | AttentionKind::DependencyBlocked
                ) =>
            {
                return Err(OperationContractError::InvalidBlockedAttention {
                    id: operation_id,
                    attention: reason.kind(),
                });
            }
            _ => {}
        }
        if let OperationOutcome::SkippedDependency { dependencies } = &outcome {
            if dependencies.is_empty() {
                return Err(OperationContractError::EmptySkippedDependencies);
            }
            if dependencies.contains(&operation_id) {
                return Err(OperationContractError::SkippedBySelf { id: operation_id });
            }
        }
        Ok(Self {
            operation_id,
            outcome,
        })
    }

    pub fn operation_id(&self) -> &OperationId {
        &self.operation_id
    }

    pub const fn outcome(&self) -> &OperationOutcome {
        &self.outcome
    }
}

impl From<OperationResult> for OperationResultWire {
    fn from(value: OperationResult) -> Self {
        Self {
            operation_id: value.operation_id,
            outcome: value.outcome,
        }
    }
}

impl<'de> Deserialize<'de> for OperationResult {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = OperationResultWire::deserialize(deserializer)?;
        Self::new(wire.operation_id, wire.outcome).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplyOutcome {
    Succeeded,
    AttentionRequired,
    PartialFailure,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(into = "ApplyResultWire")]
pub struct ApplyResult {
    plan: Plan,
    outcome: ApplyOutcome,
    operations: BTreeMap<OperationId, OperationResult>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ApplyResultWire {
    plan: Plan,
    outcome: ApplyOutcome,
    operations: Vec<OperationResult>,
}

impl ApplyResult {
    pub fn new(
        plan: Plan,
        outcome: ApplyOutcome,
        operations: impl IntoIterator<Item = OperationResult>,
    ) -> Result<Self, OperationContractError> {
        let mut collected = BTreeMap::new();
        for result in operations {
            let id = result.operation_id.clone();
            if collected.insert(id.clone(), result).is_some() {
                return Err(OperationContractError::DuplicateOperationResult { id });
            }
        }

        for id in plan.0.keys() {
            if !collected.contains_key(id) {
                return Err(OperationContractError::MissingOperationResult { id: id.clone() });
            }
        }
        for id in collected.keys() {
            if !plan.0.contains_key(id) {
                return Err(OperationContractError::UnknownOperationResult { id: id.clone() });
            }
        }

        for result in collected.values() {
            if let OperationOutcome::SkippedDependency { dependencies } = &result.outcome {
                let operation = plan
                    .get(&result.operation_id)
                    .expect("result ids were validated against the plan");
                for dependency in dependencies {
                    if !operation
                        .dependencies()
                        .iter()
                        .any(|declared| declared.operation_id() == dependency)
                    {
                        return Err(OperationContractError::UndeclaredSkippedDependency {
                            operation: result.operation_id.clone(),
                            dependency: dependency.clone(),
                        });
                    }
                    let dependency_result = collected
                        .get(dependency)
                        .expect("declared plan dependency has an exact result");
                    if !matches!(
                        dependency_result.outcome,
                        OperationOutcome::Failed { .. }
                            | OperationOutcome::Blocked { .. }
                            | OperationOutcome::SkippedDependency { .. }
                    ) {
                        return Err(OperationContractError::InvalidSkippedDependencyOutcome {
                            operation: result.operation_id.clone(),
                            dependency: dependency.clone(),
                        });
                    }
                }
            }
        }

        let has_failure = collected
            .values()
            .any(|result| matches!(result.outcome, OperationOutcome::Failed { .. }));
        let has_attention = collected.values().any(|result| {
            matches!(
                result.outcome,
                OperationOutcome::Blocked { .. }
                    | OperationOutcome::SkippedDependency { .. }
                    | OperationOutcome::Pending
            )
        });
        let valid = match outcome {
            ApplyOutcome::Succeeded => !has_failure && !has_attention,
            ApplyOutcome::AttentionRequired => !has_failure && has_attention,
            ApplyOutcome::PartialFailure => has_failure,
        };
        if !valid {
            return Err(OperationContractError::InconsistentApplyOutcome {
                outcome,
                has_failure,
                has_attention,
            });
        }

        Ok(Self {
            plan,
            outcome,
            operations: collected,
        })
    }

    pub const fn plan(&self) -> &Plan {
        &self.plan
    }

    pub const fn outcome(&self) -> ApplyOutcome {
        self.outcome
    }

    pub fn operations(&self) -> &BTreeMap<OperationId, OperationResult> {
        &self.operations
    }
}

impl From<ApplyResult> for ApplyResultWire {
    fn from(value: ApplyResult) -> Self {
        Self {
            plan: value.plan,
            outcome: value.outcome,
            operations: value.operations.into_values().collect(),
        }
    }
}

impl<'de> Deserialize<'de> for ApplyResult {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ApplyResultWire::deserialize(deserializer)?;
        Self::new(wire.plan, wire.outcome, wire.operations).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OperationContractError {
    EmptyAcknowledgmentSelectors,
    EmptyAcknowledgmentConsequences,
    InvalidAcknowledgmentShape,
    EmptyDependencyBlockers,
    InvalidOperationClassification {
        id: OperationId,
        class: OperationClass,
        acknowledgment_required: bool,
        attention: Option<AttentionKind>,
    },
    InvalidReversibility {
        id: OperationId,
        class: OperationClass,
    },
    AcknowledgmentSelectorOutsideOperation {
        id: OperationId,
        selector: OperationSelector,
    },
    AcknowledgmentAttentionMismatch {
        id: OperationId,
    },
    DuplicateOperation {
        id: OperationId,
    },
    UnknownDependency {
        operation: OperationId,
        dependency: OperationId,
    },
    SelfDependency {
        id: OperationId,
    },
    DependencyCycle {
        operations: BTreeSet<OperationId>,
    },
    EmptySkippedDependencies,
    InvalidOperationOutcomeShape,
    SkippedBySelf {
        id: OperationId,
    },
    InvalidFailedAttention {
        id: OperationId,
        attention: AttentionKind,
    },
    InvalidBlockedAttention {
        id: OperationId,
        attention: AttentionKind,
    },
    DuplicateOperationResult {
        id: OperationId,
    },
    MissingOperationResult {
        id: OperationId,
    },
    UnknownOperationResult {
        id: OperationId,
    },
    UndeclaredSkippedDependency {
        operation: OperationId,
        dependency: OperationId,
    },
    InvalidSkippedDependencyOutcome {
        operation: OperationId,
        dependency: OperationId,
    },
    InconsistentApplyOutcome {
        outcome: ApplyOutcome,
        has_failure: bool,
        has_attention: bool,
    },
}

impl fmt::Display for OperationContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyAcknowledgmentSelectors => {
                write!(
                    formatter,
                    "acknowledgment requires at least one exact selector"
                )
            }
            Self::EmptyAcknowledgmentConsequences => {
                write!(
                    formatter,
                    "acknowledgment requires at least one material consequence"
                )
            }
            Self::InvalidAcknowledgmentShape => write!(
                formatter,
                "acknowledgment fields do not match its declared kind"
            ),
            Self::EmptyDependencyBlockers => {
                write!(
                    formatter,
                    "dependency-blocked attention requires a dependency"
                )
            }
            Self::InvalidOperationClassification {
                id,
                class,
                acknowledgment_required,
                attention,
            } => write!(
                formatter,
                "operation `{id}` has invalid {class:?} classification (acknowledgment_required={acknowledgment_required}, attention={attention:?})"
            ),
            Self::InvalidReversibility { id, class } => write!(
                formatter,
                "operation `{id}` has invalid reversibility for {class:?}"
            ),
            Self::AcknowledgmentSelectorOutsideOperation { id, selector } => write!(
                formatter,
                "operation `{id}` acknowledgment selector {selector:?} is outside its scope"
            ),
            Self::AcknowledgmentAttentionMismatch { id } => write!(
                formatter,
                "operation `{id}` acknowledgment and attention details must match exactly"
            ),
            Self::DuplicateOperation { id } => write!(formatter, "duplicate operation `{id}`"),
            Self::UnknownDependency {
                operation,
                dependency,
            } => write!(
                formatter,
                "operation `{operation}` depends on unknown operation `{dependency}`"
            ),
            Self::SelfDependency { id } => write!(formatter, "operation `{id}` depends on itself"),
            Self::DependencyCycle { operations } => write!(
                formatter,
                "operation dependency cycle includes {}",
                operations
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::EmptySkippedDependencies => {
                write!(formatter, "dependent skip requires at least one dependency")
            }
            Self::InvalidOperationOutcomeShape => {
                write!(
                    formatter,
                    "operation outcome fields do not match its status"
                )
            }
            Self::SkippedBySelf { id } => {
                write!(formatter, "operation `{id}` cannot be skipped by itself")
            }
            Self::InvalidFailedAttention { id, attention } => write!(
                formatter,
                "failed operation `{id}` requires operation_failed attention, got {attention:?}"
            ),
            Self::InvalidBlockedAttention { id, attention } => write!(
                formatter,
                "blocked operation `{id}` cannot use {attention:?} attention"
            ),
            Self::DuplicateOperationResult { id } => {
                write!(formatter, "duplicate result for operation `{id}`")
            }
            Self::MissingOperationResult { id } => {
                write!(formatter, "plan operation `{id}` has no result")
            }
            Self::UnknownOperationResult { id } => {
                write!(formatter, "result references unknown plan operation `{id}`")
            }
            Self::UndeclaredSkippedDependency {
                operation,
                dependency,
            } => write!(
                formatter,
                "operation `{operation}` was skipped by undeclared dependency `{dependency}`"
            ),
            Self::InvalidSkippedDependencyOutcome {
                operation,
                dependency,
            } => write!(
                formatter,
                "operation `{operation}` was skipped by operation `{dependency}` that did not fail or block"
            ),
            Self::InconsistentApplyOutcome {
                outcome,
                has_failure,
                has_attention,
            } => write!(
                formatter,
                "apply outcome {outcome:?} is inconsistent with operation results (failure={has_failure}, attention={has_attention})"
            ),
        }
    }
}

impl std::error::Error for OperationContractError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ConsequenceCode, ConsequenceSummary};

    fn operation_id(value: &str) -> OperationId {
        OperationId::new(value).unwrap()
    }

    fn resource_selector(value: &str) -> OperationSelector {
        OperationSelector::Resource {
            resource_id: ResourceId::new(value).unwrap(),
        }
    }

    fn component_selector(resource: &str, component: &str) -> OperationSelector {
        OperationSelector::Component {
            resource_id: ResourceId::new(resource).unwrap(),
            component_id: ComponentId::new(component).unwrap(),
        }
    }

    fn consequence() -> MaterialConsequence {
        MaterialConsequence::new(
            ConsequenceCode::new("component.omitted").unwrap(),
            [ComponentId::new("hook:format").unwrap()],
            ConsequenceSummary::new("The formatting hook will not be installed").unwrap(),
        )
    }

    fn partial_operation(id: &str, dependencies: &[&str]) -> Operation {
        let selectors = [component_selector("plugin:tools", "hook:format")];
        let consequences = [consequence()];
        Operation::new(
            operation_id(id),
            HarnessId::new("codex").unwrap(),
            resource_selector("plugin:tools"),
            OperationClass::Partial,
            Reversibility::Reversible,
            dependencies
                .iter()
                .map(|id| OperationDependency::new(operation_id(id))),
            AcknowledgmentRequirement::required(selectors.clone(), consequences.clone()).unwrap(),
            Some(AttentionReason::acknowledgment_required(selectors, consequences).unwrap()),
        )
        .unwrap()
    }

    fn safe_operation(id: &str, dependencies: &[&str]) -> Operation {
        Operation::new(
            operation_id(id),
            HarnessId::new("claude").unwrap(),
            resource_selector("plugin:tools"),
            OperationClass::SafeNative,
            Reversibility::Reversible,
            dependencies
                .iter()
                .map(|id| OperationDependency::new(operation_id(id))),
            AcknowledgmentRequirement::not_required(),
            None,
        )
        .unwrap()
    }

    #[test]
    fn plan_rejects_duplicate_unknown_self_and_cyclic_dependencies() {
        assert_eq!(
            Plan::new([safe_operation("one", &[]), safe_operation("one", &[])]).unwrap_err(),
            OperationContractError::DuplicateOperation {
                id: operation_id("one")
            }
        );
        assert_eq!(
            Plan::new([safe_operation("one", &["missing"])]).unwrap_err(),
            OperationContractError::UnknownDependency {
                operation: operation_id("one"),
                dependency: operation_id("missing")
            }
        );
        assert_eq!(
            Plan::new([safe_operation("one", &["one"])]).unwrap_err(),
            OperationContractError::SelfDependency {
                id: operation_id("one")
            }
        );
        assert!(matches!(
            Plan::new([
                safe_operation("one", &["three"]),
                safe_operation("two", &["one"]),
                safe_operation("three", &["two"]),
            ]),
            Err(OperationContractError::DependencyCycle { .. })
        ));
    }

    #[test]
    fn partial_operations_require_exact_selectors_consequences_and_matching_attention() {
        assert_eq!(
            AcknowledgmentRequirement::required([], [consequence()]).unwrap_err(),
            OperationContractError::EmptyAcknowledgmentSelectors
        );
        assert_eq!(
            AcknowledgmentRequirement::required([resource_selector("plugin:tools")], [])
                .unwrap_err(),
            OperationContractError::EmptyAcknowledgmentConsequences
        );

        let missing_ack = Operation::new(
            operation_id("partial"),
            HarnessId::new("codex").unwrap(),
            resource_selector("plugin:tools"),
            OperationClass::Partial,
            Reversibility::Irreversible,
            [],
            AcknowledgmentRequirement::not_required(),
            None,
        )
        .unwrap_err();
        assert!(matches!(
            missing_ack,
            OperationContractError::InvalidOperationClassification { .. }
        ));

        let different_selectors = Operation::new(
            operation_id("partial"),
            HarnessId::new("codex").unwrap(),
            resource_selector("plugin:tools"),
            OperationClass::Partial,
            Reversibility::Irreversible,
            [],
            AcknowledgmentRequirement::required(
                [resource_selector("plugin:tools")],
                [consequence()],
            )
            .unwrap(),
            Some(
                AttentionReason::acknowledgment_required(
                    [component_selector("plugin:tools", "hook:format")],
                    [consequence()],
                )
                .unwrap(),
            ),
        )
        .unwrap_err();
        assert_eq!(
            different_selectors,
            OperationContractError::AcknowledgmentAttentionMismatch {
                id: operation_id("partial")
            }
        );

        let outside = Operation::new(
            operation_id("component-partial"),
            HarnessId::new("codex").unwrap(),
            component_selector("plugin:tools", "hook:format"),
            OperationClass::Partial,
            Reversibility::Irreversible,
            [],
            AcknowledgmentRequirement::required(
                [resource_selector("plugin:tools")],
                [consequence()],
            )
            .unwrap(),
            Some(
                AttentionReason::acknowledgment_required(
                    [resource_selector("plugin:tools")],
                    [consequence()],
                )
                .unwrap(),
            ),
        )
        .unwrap_err();
        assert!(matches!(
            outside,
            OperationContractError::AcknowledgmentSelectorOutsideOperation { .. }
        ));
    }

    #[test]
    fn operation_class_attention_and_reversibility_combinations_are_validated() {
        let safe_with_attention = Operation::new(
            operation_id("safe"),
            HarnessId::new("codex").unwrap(),
            resource_selector("skill:portable"),
            OperationClass::SafeFaithfulEquivalent,
            Reversibility::Reversible,
            [],
            AcknowledgmentRequirement::not_required(),
            Some(AttentionReason::unsupported(
                EvidenceCode::new("native.unsupported").unwrap(),
                EvidenceDetail::new("The native operation is unavailable").unwrap(),
            )),
        )
        .unwrap_err();
        assert!(matches!(
            safe_with_attention,
            OperationContractError::InvalidOperationClassification { .. }
        ));

        let no_op = Operation::new(
            operation_id("noop"),
            HarnessId::new("codex").unwrap(),
            resource_selector("skill:portable"),
            OperationClass::NoOp,
            Reversibility::Reversible,
            [],
            AcknowledgmentRequirement::not_required(),
            None,
        )
        .unwrap_err();
        assert_eq!(
            no_op,
            OperationContractError::InvalidReversibility {
                id: operation_id("noop"),
                class: OperationClass::NoOp,
            }
        );

        let unsupported = Operation::new(
            operation_id("unsupported"),
            HarnessId::new("codex").unwrap(),
            resource_selector("skill:portable"),
            OperationClass::Unsupported,
            Reversibility::Reversible,
            [],
            AcknowledgmentRequirement::not_required(),
            Some(AttentionReason::unsupported(
                EvidenceCode::new("native.unsupported").unwrap(),
                EvidenceDetail::new("The native operation is unavailable").unwrap(),
            )),
        )
        .unwrap_err();
        assert_eq!(
            unsupported,
            OperationContractError::InvalidReversibility {
                id: operation_id("unsupported"),
                class: OperationClass::Unsupported,
            }
        );
    }

    #[test]
    fn operation_and_attention_payloads_are_readable_without_serialization() {
        let operation = partial_operation("materialize", &[]);
        assert_eq!(operation.target(), &HarnessId::new("codex").unwrap());
        assert_eq!(operation.selector(), &resource_selector("plugin:tools"));
        assert_eq!(operation.class(), OperationClass::Partial);
        assert_eq!(operation.reversibility(), Reversibility::Reversible);
        assert!(operation.dependencies().is_empty());
        assert!(operation.acknowledgment().is_required());

        let attention = operation.attention().unwrap();
        assert_eq!(attention.kind(), AttentionKind::AcknowledgmentRequired);
        assert_eq!(
            attention.selectors(),
            operation.acknowledgment().selectors()
        );
        assert_eq!(
            attention.consequences(),
            operation.acknowledgment().consequences()
        );
        assert!(attention.code().is_none());
        assert!(attention.detail().is_none());
        assert!(attention.dependencies().is_none());

        let unsupported = AttentionReason::unsupported(
            EvidenceCode::new("native.unsupported").unwrap(),
            EvidenceDetail::new("No native mutation exists").unwrap(),
        );
        assert_eq!(unsupported.code().unwrap().as_str(), "native.unsupported");
        assert_eq!(
            unsupported.detail().unwrap().as_str(),
            "No native mutation exists"
        );
    }

    #[test]
    fn constructor_and_deserialization_enforce_plan_invariants() {
        let invalid = r#"{"operations":[{"id":"one","target":"codex","selector":{"kind":"resource","resource_id":"skill:one"},"class":"safe_native","reversibility":"reversible","dependencies":[{"operation_id":"missing"}],"acknowledgment":{"kind":"not_required"}}]}"#;
        let error = serde_json::from_str::<Plan>(invalid).unwrap_err();
        assert!(error.to_string().contains("unknown operation `missing`"));

        let empty_ack = r#"{"kind":"required","selectors":[],"consequences":[]}"#;
        assert!(serde_json::from_str::<AcknowledgmentRequirement>(empty_ack).is_err());

        let unknown_field = r#"{"kind":"not_required","yes":true}"#;
        assert!(serde_json::from_str::<AcknowledgmentRequirement>(unknown_field).is_err());

        let invalid_reversibility = r#"{"id":"unsupported","target":"codex","selector":{"kind":"resource","resource_id":"skill:one"},"class":"unsupported","reversibility":"reversible","acknowledgment":{"kind":"not_required"},"attention":{"kind":"unsupported","code":"native.unsupported","detail":"No native mutation exists"}}"#;
        assert!(serde_json::from_str::<Operation>(invalid_reversibility).is_err());

        let outside_selector = r#"{"id":"partial","target":"codex","selector":{"kind":"component","resource_id":"plugin:tools","component_id":"hook:format"},"class":"partial","reversibility":"irreversible","acknowledgment":{"kind":"required","selectors":[{"kind":"resource","resource_id":"plugin:tools"}],"consequences":[{"code":"component.omitted","affected_components":["hook:format"],"summary":"The formatting hook will not be installed"}]},"attention":{"kind":"acknowledgment_required","selectors":[{"kind":"resource","resource_id":"plugin:tools"}],"consequences":[{"code":"component.omitted","affected_components":["hook:format"],"summary":"The formatting hook will not be installed"}]}}"#;
        assert!(serde_json::from_str::<Operation>(outside_selector).is_err());
    }

    #[test]
    fn operation_results_validate_attention_kinds_at_both_boundaries() {
        let unsupported = AttentionReason::unsupported(
            EvidenceCode::new("native.unsupported").unwrap(),
            EvidenceDetail::new("No native mutation exists").unwrap(),
        );
        assert!(matches!(
            OperationResult::new(
                operation_id("one"),
                OperationOutcome::Failed {
                    reason: unsupported
                }
            ),
            Err(OperationContractError::InvalidFailedAttention { .. })
        ));

        let failed = AttentionReason::operation_failed(
            EvidenceCode::new("native.command.failed").unwrap(),
            EvidenceDetail::new("Native command failed").unwrap(),
        );
        assert!(matches!(
            OperationResult::new(
                operation_id("one"),
                OperationOutcome::Blocked { reason: failed }
            ),
            Err(OperationContractError::InvalidBlockedAttention { .. })
        ));

        let failed_with_unsupported = r#"{"operation_id":"one","outcome":{"status":"failed","reason":{"kind":"unsupported","code":"native.unsupported","detail":"No native mutation exists"}}}"#;
        assert!(serde_json::from_str::<OperationResult>(failed_with_unsupported).is_err());
        let blocked_with_failure = r#"{"operation_id":"one","outcome":{"status":"blocked","reason":{"kind":"operation_failed","code":"native.command.failed","detail":"Native command failed"}}}"#;
        assert!(serde_json::from_str::<OperationResult>(blocked_with_failure).is_err());
    }

    #[test]
    fn representative_plan_serializes_deterministically_and_round_trips() {
        let plan = Plan::new([
            partial_operation("materialize", &["register"]),
            safe_operation("register", &[]),
        ])
        .unwrap();
        let json = serde_json::to_string(&plan).unwrap();

        assert_eq!(serde_json::from_str::<Plan>(&json).unwrap(), plan);
        assert!(
            json.find(r#""id":"materialize""#).unwrap() < json.find(r#""id":"register""#).unwrap()
        );
        assert!(json.contains(r#""class":"partial""#));
        assert!(json.contains(r#""kind":"component""#));
        assert!(!json.contains("yes"));

        assert_eq!(
            serde_json::to_string(&OperationClass::SafeNative).unwrap(),
            r#""safe_native""#
        );
        assert_eq!(
            serde_json::to_string(&component_selector("plugin:tools", "hook:format")).unwrap(),
            r#"{"kind":"component","resource_id":"plugin:tools","component_id":"hook:format"}"#
        );
    }

    #[test]
    fn dependent_skips_are_distinct_and_final_success_rejects_unfinished_work() {
        let plan = Plan::new([
            safe_operation("register", &[]),
            safe_operation("materialize", &["register"]),
        ])
        .unwrap();
        let failure = OperationResult::new(
            operation_id("register"),
            OperationOutcome::Failed {
                reason: AttentionReason::operation_failed(
                    EvidenceCode::new("native.command.failed").unwrap(),
                    EvidenceDetail::new("Native command exited with status 1").unwrap(),
                ),
            },
        )
        .unwrap();
        let skipped = OperationResult::new(
            operation_id("materialize"),
            OperationOutcome::SkippedDependency {
                dependencies: [operation_id("register")].into_iter().collect(),
            },
        )
        .unwrap();

        assert!(matches!(
            skipped.outcome(),
            OperationOutcome::SkippedDependency { .. }
        ));
        assert!(matches!(
            ApplyResult::new(
                plan.clone(),
                ApplyOutcome::Succeeded,
                [failure.clone(), skipped.clone()]
            ),
            Err(OperationContractError::InconsistentApplyOutcome { .. })
        ));
        assert!(matches!(
            ApplyResult::new(plan, ApplyOutcome::AttentionRequired, [failure, skipped]),
            Err(OperationContractError::InconsistentApplyOutcome { .. })
        ));
    }

    #[test]
    fn partial_apply_result_is_deterministic_and_round_trips() {
        let plan = Plan::new([
            safe_operation("z-pending", &[]),
            safe_operation("a-applied", &[]),
            safe_operation("m-blocked", &[]),
        ])
        .unwrap();
        let result = ApplyResult::new(
            plan,
            ApplyOutcome::AttentionRequired,
            [
                OperationResult::new(operation_id("z-pending"), OperationOutcome::Pending).unwrap(),
                OperationResult::new(operation_id("a-applied"), OperationOutcome::Applied).unwrap(),
                OperationResult::new(
                    operation_id("m-blocked"),
                    OperationOutcome::Blocked {
                        reason: AttentionReason::unsupported(
                            EvidenceCode::new("native.operation.unsupported").unwrap(),
                            EvidenceDetail::new("No deterministic native mutation is available")
                                .unwrap(),
                        ),
                    },
                )
                .unwrap(),
            ],
        )
        .unwrap();
        let json = serde_json::to_string(&result).unwrap();

        assert_eq!(serde_json::from_str::<ApplyResult>(&json).unwrap(), result);
        assert!(json.find("a-applied").unwrap() < json.find("m-blocked").unwrap());
        assert!(json.find("m-blocked").unwrap() < json.find("z-pending").unwrap());
        assert!(json.contains(r#""status":"blocked""#));
        assert!(json.contains(r#""status":"pending""#));
    }

    #[test]
    fn result_deserialization_rejects_invalid_summaries_and_skips() {
        let false_success = r#"{"plan":{"operations":[{"id":"one","target":"codex","selector":{"kind":"resource","resource_id":"skill:one"},"class":"safe_native","reversibility":"reversible","acknowledgment":{"kind":"not_required"}}]},"outcome":"succeeded","operations":[{"operation_id":"one","outcome":{"status":"pending"}}]}"#;
        assert!(serde_json::from_str::<ApplyResult>(false_success).is_err());

        let empty_skip =
            r#"{"operation_id":"one","outcome":{"status":"skipped_dependency","dependencies":[]}}"#;
        assert!(serde_json::from_str::<OperationResult>(empty_skip).is_err());

        let self_skip = r#"{"operation_id":"one","outcome":{"status":"skipped_dependency","dependencies":["one"]}}"#;
        assert!(serde_json::from_str::<OperationResult>(self_skip).is_err());

        let unknown =
            r#"{"plan":{"operations":[]},"outcome":"succeeded","operations":[],"assume_ok":true}"#;
        assert!(serde_json::from_str::<ApplyResult>(unknown).is_err());

        let unknown_dependency = OperationResult::new(
            operation_id("two"),
            OperationOutcome::SkippedDependency {
                dependencies: [operation_id("missing")].into_iter().collect(),
            },
        )
        .unwrap();
        assert_eq!(
            ApplyResult::new(
                Plan::new([safe_operation("two", &[])]).unwrap(),
                ApplyOutcome::AttentionRequired,
                [unknown_dependency]
            )
            .unwrap_err(),
            OperationContractError::UndeclaredSkippedDependency {
                operation: operation_id("two"),
                dependency: operation_id("missing")
            }
        );

        assert_eq!(
            ApplyResult::new(
                Plan::new([safe_operation("one", &[])]).unwrap(),
                ApplyOutcome::Succeeded,
                [],
            )
            .unwrap_err(),
            OperationContractError::MissingOperationResult {
                id: operation_id("one")
            }
        );
        assert_eq!(
            ApplyResult::new(
                Plan::new([]).unwrap(),
                ApplyOutcome::Succeeded,
                [OperationResult::new(operation_id("extra"), OperationOutcome::Applied).unwrap()],
            )
            .unwrap_err(),
            OperationContractError::UnknownOperationResult {
                id: operation_id("extra")
            }
        );

        let missing_persisted = r#"{"plan":{"operations":[{"id":"one","target":"codex","selector":{"kind":"resource","resource_id":"skill:one"},"class":"safe_native","reversibility":"reversible","acknowledgment":{"kind":"not_required"}}]},"outcome":"succeeded","operations":[]}"#;
        assert!(serde_json::from_str::<ApplyResult>(missing_persisted).is_err());
    }
}
