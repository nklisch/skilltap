//! Validated reconciliation plan and apply-result contracts.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{
    AbsolutePath, CompatibilityResult, ComponentId, ConsequenceCode, EvidenceCode, EvidenceDetail,
    HarnessId, MaterialConsequence, NativeId, OperationId, Provenance, ResourceId, Scope,
    TransferFidelity,
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
        if executable && semantics.affected_surfaces.is_empty() {
            return Err(OperationContractError::EmptyAffectedSurfaces { id, class });
        }
        let fidelity = semantics.compatibility.fidelity();
        let valid_fidelity = match class {
            OperationClass::SafeNative | OperationClass::SafeFaithfulEquivalent => {
                fidelity == TransferFidelity::Faithful
            }
            OperationClass::SafeMaterialization => fidelity == TransferFidelity::Materializable,
            OperationClass::Partial => fidelity == TransferFidelity::Partial,
            OperationClass::Unsupported | OperationClass::Conflict => {
                fidelity == TransferFidelity::Blocked
            }
            OperationClass::NoOp => fidelity != TransferFidelity::Blocked,
        };
        if !valid_fidelity {
            return Err(OperationContractError::InvalidFidelityForOperationClass {
                id,
                class,
                fidelity,
            });
        }
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
            if class == OperationClass::Partial
                && consequences != semantics.compatibility.consequences()
            {
                return Err(OperationContractError::PartialConsequenceMismatch {
                    id,
                    acknowledged: consequences.clone(),
                    compatibility: semantics.compatibility.consequences().clone(),
                });
            }
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
    EmptyAffectedSurfaces {
        id: OperationId,
        class: OperationClass,
    },
    InvalidFidelityForOperationClass {
        id: OperationId,
        class: OperationClass,
        fidelity: TransferFidelity,
    },
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
    PartialConsequenceMismatch {
        id: OperationId,
        acknowledged: BTreeSet<MaterialConsequence>,
        compatibility: BTreeSet<MaterialConsequence>,
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
            Self::EmptyAffectedSurfaces { id, class } => write!(
                formatter,
                "executable operation `{id}` with class {class:?} requires an affected surface"
            ),
            Self::InvalidFidelityForOperationClass {
                id,
                class,
                fidelity,
            } => write!(
                formatter,
                "operation `{id}` class {class:?} is incompatible with {fidelity:?} fidelity"
            ),
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
            Self::PartialConsequenceMismatch {
                id,
                acknowledged,
                compatibility,
            } => write!(
                formatter,
                "partial operation `{id}` acknowledgment consequences differ from compatibility consequences (acknowledged={}, compatibility={})",
                acknowledged.len(),
                compatibility.len()
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
mod tests;
