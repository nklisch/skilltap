//! Desired and observed resource graph contracts.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use super::{
    Fingerprint, HarnessId, HarnessSet, MaterialConsequence, NativeId, ResolvedRevision,
    ResourceId, Scope, Source, ValidationError, validate_identifier, validate_text,
};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ComponentId(String);

impl ComponentId {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        validate_identifier(&value, "component id", 256)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ComponentId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Serialize for ComponentId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ComponentId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    Harness,
    Marketplace,
    Plugin,
    StandaloneSkill,
    InstructionLocation,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(
    tag = "kind",
    content = "native_kind",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ComponentKind {
    Skill,
    McpServer,
    Hook,
    Agent,
    App,
    Connector,
    LspServer,
    Command,
    OutputStyle,
    Theme,
    Monitor,
    Executable,
    Settings,
    HarnessSpecific(NativeId),
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentRequiredness {
    Required,
    Optional,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceComponent {
    pub id: ComponentId,
    pub kind: ComponentKind,
    pub requiredness: ComponentRequiredness,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub dependencies: BTreeSet<ComponentId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ComponentGraphError {
    DuplicateComponent {
        id: ComponentId,
    },
    DanglingDependency {
        component: ComponentId,
        dependency: ComponentId,
    },
    SelfDependency {
        id: ComponentId,
    },
    DependencyCycle {
        components: BTreeSet<ComponentId>,
    },
}

impl fmt::Display for ComponentGraphError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateComponent { id } => write!(formatter, "duplicate component `{id}`"),
            Self::DanglingDependency {
                component,
                dependency,
            } => write!(
                formatter,
                "component `{component}` depends on unknown component `{dependency}`"
            ),
            Self::SelfDependency { id } => write!(formatter, "component `{id}` depends on itself"),
            Self::DependencyCycle { components } => write!(
                formatter,
                "component dependency cycle includes {}",
                components
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

impl std::error::Error for ComponentGraphError {}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(into = "Vec<ResourceComponent>")]
pub struct ComponentGraph(BTreeMap<ComponentId, ResourceComponent>);

impl ComponentGraph {
    pub fn new(
        components: impl IntoIterator<Item = ResourceComponent>,
    ) -> Result<Self, ComponentGraphError> {
        let mut collected = BTreeMap::new();
        for component in components {
            let id = component.id.clone();
            if collected.insert(id.clone(), component).is_some() {
                return Err(ComponentGraphError::DuplicateComponent { id });
            }
        }
        validate_component_dependencies(&collected)?;
        Ok(Self(collected))
    }

    pub fn get(&self, id: &ComponentId) -> Option<&ResourceComponent> {
        self.0.get(id)
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = (&ComponentId, &ResourceComponent)> {
        self.0.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl From<ComponentGraph> for Vec<ResourceComponent> {
    fn from(value: ComponentGraph) -> Self {
        value.0.into_values().collect()
    }
}

impl<'de> Deserialize<'de> for ComponentGraph {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let components = Vec::<ResourceComponent>::deserialize(deserializer)?;
        Self::new(components).map_err(serde::de::Error::custom)
    }
}

fn validate_component_dependencies(
    components: &BTreeMap<ComponentId, ResourceComponent>,
) -> Result<(), ComponentGraphError> {
    for (id, component) in components {
        for dependency in &component.dependencies {
            if dependency == id {
                return Err(ComponentGraphError::SelfDependency { id: id.clone() });
            }
            if !components.contains_key(dependency) {
                return Err(ComponentGraphError::DanglingDependency {
                    component: id.clone(),
                    dependency: dependency.clone(),
                });
            }
        }
    }
    if let Some(components) = find_exact_cycle(
        components
            .iter()
            .map(|(id, component)| (id, &component.dependencies)),
    ) {
        return Err(ComponentGraphError::DependencyCycle { components });
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Provenance {
    Native,
    Adopted,
    Direct,
    Materialized,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Ownership {
    Unmanaged,
    Harness,
    Skilltap,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateIntent {
    Track,
    Pinned,
    Disabled,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceHealth {
    Healthy,
    Drifted,
    Degraded,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    content = "source_harness",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum DesiredOrigin {
    Direct,
    Adopted(HarnessId),
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentChoice {
    Default,
    Include,
    Exclude,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationLayer {
    Declared,
    Effective,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationKey {
    resource: ResourceId,
    harness: HarnessId,
    layer: ObservationLayer,
}

impl ObservationKey {
    pub fn new(resource: ResourceId, harness: HarnessId, layer: ObservationLayer) -> Self {
        Self {
            resource,
            harness,
            layer,
        }
    }

    pub fn resource(&self) -> &ResourceId {
        &self.resource
    }

    pub fn harness(&self) -> &HarnessId {
        &self.harness
    }

    pub const fn layer(&self) -> ObservationLayer {
        self.layer
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResourceContractError {
    MissingComponentChoice {
        component: ComponentId,
    },
    UnknownComponentChoice {
        component: ComponentId,
    },
    ConsequenceTargetNotTargeted {
        target: HarnessId,
    },
    EmptyAcceptedConsequences {
        target: HarnessId,
    },
    ConsequenceComponentUnknown {
        target: HarnessId,
        component: ComponentId,
    },
    ObservationMetadataNotObject,
}

impl fmt::Display for ResourceContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingComponentChoice { component } => {
                write!(formatter, "component `{component}` has no explicit choice")
            }
            Self::UnknownComponentChoice { component } => {
                write!(
                    formatter,
                    "component choice references unknown `{component}`"
                )
            }
            Self::ConsequenceTargetNotTargeted { target } => write!(
                formatter,
                "accepted consequences reference untargeted harness `{target}`"
            ),
            Self::EmptyAcceptedConsequences { target } => write!(
                formatter,
                "accepted consequences for `{target}` must not be empty"
            ),
            Self::ConsequenceComponentUnknown { target, component } => write!(
                formatter,
                "accepted consequence for `{target}` references unknown component `{component}`"
            ),
            Self::ObservationMetadataNotObject => {
                write!(
                    formatter,
                    "observation metadata must be a JSON object or null"
                )
            }
        }
    }
}

impl std::error::Error for ResourceContractError {}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(into = "DesiredResourceWire")]
pub struct DesiredResource {
    id: ResourceId,
    kind: ResourceKind,
    scope: Scope,
    targets: HarnessSet,
    origin: DesiredOrigin,
    source: Option<Source>,
    update: UpdateIntent,
    components: ComponentGraph,
    component_choices: BTreeMap<ComponentId, ComponentChoice>,
    accepted_consequences: BTreeMap<HarnessId, BTreeSet<MaterialConsequence>>,
    dependencies: BTreeSet<ResourceId>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct DesiredResourceWire {
    id: ResourceId,
    kind: ResourceKind,
    scope: Scope,
    targets: HarnessSet,
    origin: DesiredOrigin,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<Source>,
    update: UpdateIntent,
    #[serde(default, skip_serializing_if = "ComponentGraph::is_empty")]
    components: ComponentGraph,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    component_choices: BTreeMap<ComponentId, ComponentChoice>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    accepted_consequences: BTreeMap<HarnessId, BTreeSet<MaterialConsequence>>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    dependencies: BTreeSet<ResourceId>,
}

impl DesiredResource {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: ResourceId,
        kind: ResourceKind,
        scope: Scope,
        targets: HarnessSet,
        origin: DesiredOrigin,
        source: Option<Source>,
        update: UpdateIntent,
        components: ComponentGraph,
        component_choices: BTreeMap<ComponentId, ComponentChoice>,
        accepted_consequences: BTreeMap<HarnessId, BTreeSet<MaterialConsequence>>,
        dependencies: BTreeSet<ResourceId>,
    ) -> Result<Self, ResourceContractError> {
        for component in components.0.keys() {
            if !component_choices.contains_key(component) {
                return Err(ResourceContractError::MissingComponentChoice {
                    component: component.clone(),
                });
            }
        }
        for component in component_choices.keys() {
            if !components.0.contains_key(component) {
                return Err(ResourceContractError::UnknownComponentChoice {
                    component: component.clone(),
                });
            }
        }
        for (target, consequences) in &accepted_consequences {
            if !targets.contains(target) {
                return Err(ResourceContractError::ConsequenceTargetNotTargeted {
                    target: target.clone(),
                });
            }
            if consequences.is_empty() {
                return Err(ResourceContractError::EmptyAcceptedConsequences {
                    target: target.clone(),
                });
            }
            for consequence in consequences {
                for component in &consequence.affected_components {
                    if !components.0.contains_key(component) {
                        return Err(ResourceContractError::ConsequenceComponentUnknown {
                            target: target.clone(),
                            component: component.clone(),
                        });
                    }
                }
            }
        }
        Ok(Self {
            id,
            kind,
            scope,
            targets,
            origin,
            source,
            update,
            components,
            component_choices,
            accepted_consequences,
            dependencies,
        })
    }

    pub fn id(&self) -> &ResourceId {
        &self.id
    }
    pub const fn kind(&self) -> ResourceKind {
        self.kind
    }
    pub const fn scope(&self) -> &Scope {
        &self.scope
    }
    pub const fn targets(&self) -> &HarnessSet {
        &self.targets
    }
    pub const fn origin(&self) -> &DesiredOrigin {
        &self.origin
    }
    pub const fn source(&self) -> Option<&Source> {
        self.source.as_ref()
    }
    pub const fn update(&self) -> UpdateIntent {
        self.update
    }
    pub const fn components(&self) -> &ComponentGraph {
        &self.components
    }
    pub const fn component_choices(&self) -> &BTreeMap<ComponentId, ComponentChoice> {
        &self.component_choices
    }
    pub const fn accepted_consequences(
        &self,
    ) -> &BTreeMap<HarnessId, BTreeSet<MaterialConsequence>> {
        &self.accepted_consequences
    }
    pub const fn dependencies(&self) -> &BTreeSet<ResourceId> {
        &self.dependencies
    }
}

impl From<DesiredResource> for DesiredResourceWire {
    fn from(value: DesiredResource) -> Self {
        Self {
            id: value.id,
            kind: value.kind,
            scope: value.scope,
            targets: value.targets,
            origin: value.origin,
            source: value.source,
            update: value.update,
            components: value.components,
            component_choices: value.component_choices,
            accepted_consequences: value.accepted_consequences,
            dependencies: value.dependencies,
        }
    }
}

impl TryFrom<DesiredResourceWire> for DesiredResource {
    type Error = ResourceContractError;
    fn try_from(value: DesiredResourceWire) -> Result<Self, Self::Error> {
        Self::new(
            value.id,
            value.kind,
            value.scope,
            value.targets,
            value.origin,
            value.source,
            value.update,
            value.components,
            value.component_choices,
            value.accepted_consequences,
            value.dependencies,
        )
    }
}

impl<'de> Deserialize<'de> for DesiredResource {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        DesiredResourceWire::deserialize(deserializer)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(into = "ObservedResourceWire")]
pub struct ObservedResource {
    key: ObservationKey,
    kind: ResourceKind,
    scope: Scope,
    provenance: Provenance,
    ownership: Ownership,
    health: ResourceHealth,
    components: ComponentGraph,
    dependencies: BTreeSet<ResourceId>,
    native_identity: NativeId,
    revision: Option<ResolvedRevision>,
    fingerprint: Option<Fingerprint>,
    metadata: Value,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ObservedResourceWire {
    key: ObservationKey,
    kind: ResourceKind,
    scope: Scope,
    provenance: Provenance,
    ownership: Ownership,
    health: ResourceHealth,
    #[serde(default, skip_serializing_if = "ComponentGraph::is_empty")]
    components: ComponentGraph,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    dependencies: BTreeSet<ResourceId>,
    native_identity: NativeId,
    #[serde(skip_serializing_if = "Option::is_none")]
    revision: Option<ResolvedRevision>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fingerprint: Option<Fingerprint>,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    metadata: Value,
}

impl ObservedResource {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        key: ObservationKey,
        kind: ResourceKind,
        scope: Scope,
        provenance: Provenance,
        ownership: Ownership,
        health: ResourceHealth,
        components: ComponentGraph,
        dependencies: BTreeSet<ResourceId>,
        native_identity: NativeId,
        revision: Option<ResolvedRevision>,
        fingerprint: Option<Fingerprint>,
        metadata: Value,
    ) -> Result<Self, ResourceContractError> {
        if !metadata.is_null() && !metadata.is_object() {
            return Err(ResourceContractError::ObservationMetadataNotObject);
        }
        Ok(Self {
            key,
            kind,
            scope,
            provenance,
            ownership,
            health,
            components,
            dependencies,
            native_identity,
            revision,
            fingerprint,
            metadata,
        })
    }

    pub const fn key(&self) -> &ObservationKey {
        &self.key
    }
    pub const fn kind(&self) -> ResourceKind {
        self.kind
    }
    pub const fn scope(&self) -> &Scope {
        &self.scope
    }
    pub const fn provenance(&self) -> Provenance {
        self.provenance
    }
    pub const fn ownership(&self) -> Ownership {
        self.ownership
    }
    pub const fn health(&self) -> ResourceHealth {
        self.health
    }
    pub const fn components(&self) -> &ComponentGraph {
        &self.components
    }
    pub const fn dependencies(&self) -> &BTreeSet<ResourceId> {
        &self.dependencies
    }
    pub const fn native_identity(&self) -> &NativeId {
        &self.native_identity
    }
    pub const fn revision(&self) -> Option<&ResolvedRevision> {
        self.revision.as_ref()
    }
    pub const fn fingerprint(&self) -> Option<&Fingerprint> {
        self.fingerprint.as_ref()
    }
    pub const fn metadata(&self) -> &Value {
        &self.metadata
    }
}

impl From<ObservedResource> for ObservedResourceWire {
    fn from(value: ObservedResource) -> Self {
        Self {
            key: value.key,
            kind: value.kind,
            scope: value.scope,
            provenance: value.provenance,
            ownership: value.ownership,
            health: value.health,
            components: value.components,
            dependencies: value.dependencies,
            native_identity: value.native_identity,
            revision: value.revision,
            fingerprint: value.fingerprint,
            metadata: value.metadata,
        }
    }
}

impl TryFrom<ObservedResourceWire> for ObservedResource {
    type Error = ResourceContractError;
    fn try_from(value: ObservedResourceWire) -> Result<Self, Self::Error> {
        Self::new(
            value.key,
            value.kind,
            value.scope,
            value.provenance,
            value.ownership,
            value.health,
            value.components,
            value.dependencies,
            value.native_identity,
            value.revision,
            value.fingerprint,
            value.metadata,
        )
    }
}

impl<'de> Deserialize<'de> for ObservedResource {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        ObservedResourceWire::deserialize(deserializer)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationFindingKind {
    MalformedUnmanagedEntry,
    UnreadableNativeState,
    UnsupportedNativeShape,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(into = "ObservationFindingWire")]
pub struct ObservationFinding {
    harness: HarnessId,
    scope: Scope,
    kind: ObservationFindingKind,
    native_identity: Option<NativeId>,
    message: String,
    metadata: Value,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ObservationFindingWire {
    harness: HarnessId,
    scope: Scope,
    kind: ObservationFindingKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    native_identity: Option<NativeId>,
    message: String,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    metadata: Value,
}

impl ObservationFinding {
    pub fn new(
        harness: HarnessId,
        scope: Scope,
        kind: ObservationFindingKind,
        native_identity: Option<NativeId>,
        message: impl Into<String>,
        metadata: Value,
    ) -> Result<Self, ValidationError> {
        let message = message.into();
        validate_text(&message, "observation finding message", 4096)?;
        Ok(Self {
            harness,
            scope,
            kind,
            native_identity,
            message,
            metadata,
        })
    }

    pub fn harness(&self) -> &HarnessId {
        &self.harness
    }

    pub const fn scope(&self) -> &Scope {
        &self.scope
    }

    pub const fn kind(&self) -> ObservationFindingKind {
        self.kind
    }

    pub fn native_identity(&self) -> Option<&NativeId> {
        self.native_identity.as_ref()
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub const fn metadata(&self) -> &Value {
        &self.metadata
    }
}

impl From<ObservationFinding> for ObservationFindingWire {
    fn from(value: ObservationFinding) -> Self {
        Self {
            harness: value.harness,
            scope: value.scope,
            kind: value.kind,
            native_identity: value.native_identity,
            message: value.message,
            metadata: value.metadata,
        }
    }
}

impl TryFrom<ObservationFindingWire> for ObservationFinding {
    type Error = ValidationError;

    fn try_from(value: ObservationFindingWire) -> Result<Self, Self::Error> {
        Self::new(
            value.harness,
            value.scope,
            value.kind,
            value.native_identity,
            value.message,
            value.metadata,
        )
    }
}

impl<'de> Deserialize<'de> for ObservationFinding {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ObservationFindingWire::deserialize(deserializer)?;
        Self::try_from(wire).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GraphCollection {
    Desired,
    Observed,
}

impl fmt::Display for GraphCollection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Desired => "desired",
            Self::Observed => "observed",
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResourceGraphError {
    DuplicateResource {
        collection: GraphCollection,
        id: ResourceId,
    },
    DuplicateObservation {
        key: ObservationKey,
    },
    DanglingDependency {
        collection: GraphCollection,
        resource: ResourceId,
        dependency: ResourceId,
    },
    SelfDependency {
        collection: GraphCollection,
        id: ResourceId,
    },
    DependencyCycle {
        collection: GraphCollection,
        resources: BTreeSet<ResourceId>,
    },
    DanglingObservedDependency {
        key: ObservationKey,
        dependency: ResourceId,
    },
    ObservedSelfDependency {
        key: ObservationKey,
    },
    ObservedDependencyCycle {
        harness: HarnessId,
        layer: ObservationLayer,
        resources: BTreeSet<ResourceId>,
    },
}

impl fmt::Display for ResourceGraphError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateResource { collection, id } => {
                write!(formatter, "duplicate {collection} resource `{id}`")
            }
            Self::DuplicateObservation { key } => write!(
                formatter,
                "duplicate {:?} observation for `{}` in `{}`",
                key.layer, key.resource, key.harness
            ),
            Self::DanglingDependency {
                collection,
                resource,
                dependency,
            } => write!(
                formatter,
                "{collection} resource `{resource}` depends on unknown resource `{dependency}`"
            ),
            Self::SelfDependency { collection, id } => {
                write!(formatter, "{collection} resource `{id}` depends on itself")
            }
            Self::DependencyCycle {
                collection,
                resources,
            } => write!(
                formatter,
                "{collection} resource dependency cycle includes {}",
                resources
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::DanglingObservedDependency { key, dependency } => write!(
                formatter,
                "{:?} observation for `{}` in `{}` depends on unknown resource `{dependency}` in the same context",
                key.layer, key.resource, key.harness
            ),
            Self::ObservedSelfDependency { key } => write!(
                formatter,
                "{:?} observation for `{}` in `{}` depends on itself",
                key.layer, key.resource, key.harness
            ),
            Self::ObservedDependencyCycle {
                harness,
                layer,
                resources,
            } => write!(
                formatter,
                "{layer:?} observation dependency cycle in `{harness}` includes {}",
                resources
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

impl std::error::Error for ResourceGraphError {}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(into = "ResourceGraphWire")]
pub struct ResourceGraph {
    desired: BTreeMap<ResourceId, DesiredResource>,
    observed: BTreeMap<ObservationKey, ObservedResource>,
    findings: Vec<ObservationFinding>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ResourceGraphWire {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    desired: Vec<DesiredResource>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    observed: Vec<ObservedResource>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    findings: Vec<ObservationFinding>,
}

impl ResourceGraph {
    pub fn new(
        desired: impl IntoIterator<Item = DesiredResource>,
        observed: impl IntoIterator<Item = ObservedResource>,
        findings: impl IntoIterator<Item = ObservationFinding>,
    ) -> Result<Self, ResourceGraphError> {
        let desired = collect_unique(desired, GraphCollection::Desired, |resource| resource.id())?;
        let mut observed_map = BTreeMap::new();
        for observation in observed {
            let key = observation.key().clone();
            if observed_map.insert(key.clone(), observation).is_some() {
                return Err(ResourceGraphError::DuplicateObservation { key });
            }
        }
        validate_dependencies(
            GraphCollection::Desired,
            desired
                .iter()
                .map(|(id, resource)| (id, resource.dependencies())),
        )?;
        validate_observed_dependencies(&observed_map)?;
        let mut findings = findings.into_iter().collect::<Vec<_>>();
        findings.sort_by(finding_order);
        Ok(Self {
            desired,
            observed: observed_map,
            findings,
        })
    }

    pub fn desired(&self) -> &BTreeMap<ResourceId, DesiredResource> {
        &self.desired
    }

    pub fn observed(&self) -> &BTreeMap<ObservationKey, ObservedResource> {
        &self.observed
    }

    pub fn findings(&self) -> &[ObservationFinding] {
        &self.findings
    }
}

fn collect_unique<T>(
    resources: impl IntoIterator<Item = T>,
    collection: GraphCollection,
    id: impl Fn(&T) -> &ResourceId,
) -> Result<BTreeMap<ResourceId, T>, ResourceGraphError> {
    let mut collected = BTreeMap::new();
    for resource in resources {
        let resource_id = id(&resource).clone();
        if collected.insert(resource_id.clone(), resource).is_some() {
            return Err(ResourceGraphError::DuplicateResource {
                collection,
                id: resource_id,
            });
        }
    }
    Ok(collected)
}

fn validate_dependencies<'a>(
    collection: GraphCollection,
    resources: impl IntoIterator<Item = (&'a ResourceId, &'a BTreeSet<ResourceId>)>,
) -> Result<(), ResourceGraphError> {
    let resources = resources.into_iter().collect::<BTreeMap<_, _>>();
    let ids = resources.keys().copied().collect::<BTreeSet<_>>();
    for (&resource, dependencies) in &resources {
        for dependency in *dependencies {
            if dependency == resource {
                return Err(ResourceGraphError::SelfDependency {
                    collection,
                    id: resource.clone(),
                });
            }
            if !ids.contains(dependency) {
                return Err(ResourceGraphError::DanglingDependency {
                    collection,
                    resource: resource.clone(),
                    dependency: dependency.clone(),
                });
            }
        }
    }
    if let Some(resources) = find_exact_cycle(resources.iter().map(|(&id, &deps)| (id, deps))) {
        return Err(ResourceGraphError::DependencyCycle {
            collection,
            resources,
        });
    }
    Ok(())
}

fn validate_observed_dependencies(
    observed: &BTreeMap<ObservationKey, ObservedResource>,
) -> Result<(), ResourceGraphError> {
    let mut contexts: BTreeMap<
        (HarnessId, ObservationLayer),
        BTreeMap<ResourceId, BTreeSet<ResourceId>>,
    > = BTreeMap::new();
    for (key, resource) in observed {
        contexts
            .entry((key.harness.clone(), key.layer))
            .or_default()
            .insert(key.resource.clone(), resource.dependencies.clone());
    }
    for ((harness, layer), resources) in contexts {
        for (resource, dependencies) in &resources {
            let key = ObservationKey::new(resource.clone(), harness.clone(), layer);
            for dependency in dependencies {
                if dependency == resource {
                    return Err(ResourceGraphError::ObservedSelfDependency { key });
                }
                if !resources.contains_key(dependency) {
                    return Err(ResourceGraphError::DanglingObservedDependency {
                        key,
                        dependency: dependency.clone(),
                    });
                }
            }
        }
        if let Some(resources) = find_exact_cycle(resources.iter()) {
            return Err(ResourceGraphError::ObservedDependencyCycle {
                harness,
                layer,
                resources,
            });
        }
    }
    Ok(())
}

fn find_exact_cycle<'a, K>(
    graph: impl IntoIterator<Item = (&'a K, &'a BTreeSet<K>)>,
) -> Option<BTreeSet<K>>
where
    K: Clone + Ord + 'a,
{
    fn visit<K: Clone + Ord>(
        node: &K,
        graph: &BTreeMap<&K, &BTreeSet<K>>,
        complete: &mut BTreeSet<K>,
        stack: &mut Vec<K>,
        active: &mut BTreeMap<K, usize>,
    ) -> Option<BTreeSet<K>> {
        if complete.contains(node) {
            return None;
        }
        if let Some(start) = active.get(node) {
            return Some(stack[*start..].iter().cloned().collect());
        }
        active.insert(node.clone(), stack.len());
        stack.push(node.clone());
        if let Some(dependencies) = graph.get(node) {
            for dependency in *dependencies {
                if let Some(cycle) = visit(dependency, graph, complete, stack, active) {
                    return Some(cycle);
                }
            }
        }
        stack.pop();
        active.remove(node);
        complete.insert(node.clone());
        None
    }

    let graph = graph.into_iter().collect::<BTreeMap<_, _>>();
    let mut complete = BTreeSet::new();
    for node in graph.keys() {
        if let Some(cycle) = visit(
            *node,
            &graph,
            &mut complete,
            &mut Vec::new(),
            &mut BTreeMap::new(),
        ) {
            return Some(cycle);
        }
    }
    None
}

fn finding_order(left: &ObservationFinding, right: &ObservationFinding) -> std::cmp::Ordering {
    finding_key(left)
        .cmp(&finding_key(right))
        .then_with(|| canonical_json(&left.metadata).cmp(&canonical_json(&right.metadata)))
}

fn finding_key(
    finding: &ObservationFinding,
) -> (
    &HarnessId,
    &str,
    ObservationFindingKind,
    Option<&NativeId>,
    &str,
) {
    let scope = match &finding.scope {
        Scope::Global => "",
        Scope::Project(path) => path.as_str(),
    };
    (
        &finding.harness,
        scope,
        finding.kind,
        finding.native_identity.as_ref(),
        &finding.message,
    )
}

fn canonical_json(value: &Value) -> String {
    fn write(value: &Value, output: &mut String) {
        match value {
            Value::Null => output.push_str("null"),
            Value::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
            Value::Number(value) => output.push_str(&value.to_string()),
            Value::String(value) => {
                output.push_str(&serde_json::to_string(value).expect("JSON strings serialize"));
            }
            Value::Array(values) => {
                output.push('[');
                for (index, value) in values.iter().enumerate() {
                    if index > 0 {
                        output.push(',');
                    }
                    write(value, output);
                }
                output.push(']');
            }
            Value::Object(values) => {
                output.push('{');
                let mut entries = values.iter().collect::<Vec<_>>();
                entries.sort_unstable_by_key(|(key, _)| *key);
                for (index, (key, value)) in entries.into_iter().enumerate() {
                    if index > 0 {
                        output.push(',');
                    }
                    output.push_str(&serde_json::to_string(key).expect("JSON keys serialize"));
                    output.push(':');
                    write(value, output);
                }
                output.push('}');
            }
        }
    }

    let mut output = String::new();
    write(value, &mut output);
    output
}

impl From<ResourceGraph> for ResourceGraphWire {
    fn from(value: ResourceGraph) -> Self {
        Self {
            desired: value.desired.into_values().collect(),
            observed: value.observed.into_values().collect(),
            findings: value.findings,
        }
    }
}

impl TryFrom<ResourceGraphWire> for ResourceGraph {
    type Error = ResourceGraphError;

    fn try_from(value: ResourceGraphWire) -> Result<Self, Self::Error> {
        Self::new(value.desired, value.observed, value.findings)
    }
}

impl<'de> Deserialize<'de> for ResourceGraph {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ResourceGraphWire::deserialize(deserializer)?;
        Self::try_from(wire).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests;
