//! Desired and observed resource graph contracts.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use super::{
    Fingerprint, HarnessId, HarnessSet, MaterialConsequence, NativeId, ResolvedRevision,
    ResourceKey, Scope, Source,
    dependency_graph::{ReferenceError, find_exact_cycle, validate_references},
    validate_identifier,
    validated_newtype::validated_string_newtype,
};
use serde::{Deserialize, Deserializer, Serialize};

mod finding;
pub use finding::{
    ObservationField, ObservationFieldCode, ObservationFields, ObservationFinding,
    ObservationFindingCode, ObservationFindingError, ObservationSeverity, ObservationSubject,
    ObservationSummary,
};

validated_string_newtype!(ComponentId, "component id", 256, validate_identifier);

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
    let dependencies = components
        .iter()
        .map(|(id, component)| (id, &component.dependencies));
    validate_references(dependencies.clone()).map_err(|error| match error {
        ReferenceError::SelfReference { node } => ComponentGraphError::SelfDependency { id: node },
        ReferenceError::UnknownReference { node, reference } => {
            ComponentGraphError::DanglingDependency {
                component: node,
                dependency: reference,
            }
        }
    })?;
    if let Some(components) = find_exact_cycle(dependencies) {
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
    resource: ResourceKey,
    harness: HarnessId,
    layer: ObservationLayer,
}

impl ObservationKey {
    pub fn new(resource: ResourceKey, harness: HarnessId, layer: ObservationLayer) -> Self {
        Self {
            resource,
            harness,
            layer,
        }
    }

    pub const fn resource(&self) -> &ResourceKey {
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
        }
    }
}

impl std::error::Error for ResourceContractError {}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(into = "DesiredResourceWire")]
pub struct DesiredResource {
    key: ResourceKey,
    kind: ResourceKind,
    targets: HarnessSet,
    origin: DesiredOrigin,
    source: Option<Source>,
    update: UpdateIntent,
    components: ComponentGraph,
    component_choices: BTreeMap<ComponentId, ComponentChoice>,
    accepted_consequences: BTreeMap<HarnessId, BTreeSet<MaterialConsequence>>,
    dependencies: BTreeSet<ResourceKey>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct DesiredResourceWire {
    key: ResourceKey,
    kind: ResourceKind,
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
    dependencies: BTreeSet<ResourceKey>,
}

impl DesiredResource {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        key: ResourceKey,
        kind: ResourceKind,
        targets: HarnessSet,
        origin: DesiredOrigin,
        source: Option<Source>,
        update: UpdateIntent,
        components: ComponentGraph,
        component_choices: BTreeMap<ComponentId, ComponentChoice>,
        accepted_consequences: BTreeMap<HarnessId, BTreeSet<MaterialConsequence>>,
        dependencies: BTreeSet<ResourceKey>,
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
            key,
            kind,
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

    pub const fn key(&self) -> &ResourceKey {
        &self.key
    }
    pub const fn id(&self) -> &super::ResourceId {
        self.key.id()
    }
    pub const fn kind(&self) -> ResourceKind {
        self.kind
    }
    pub const fn scope(&self) -> &Scope {
        self.key.scope()
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
    pub const fn dependencies(&self) -> &BTreeSet<ResourceKey> {
        &self.dependencies
    }
}

impl From<DesiredResource> for DesiredResourceWire {
    fn from(value: DesiredResource) -> Self {
        Self {
            key: value.key,
            kind: value.kind,
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
            value.key,
            value.kind,
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
    provenance: Provenance,
    ownership: Ownership,
    health: ResourceHealth,
    source: Option<Source>,
    components: ComponentGraph,
    dependencies: BTreeSet<ObservedDependency>,
    native_identity: NativeId,
    revision: Option<ResolvedRevision>,
    fingerprint: Option<Fingerprint>,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "resolution", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObservedDependency {
    Resolved { resource: ResourceKey },
    Unresolved { native_identity: NativeId },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ObservedResourceWire {
    key: ObservationKey,
    kind: ResourceKind,
    provenance: Provenance,
    ownership: Ownership,
    health: ResourceHealth,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<Source>,
    #[serde(default, skip_serializing_if = "ComponentGraph::is_empty")]
    components: ComponentGraph,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    dependencies: BTreeSet<ObservedDependency>,
    native_identity: NativeId,
    #[serde(skip_serializing_if = "Option::is_none")]
    revision: Option<ResolvedRevision>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fingerprint: Option<Fingerprint>,
}

impl ObservedResource {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        key: ObservationKey,
        kind: ResourceKind,
        provenance: Provenance,
        ownership: Ownership,
        health: ResourceHealth,
        source: Option<Source>,
        components: ComponentGraph,
        dependencies: BTreeSet<ObservedDependency>,
        native_identity: NativeId,
        revision: Option<ResolvedRevision>,
        fingerprint: Option<Fingerprint>,
    ) -> Self {
        Self {
            key,
            kind,
            provenance,
            ownership,
            health,
            source,
            components,
            dependencies,
            native_identity,
            revision,
            fingerprint,
        }
    }

    pub const fn key(&self) -> &ObservationKey {
        &self.key
    }
    pub const fn kind(&self) -> ResourceKind {
        self.kind
    }
    pub const fn scope(&self) -> &Scope {
        self.key.resource().scope()
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
    pub const fn source(&self) -> Option<&Source> {
        self.source.as_ref()
    }
    pub const fn dependencies(&self) -> &BTreeSet<ObservedDependency> {
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
}

impl From<ObservedResource> for ObservedResourceWire {
    fn from(value: ObservedResource) -> Self {
        Self {
            key: value.key,
            kind: value.kind,
            provenance: value.provenance,
            ownership: value.ownership,
            health: value.health,
            source: value.source,
            components: value.components,
            dependencies: value.dependencies,
            native_identity: value.native_identity,
            revision: value.revision,
            fingerprint: value.fingerprint,
        }
    }
}

impl From<ObservedResourceWire> for ObservedResource {
    fn from(value: ObservedResourceWire) -> Self {
        Self::new(
            value.key,
            value.kind,
            value.provenance,
            value.ownership,
            value.health,
            value.source,
            value.components,
            value.dependencies,
            value.native_identity,
            value.revision,
            value.fingerprint,
        )
    }
}

impl<'de> Deserialize<'de> for ObservedResource {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(ObservedResourceWire::deserialize(deserializer)?.into())
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
        key: ResourceKey,
    },
    DuplicateObservation {
        key: ObservationKey,
    },
    DanglingDependency {
        collection: GraphCollection,
        resource: ResourceKey,
        dependency: ResourceKey,
    },
    SelfDependency {
        collection: GraphCollection,
        key: ResourceKey,
    },
    DependencyCycle {
        collection: GraphCollection,
        resources: BTreeSet<ResourceKey>,
    },
    ObservedSelfDependency {
        key: ObservationKey,
    },
    ObservedDependencyCycle {
        harness: HarnessId,
        layer: ObservationLayer,
        resources: BTreeSet<ResourceKey>,
    },
}

impl fmt::Display for ResourceGraphError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateResource { collection, key } => {
                write!(formatter, "duplicate {collection} resource `{key}`")
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
            Self::SelfDependency { collection, key } => {
                write!(formatter, "{collection} resource `{key}` depends on itself")
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
    desired: BTreeMap<ResourceKey, DesiredResource>,
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
        let desired = collect_unique(desired, GraphCollection::Desired, |resource| resource.key())?;
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
        findings.sort();
        Ok(Self {
            desired,
            observed: observed_map,
            findings,
        })
    }

    pub fn desired(&self) -> &BTreeMap<ResourceKey, DesiredResource> {
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
    key: impl Fn(&T) -> &ResourceKey,
) -> Result<BTreeMap<ResourceKey, T>, ResourceGraphError> {
    let mut collected = BTreeMap::new();
    for resource in resources {
        let resource_key = key(&resource).clone();
        if collected.insert(resource_key.clone(), resource).is_some() {
            return Err(ResourceGraphError::DuplicateResource {
                collection,
                key: resource_key,
            });
        }
    }
    Ok(collected)
}

fn validate_dependencies<'a>(
    collection: GraphCollection,
    resources: impl IntoIterator<Item = (&'a ResourceKey, &'a BTreeSet<ResourceKey>)>,
) -> Result<(), ResourceGraphError> {
    let resources = resources.into_iter().collect::<BTreeMap<_, _>>();
    validate_references(
        resources
            .iter()
            .map(|(&id, &dependencies)| (id, dependencies)),
    )
    .map_err(|error| match error {
        ReferenceError::SelfReference { node } => ResourceGraphError::SelfDependency {
            collection,
            key: node,
        },
        ReferenceError::UnknownReference { node, reference } => {
            ResourceGraphError::DanglingDependency {
                collection,
                resource: node,
                dependency: reference,
            }
        }
    })?;
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
        BTreeMap<ResourceKey, BTreeSet<ResourceKey>>,
    > = BTreeMap::new();
    for (key, resource) in observed {
        let resolved = resource
            .dependencies
            .iter()
            .filter_map(|dependency| match dependency {
                ObservedDependency::Resolved { resource } => Some(resource.clone()),
                ObservedDependency::Unresolved { .. } => None,
            })
            .collect();
        contexts
            .entry((key.harness.clone(), key.layer))
            .or_default()
            .insert(key.resource.clone(), resolved);
    }
    for ((harness, layer), resources) in contexts {
        for (resource, dependencies) in &resources {
            if dependencies.contains(resource) {
                return Err(ResourceGraphError::ObservedSelfDependency {
                    key: ObservationKey::new(resource.clone(), harness.clone(), layer),
                });
            }
        }
        let known_edges = resources
            .iter()
            .map(|(resource, dependencies)| {
                (
                    resource,
                    dependencies
                        .iter()
                        .filter(|dependency| resources.contains_key(*dependency))
                        .cloned()
                        .collect::<BTreeSet<_>>(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        if let Some(resources) =
            find_exact_cycle(known_edges.iter().map(|(key, deps)| (*key, deps)))
        {
            return Err(ResourceGraphError::ObservedDependencyCycle {
                harness,
                layer,
                resources,
            });
        }
    }
    Ok(())
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
mod layered_tests;
