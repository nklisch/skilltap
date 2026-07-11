//! Validated reconciliation plan and apply-result contracts.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{
    AbsolutePath, CompatibilityResult, ComponentId, ConsequenceCode, EvidenceCode, EvidenceDetail,
    HarnessId, MaterialConsequence, NativeId, OperationId, Provenance, ResourceId, Scope,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationAction {
    MarketplaceRegister,
    MarketplaceRemove,
    MarketplaceUpdate,
    PluginInstall,
    PluginRemove,
    PluginEnable,
    PluginDisable,
    PluginUpdate,
    SkillInstall,
    SkillRemove,
    SkillUpdate,
    Materialize,
    InstructionSetup,
    InstructionRepair,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OperationReason {
    code: EvidenceCode,
    detail: EvidenceDetail,
}

impl OperationReason {
    pub fn new(code: EvidenceCode, detail: EvidenceDetail) -> Self {
        Self { code, detail }
    }

    pub fn code(&self) -> &EvidenceCode {
        &self.code
    }

    pub fn detail(&self) -> &EvidenceDetail {
        &self.detail
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CommandArgument {
    Literal { value: NativeId },
    Redacted {},
}

impl CommandArgument {
    pub fn literal(value: NativeId) -> Self {
        Self::Literal { value }
    }

    pub const fn redacted() -> Self {
        Self::Redacted {}
    }

    pub fn literal_value(&self) -> Option<&NativeId> {
        match self {
            Self::Literal { value } => Some(value),
            Self::Redacted {} => None,
        }
    }

    pub const fn is_redacted(&self) -> bool {
        matches!(self, Self::Redacted {})
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AffectedSurface {
    File {
        path: AbsolutePath,
    },
    NativeCommand {
        target: HarnessId,
        executable: NativeId,
        arguments: Vec<CommandArgument>,
    },
}

impl AffectedSurface {
    pub const fn file(path: AbsolutePath) -> Self {
        Self::File { path }
    }

    pub fn native_command(
        target: HarnessId,
        executable: NativeId,
        arguments: impl IntoIterator<Item = CommandArgument>,
    ) -> Self {
        Self::NativeCommand {
            target,
            executable,
            arguments: arguments.into_iter().collect(),
        }
    }

    pub const fn path(&self) -> Option<&AbsolutePath> {
        match self {
            Self::File { path } => Some(path),
            Self::NativeCommand { .. } => None,
        }
    }

    pub const fn target(&self) -> Option<&HarnessId> {
        match self {
            Self::File { .. } => None,
            Self::NativeCommand { target, .. } => Some(target),
        }
    }

    pub const fn executable(&self) -> Option<&NativeId> {
        match self {
            Self::File { .. } => None,
            Self::NativeCommand { executable, .. } => Some(executable),
        }
    }

    pub fn arguments(&self) -> Option<&[CommandArgument]> {
        match self {
            Self::File { .. } => None,
            Self::NativeCommand { arguments, .. } => Some(arguments),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OperationSemantics {
    action: OperationAction,
    scope: Scope,
    reason: OperationReason,
    compatibility: CompatibilityResult,
    provenance: Provenance,
    affected_surfaces: BTreeSet<AffectedSurface>,
}

impl OperationSemantics {
    pub fn new(
        action: OperationAction,
        scope: Scope,
        reason: OperationReason,
        compatibility: CompatibilityResult,
        provenance: Provenance,
        affected_surfaces: impl IntoIterator<Item = AffectedSurface>,
    ) -> Self {
        Self {
            action,
            scope,
            reason,
            compatibility,
            provenance,
            affected_surfaces: affected_surfaces.into_iter().collect(),
        }
    }

    pub const fn action(&self) -> OperationAction {
        self.action
    }

    pub const fn scope(&self) -> &Scope {
        &self.scope
    }

    pub const fn reason(&self) -> &OperationReason {
        &self.reason
    }

    pub const fn compatibility(&self) -> &CompatibilityResult {
        &self.compatibility
    }

    pub const fn provenance(&self) -> Provenance {
        self.provenance
    }

    pub fn affected_surfaces(&self) -> &BTreeSet<AffectedSurface> {
        &self.affected_surfaces
    }
}

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
    semantics: OperationSemantics,
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
    action: OperationAction,
    scope: Scope,
    reason: OperationReason,
    compatibility: CompatibilityResult,
    provenance: Provenance,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    affected_surfaces: BTreeSet<AffectedSurface>,
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
        semantics: OperationSemantics,
        class: OperationClass,
        reversibility: Reversibility,
        dependencies: impl IntoIterator<Item = OperationDependency>,
        acknowledgment: AcknowledgmentRequirement,
        attention: Option<AttentionReason>,
    ) -> Result<Self, OperationContractError> {
        let dependencies = dependencies.into_iter().collect::<BTreeSet<_>>();
        if semantics.compatibility.target() != &target {
            return Err(OperationContractError::CompatibilityTargetMismatch {
                id,
                target,
                compatibility_target: semantics.compatibility.target().clone(),
            });
        }
        if let Some(surface_target) = semantics.affected_surfaces.iter().find_map(|surface| {
            if let AffectedSurface::NativeCommand {
                target: surface_target,
                ..
            } = surface
                && surface_target != &target
            {
                return Some(surface_target.clone());
            }
            None
        }) {
            return Err(OperationContractError::AffectedSurfaceTargetMismatch {
                id,
                target,
                surface_target,
            });
        }
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
        if let (Some(selectors), Some(consequences)) =
            (acknowledgment.selectors(), acknowledgment.consequences())
        {
            validate_consequence_coverage(&id, selectors, consequences)?;
        }

        Ok(Self {
            id,
            target,
            selector,
            semantics,
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

    pub const fn semantics(&self) -> &OperationSemantics {
        &self.semantics
    }

    pub const fn action(&self) -> OperationAction {
        self.semantics.action()
    }

    pub const fn scope(&self) -> &Scope {
        self.semantics.scope()
    }

    pub const fn reason(&self) -> &OperationReason {
        self.semantics.reason()
    }

    pub const fn compatibility(&self) -> &CompatibilityResult {
        self.semantics.compatibility()
    }

    pub const fn provenance(&self) -> Provenance {
        self.semantics.provenance()
    }

    pub fn affected_surfaces(&self) -> &BTreeSet<AffectedSurface> {
        self.semantics.affected_surfaces()
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

fn validate_consequence_coverage(
    operation: &OperationId,
    selectors: &BTreeSet<OperationSelector>,
    consequences: &BTreeSet<MaterialConsequence>,
) -> Result<(), OperationContractError> {
    for consequence in consequences {
        if consequence.affected_components.is_empty() {
            if !selectors
                .iter()
                .any(|selector| matches!(selector, OperationSelector::Resource { .. }))
            {
                return Err(OperationContractError::UncoveredResourceConsequence {
                    operation: operation.clone(),
                    code: consequence.code.clone(),
                });
            }
            continue;
        }
        for component in &consequence.affected_components {
            let covered = selectors.iter().any(|selector| match selector {
                OperationSelector::Resource { .. } => true,
                OperationSelector::Component { component_id, .. } => component_id == component,
            });
            if !covered {
                return Err(OperationContractError::UncoveredComponentConsequence {
                    operation: operation.clone(),
                    code: consequence.code.clone(),
                    component: component.clone(),
                });
            }
        }
    }
    Ok(())
}

impl From<Operation> for OperationWire {
    fn from(value: Operation) -> Self {
        Self {
            id: value.id,
            target: value.target,
            selector: value.selector,
            action: value.semantics.action,
            scope: value.semantics.scope,
            reason: value.semantics.reason,
            compatibility: value.semantics.compatibility,
            provenance: value.semantics.provenance,
            affected_surfaces: value.semantics.affected_surfaces,
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
            OperationSemantics::new(
                value.action,
                value.scope,
                value.reason,
                value.compatibility,
                value.provenance,
                value.affected_surfaces,
            ),
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
        let unresolved = remaining
            .into_iter()
            .filter(|(_, count)| *count > 0)
            .map(|(id, _)| id.clone())
            .collect::<BTreeSet<_>>();
        return Err(OperationContractError::DependencyCycle {
            operations: unresolved
                .iter()
                .filter(|id| {
                    operation_reaches(id, id, operations, &unresolved, &mut BTreeSet::new())
                })
                .cloned()
                .collect(),
        });
    }
    Ok(())
}

fn operation_reaches(
    current: &OperationId,
    target: &OperationId,
    operations: &BTreeMap<OperationId, Operation>,
    allowed: &BTreeSet<OperationId>,
    visited: &mut BTreeSet<OperationId>,
) -> bool {
    for dependency in &operations
        .get(current)
        .expect("cycle search only visits known operations")
        .dependencies
    {
        let dependency = dependency.operation_id();
        if dependency == target {
            return true;
        }
        if allowed.contains(dependency)
            && visited.insert(dependency.clone())
            && operation_reaches(dependency, target, operations, allowed, visited)
        {
            return true;
        }
    }
    false
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
            let operation = plan
                .get(&result.operation_id)
                .expect("result ids were validated against the plan");
            let class_outcome_valid = match (operation.class(), &result.outcome) {
                (OperationClass::Unsupported, OperationOutcome::Blocked { reason }) => {
                    reason.kind() == AttentionKind::Unsupported
                }
                (OperationClass::Conflict, OperationOutcome::Blocked { reason }) => {
                    reason.kind() == AttentionKind::Conflict
                }
                (OperationClass::NoOp, OperationOutcome::NoChange) => true,
                (
                    OperationClass::Unsupported | OperationClass::Conflict | OperationClass::NoOp,
                    _,
                ) => false,
                _ => true,
            };
            if !class_outcome_valid {
                return Err(OperationContractError::InvalidOutcomeForOperationClass {
                    operation: result.operation_id.clone(),
                    class: operation.class(),
                });
            }

            if let OperationOutcome::SkippedDependency { dependencies } = &result.outcome {
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
                            | OperationOutcome::Pending
                    ) {
                        return Err(OperationContractError::InvalidSkippedDependencyOutcome {
                            operation: result.operation_id.clone(),
                            dependency: dependency.clone(),
                        });
                    }
                }
            }

            let actual_blockers = operation
                .dependencies()
                .iter()
                .filter_map(|dependency| {
                    let dependency_id = dependency.operation_id();
                    let result = collected
                        .get(dependency_id)
                        .expect("every planned operation has an exact result");
                    (!matches!(
                        result.outcome,
                        OperationOutcome::Applied | OperationOutcome::NoChange
                    ))
                    .then(|| dependency_id.clone())
                })
                .collect::<BTreeSet<_>>();
            match &result.outcome {
                OperationOutcome::SkippedDependency { dependencies }
                    if dependencies != &actual_blockers =>
                {
                    return Err(OperationContractError::DependencySkipMismatch {
                        operation: result.operation_id.clone(),
                        expected: actual_blockers,
                        actual: dependencies.clone(),
                    });
                }
                OperationOutcome::SkippedDependency { .. } => {}
                _ if !actual_blockers.is_empty() => {
                    return Err(OperationContractError::DependencyBlockersRequireSkip {
                        operation: result.operation_id.clone(),
                        blockers: actual_blockers,
                    });
                }
                _ => {}
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
    CompatibilityTargetMismatch {
        id: OperationId,
        target: HarnessId,
        compatibility_target: HarnessId,
    },
    AffectedSurfaceTargetMismatch {
        id: OperationId,
        target: HarnessId,
        surface_target: HarnessId,
    },
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
    UncoveredResourceConsequence {
        operation: OperationId,
        code: ConsequenceCode,
    },
    UncoveredComponentConsequence {
        operation: OperationId,
        code: ConsequenceCode,
        component: ComponentId,
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
    InvalidOutcomeForOperationClass {
        operation: OperationId,
        class: OperationClass,
    },
    DependencyBlockersRequireSkip {
        operation: OperationId,
        blockers: BTreeSet<OperationId>,
    },
    DependencySkipMismatch {
        operation: OperationId,
        expected: BTreeSet<OperationId>,
        actual: BTreeSet<OperationId>,
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
            Self::CompatibilityTargetMismatch {
                id,
                target,
                compatibility_target,
            } => write!(
                formatter,
                "operation `{id}` targets `{target}` but compatibility targets `{compatibility_target}`"
            ),
            Self::AffectedSurfaceTargetMismatch {
                id,
                target,
                surface_target,
            } => write!(
                formatter,
                "operation `{id}` targets `{target}` but command preview targets `{surface_target}`"
            ),
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
            Self::UncoveredResourceConsequence { operation, code } => write!(
                formatter,
                "operation `{operation}` consequence `{code}` requires an acknowledged resource selector"
            ),
            Self::UncoveredComponentConsequence {
                operation,
                code,
                component,
            } => write!(
                formatter,
                "operation `{operation}` consequence `{code}` component `{component}` is not acknowledged"
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
                "operation `{operation}` was skipped by operation `{dependency}` that did not block execution"
            ),
            Self::InvalidOutcomeForOperationClass { operation, class } => write!(
                formatter,
                "operation `{operation}` result is invalid for {class:?}"
            ),
            Self::DependencyBlockersRequireSkip {
                operation,
                blockers,
            } => write!(
                formatter,
                "operation `{operation}` must be skipped because dependencies are blocked: {}",
                blockers
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::DependencySkipMismatch {
                operation,
                expected,
                actual,
            } => write!(
                formatter,
                "operation `{operation}` skip dependencies mismatch (expected {}, got {})",
                expected
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", "),
                actual
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
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
    use crate::domain::{CompatibilityClass, ConsequenceSummary, TransferFidelity};

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

    fn semantics(target: &str) -> OperationSemantics {
        semantics_with(target, [])
    }

    fn semantics_with(
        target: &str,
        affected_surfaces: impl IntoIterator<Item = AffectedSurface>,
    ) -> OperationSemantics {
        let target = HarnessId::new(target).unwrap();
        OperationSemantics::new(
            OperationAction::PluginInstall,
            Scope::Global,
            OperationReason::new(
                EvidenceCode::new("desired.plugin.missing").unwrap(),
                EvidenceDetail::new("The desired plugin is not installed").unwrap(),
            ),
            CompatibilityResult::new(
                target,
                CompatibilityClass::Compatible,
                TransferFidelity::Faithful,
                [],
                [],
            )
            .unwrap(),
            Provenance::Native,
            affected_surfaces,
        )
    }

    fn partial_operation(id: &str, dependencies: &[&str]) -> Operation {
        let selectors = [component_selector("plugin:tools", "hook:format")];
        let consequences = [consequence()];
        Operation::new(
            operation_id(id),
            HarnessId::new("codex").unwrap(),
            resource_selector("plugin:tools"),
            semantics("codex"),
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
            semantics("claude"),
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

    fn blocked_operation(id: &str, class: OperationClass) -> Operation {
        let attention = match class {
            OperationClass::Unsupported => AttentionReason::unsupported(
                EvidenceCode::new("native.unsupported").unwrap(),
                EvidenceDetail::new("No native operation is available").unwrap(),
            ),
            OperationClass::Conflict => AttentionReason::conflict(
                EvidenceCode::new("native.conflict").unwrap(),
                EvidenceDetail::new("An unmanaged native resource conflicts").unwrap(),
            ),
            _ => panic!("blocked_operation requires unsupported or conflict"),
        };
        Operation::new(
            operation_id(id),
            HarnessId::new("codex").unwrap(),
            resource_selector("plugin:tools"),
            semantics("codex"),
            class,
            Reversibility::NotApplicable,
            [],
            AcknowledgmentRequirement::not_required(),
            Some(attention),
        )
        .unwrap()
    }

    fn no_op(id: &str) -> Operation {
        Operation::new(
            operation_id(id),
            HarnessId::new("codex").unwrap(),
            resource_selector("plugin:tools"),
            semantics("codex"),
            OperationClass::NoOp,
            Reversibility::NotApplicable,
            [],
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
        assert_eq!(
            Plan::new([
                safe_operation("one", &["three"]),
                safe_operation("two", &["one"]),
                safe_operation("three", &["two"]),
                safe_operation("downstream", &["one"]),
            ])
            .unwrap_err(),
            OperationContractError::DependencyCycle {
                operations: [
                    operation_id("one"),
                    operation_id("two"),
                    operation_id("three")
                ]
                .into_iter()
                .collect()
            }
        );
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
            semantics("codex"),
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
            semantics("codex"),
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
            semantics("codex"),
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
    fn material_consequences_require_matching_acknowledged_components() {
        let resource_wide = MaterialConsequence::new(
            ConsequenceCode::new("component.omitted").unwrap(),
            [ComponentId::new("hook:other").unwrap()],
            ConsequenceSummary::new("Another hook will be omitted").unwrap(),
        );
        let selectors = [resource_selector("plugin:tools")];
        assert!(
            Operation::new(
                operation_id("resource-partial"),
                HarnessId::new("codex").unwrap(),
                resource_selector("plugin:tools"),
                semantics("codex"),
                OperationClass::Partial,
                Reversibility::Irreversible,
                [],
                AcknowledgmentRequirement::required(selectors.clone(), [resource_wide.clone()])
                    .unwrap(),
                Some(AttentionReason::acknowledgment_required(selectors, [resource_wide]).unwrap()),
            )
            .is_ok()
        );

        let uncovered = MaterialConsequence::new(
            ConsequenceCode::new("component.omitted").unwrap(),
            [ComponentId::new("hook:other").unwrap()],
            ConsequenceSummary::new("Another hook will be omitted").unwrap(),
        );
        let selectors = [component_selector("plugin:tools", "hook:format")];
        let error = Operation::new(
            operation_id("partial"),
            HarnessId::new("codex").unwrap(),
            resource_selector("plugin:tools"),
            semantics("codex"),
            OperationClass::Partial,
            Reversibility::Irreversible,
            [],
            AcknowledgmentRequirement::required(selectors.clone(), [uncovered.clone()]).unwrap(),
            Some(AttentionReason::acknowledgment_required(selectors, [uncovered]).unwrap()),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            OperationContractError::UncoveredComponentConsequence { .. }
        ));

        let resource_consequence = MaterialConsequence::new(
            ConsequenceCode::new("resource.materialized").unwrap(),
            [],
            ConsequenceSummary::new("The plugin will become skilltap-owned").unwrap(),
        );
        let selectors = [component_selector("plugin:tools", "hook:format")];
        let error = Operation::new(
            operation_id("partial"),
            HarnessId::new("codex").unwrap(),
            component_selector("plugin:tools", "hook:format"),
            semantics("codex"),
            OperationClass::Partial,
            Reversibility::Irreversible,
            [],
            AcknowledgmentRequirement::required(selectors.clone(), [resource_consequence.clone()])
                .unwrap(),
            Some(
                AttentionReason::acknowledgment_required(selectors, [resource_consequence])
                    .unwrap(),
            ),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            OperationContractError::UncoveredResourceConsequence { .. }
        ));

        let mut raw = serde_json::to_value(partial_operation("partial", &[]))
            .unwrap()
            .as_object()
            .unwrap()
            .clone();
        for field in ["acknowledgment", "attention"] {
            raw.get_mut(field)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .get_mut("consequences")
                .unwrap()
                .as_array_mut()
                .unwrap()[0]
                .as_object_mut()
                .unwrap()
                .insert(
                    "affected_components".into(),
                    serde_json::json!(["hook:other"]),
                );
        }
        assert!(serde_json::from_value::<Operation>(raw.into()).is_err());
    }

    #[test]
    fn operation_class_attention_and_reversibility_combinations_are_validated() {
        let safe_with_attention = Operation::new(
            operation_id("safe"),
            HarnessId::new("codex").unwrap(),
            resource_selector("skill:portable"),
            semantics("codex"),
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
            semantics("codex"),
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
            semantics("codex"),
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
    fn semantic_payloads_are_target_bound_redactable_and_deterministic() {
        let surfaces = [
            AffectedSurface::native_command(
                HarnessId::new("codex").unwrap(),
                NativeId::new("codex").unwrap(),
                [
                    CommandArgument::literal(NativeId::new("plugin").unwrap()),
                    CommandArgument::redacted(),
                ],
            ),
            AffectedSurface::file(AbsolutePath::new("/home/user/.codex/config.toml").unwrap()),
        ];
        let operation = Operation::new(
            operation_id("install"),
            HarnessId::new("codex").unwrap(),
            resource_selector("plugin:tools"),
            semantics_with("codex", surfaces),
            OperationClass::SafeNative,
            Reversibility::Reversible,
            [],
            AcknowledgmentRequirement::not_required(),
            None,
        )
        .unwrap();

        assert_eq!(operation.action(), OperationAction::PluginInstall);
        assert_eq!(operation.scope(), &Scope::Global);
        assert_eq!(operation.reason().code().as_str(), "desired.plugin.missing");
        assert_eq!(operation.compatibility().target(), operation.target());
        assert_eq!(operation.provenance(), Provenance::Native);
        let command = operation
            .affected_surfaces()
            .iter()
            .find(|surface| surface.executable().is_some())
            .unwrap();
        assert_eq!(command.target(), Some(operation.target()));
        assert_eq!(command.executable().unwrap().as_str(), "codex");
        assert!(command.arguments().unwrap()[1].is_redacted());
        assert!(command.arguments().unwrap()[1].literal_value().is_none());

        let json = serde_json::to_string(&operation).unwrap();
        assert_eq!(serde_json::from_str::<Operation>(&json).unwrap(), operation);
        assert!(json.contains(r#""action":"plugin_install""#));
        assert!(json.contains(r#"{"kind":"redacted"}"#));
        assert!(
            json.find(r#""kind":"file""#).unwrap()
                < json.find(r#""kind":"native_command""#).unwrap()
        );
        assert!(
            serde_json::from_str::<CommandArgument>(
                r#"{"kind":"redacted","value":"must-not-survive"}"#
            )
            .is_err()
        );
    }

    #[test]
    fn semantic_targets_are_validated_by_constructor_and_serde() {
        let mismatch = Operation::new(
            operation_id("install"),
            HarnessId::new("codex").unwrap(),
            resource_selector("plugin:tools"),
            semantics("claude"),
            OperationClass::SafeNative,
            Reversibility::Reversible,
            [],
            AcknowledgmentRequirement::not_required(),
            None,
        )
        .unwrap_err();
        assert!(matches!(
            mismatch,
            OperationContractError::CompatibilityTargetMismatch { .. }
        ));

        let command_mismatch = Operation::new(
            operation_id("install"),
            HarnessId::new("codex").unwrap(),
            resource_selector("plugin:tools"),
            semantics_with(
                "codex",
                [AffectedSurface::native_command(
                    HarnessId::new("claude").unwrap(),
                    NativeId::new("claude").unwrap(),
                    [],
                )],
            ),
            OperationClass::SafeNative,
            Reversibility::Reversible,
            [],
            AcknowledgmentRequirement::not_required(),
            None,
        )
        .unwrap_err();
        assert!(matches!(
            command_mismatch,
            OperationContractError::AffectedSurfaceTargetMismatch { .. }
        ));

        let raw = serde_json::to_value(safe_operation("install", &[])).unwrap();
        let mut raw = raw.as_object().unwrap().clone();
        raw.get_mut("compatibility")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("target".into(), serde_json::json!("codex"));
        assert!(serde_json::from_value::<Operation>(raw.into()).is_err());

        let command = AffectedSurface::native_command(
            HarnessId::new("codex").unwrap(),
            NativeId::new("codex").unwrap(),
            [],
        );
        let operation = Operation::new(
            operation_id("install"),
            HarnessId::new("codex").unwrap(),
            resource_selector("plugin:tools"),
            semantics_with("codex", [command]),
            OperationClass::SafeNative,
            Reversibility::Reversible,
            [],
            AcknowledgmentRequirement::not_required(),
            None,
        )
        .unwrap();
        let mut raw = serde_json::to_value(operation).unwrap();
        raw["affected_surfaces"][0]["target"] = serde_json::json!("claude");
        assert!(serde_json::from_value::<Operation>(raw).is_err());
    }

    #[test]
    fn constructor_and_deserialization_enforce_plan_invariants() {
        let mut invalid =
            serde_json::to_value(Plan::new([safe_operation("one", &[])]).unwrap()).unwrap();
        invalid["operations"][0]["dependencies"] = serde_json::json!([{"operation_id": "missing"}]);
        let error = serde_json::from_value::<Plan>(invalid).unwrap_err();
        assert!(error.to_string().contains("unknown operation `missing`"));

        let empty_ack = r#"{"kind":"required","selectors":[],"consequences":[]}"#;
        assert!(serde_json::from_str::<AcknowledgmentRequirement>(empty_ack).is_err());

        let unknown_field = r#"{"kind":"not_required","yes":true}"#;
        assert!(serde_json::from_str::<AcknowledgmentRequirement>(unknown_field).is_err());

        let mut invalid_reversibility = serde_json::to_value(blocked_operation(
            "unsupported",
            OperationClass::Unsupported,
        ))
        .unwrap();
        invalid_reversibility["reversibility"] = serde_json::json!("reversible");
        assert!(serde_json::from_value::<Operation>(invalid_reversibility).is_err());

        let mut outside_selector = serde_json::to_value(partial_operation("partial", &[])).unwrap();
        outside_selector["selector"] = serde_json::json!({
            "kind": "component",
            "resource_id": "plugin:tools",
            "component_id": "hook:format"
        });
        for field in ["acknowledgment", "attention"] {
            outside_selector[field]["selectors"] = serde_json::json!([{
                "kind": "resource",
                "resource_id": "plugin:tools"
            }]);
        }
        assert!(serde_json::from_value::<Operation>(outside_selector).is_err());
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
    fn apply_results_enforce_plan_class_outcomes() {
        for class in [OperationClass::Unsupported, OperationClass::Conflict] {
            let operation = blocked_operation("blocked", class);
            let error = ApplyResult::new(
                Plan::new([operation.clone()]).unwrap(),
                ApplyOutcome::Succeeded,
                [
                    OperationResult::new(operation_id("blocked"), OperationOutcome::Applied)
                        .unwrap(),
                ],
            )
            .unwrap_err();
            assert_eq!(
                error,
                OperationContractError::InvalidOutcomeForOperationClass {
                    operation: operation_id("blocked"),
                    class,
                }
            );

            let wrong_reason = match class {
                OperationClass::Unsupported => AttentionReason::conflict(
                    EvidenceCode::new("native.conflict").unwrap(),
                    EvidenceDetail::new("An unmanaged resource conflicts").unwrap(),
                ),
                OperationClass::Conflict => AttentionReason::unsupported(
                    EvidenceCode::new("native.unsupported").unwrap(),
                    EvidenceDetail::new("No native operation exists").unwrap(),
                ),
                _ => unreachable!(),
            };
            assert!(matches!(
                ApplyResult::new(
                    Plan::new([operation]).unwrap(),
                    ApplyOutcome::AttentionRequired,
                    [OperationResult::new(
                        operation_id("blocked"),
                        OperationOutcome::Blocked {
                            reason: wrong_reason
                        }
                    )
                    .unwrap()]
                ),
                Err(OperationContractError::InvalidOutcomeForOperationClass { .. })
            ));
        }

        let error = ApplyResult::new(
            Plan::new([no_op("noop")]).unwrap(),
            ApplyOutcome::Succeeded,
            [OperationResult::new(operation_id("noop"), OperationOutcome::Applied).unwrap()],
        )
        .unwrap_err();
        assert_eq!(
            error,
            OperationContractError::InvalidOutcomeForOperationClass {
                operation: operation_id("noop"),
                class: OperationClass::NoOp,
            }
        );

        let valid = ApplyResult::new(
            Plan::new([no_op("noop")]).unwrap(),
            ApplyOutcome::Succeeded,
            [OperationResult::new(operation_id("noop"), OperationOutcome::NoChange).unwrap()],
        )
        .unwrap();
        let mut raw = serde_json::to_value(valid).unwrap();
        raw["operations"][0]["outcome"] = serde_json::json!({"status": "applied"});
        assert!(serde_json::from_value::<ApplyResult>(raw).is_err());
    }

    #[test]
    fn dependency_blockers_require_an_exact_skip_set() {
        let plan = Plan::new([
            safe_operation("pending", &[]),
            safe_operation("failed", &[]),
            safe_operation("dependent", &["pending", "failed"]),
        ])
        .unwrap();
        let pending =
            OperationResult::new(operation_id("pending"), OperationOutcome::Pending).unwrap();
        let failed = OperationResult::new(
            operation_id("failed"),
            OperationOutcome::Failed {
                reason: AttentionReason::operation_failed(
                    EvidenceCode::new("native.command.failed").unwrap(),
                    EvidenceDetail::new("Native command failed").unwrap(),
                ),
            },
        )
        .unwrap();

        let error = ApplyResult::new(
            plan.clone(),
            ApplyOutcome::PartialFailure,
            [
                pending.clone(),
                failed.clone(),
                OperationResult::new(operation_id("dependent"), OperationOutcome::Applied).unwrap(),
            ],
        )
        .unwrap_err();
        assert!(matches!(
            error,
            OperationContractError::DependencyBlockersRequireSkip { .. }
        ));

        let error = ApplyResult::new(
            plan.clone(),
            ApplyOutcome::PartialFailure,
            [
                pending.clone(),
                failed.clone(),
                OperationResult::new(
                    operation_id("dependent"),
                    OperationOutcome::SkippedDependency {
                        dependencies: [operation_id("failed")].into_iter().collect(),
                    },
                )
                .unwrap(),
            ],
        )
        .unwrap_err();
        assert!(matches!(
            error,
            OperationContractError::DependencySkipMismatch { .. }
        ));

        let result = ApplyResult::new(
            plan,
            ApplyOutcome::PartialFailure,
            [
                pending,
                failed,
                OperationResult::new(
                    operation_id("dependent"),
                    OperationOutcome::SkippedDependency {
                        dependencies: [operation_id("failed"), operation_id("pending")]
                            .into_iter()
                            .collect(),
                    },
                )
                .unwrap(),
            ],
        )
        .unwrap();
        let json = serde_json::to_string(&result).unwrap();
        assert_eq!(serde_json::from_str::<ApplyResult>(&json).unwrap(), result);

        let mut raw = serde_json::to_value(result).unwrap();
        let dependent = raw["operations"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|result| result["operation_id"] == "dependent")
            .unwrap();
        dependent["outcome"] = serde_json::json!({"status": "applied"});
        assert!(serde_json::from_value::<ApplyResult>(raw).is_err());
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
        let pending = ApplyResult::new(
            Plan::new([safe_operation("one", &[])]).unwrap(),
            ApplyOutcome::AttentionRequired,
            [OperationResult::new(operation_id("one"), OperationOutcome::Pending).unwrap()],
        )
        .unwrap();
        let mut false_success = serde_json::to_value(pending).unwrap();
        false_success["outcome"] = serde_json::json!("succeeded");
        assert!(serde_json::from_value::<ApplyResult>(false_success).is_err());

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

        let missing_persisted = serde_json::json!({
            "plan": serde_json::to_value(Plan::new([safe_operation("one", &[])]).unwrap())
                .unwrap(),
            "outcome": "succeeded",
            "operations": []
        });
        assert!(serde_json::from_value::<ApplyResult>(missing_persisted).is_err());
    }
}
