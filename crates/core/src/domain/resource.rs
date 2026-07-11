//! Desired and observed resource graph contracts.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use super::{
    Fingerprint, HarnessId, HarnessSet, NativeId, ResolvedRevision, ResourceId, Scope, Source,
    ValidationError, validate_text,
};

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

/// Adapter-owned JSON grouped by harness. Core preserves these values verbatim
/// and does not interpret keys inside a harness namespace.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct OpaqueHarnessMetadata(BTreeMap<HarnessId, Value>);

impl OpaqueHarnessMetadata {
    pub fn new(values: impl IntoIterator<Item = (HarnessId, Value)>) -> Self {
        Self(values.into_iter().collect())
    }

    pub fn get(&self, harness: &HarnessId) -> Option<&Value> {
        self.0.get(harness)
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = (&HarnessId, &Value)> {
        self.0.iter()
    }
}

impl<'de> Deserialize<'de> for OpaqueHarnessMetadata {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        BTreeMap::<HarnessId, Value>::deserialize(deserializer).map(Self)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DesiredResource {
    pub id: ResourceId,
    pub kind: ResourceKind,
    pub scope: Scope,
    pub targets: HarnessSet,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
    pub update: UpdateIntent,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub components: BTreeSet<ComponentKind>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub dependencies: BTreeSet<ResourceId>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedResource {
    pub id: ResourceId,
    pub kind: ResourceKind,
    pub scope: Scope,
    pub provenance: Provenance,
    pub ownership: Ownership,
    pub health: ResourceHealth,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub components: BTreeSet<ComponentKind>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub dependencies: BTreeSet<ResourceId>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub native_identities: BTreeMap<HarnessId, NativeId>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub revisions: BTreeMap<HarnessId, ResolvedRevision>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fingerprints: BTreeMap<HarnessId, Fingerprint>,
    #[serde(default, skip_serializing_if = "metadata_is_empty")]
    pub metadata: OpaqueHarnessMetadata,
}

fn metadata_is_empty(metadata: &OpaqueHarnessMetadata) -> bool {
    metadata.0.is_empty()
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
}

impl fmt::Display for ResourceGraphError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateResource { collection, id } => {
                write!(formatter, "duplicate {collection} resource `{id}`")
            }
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
        }
    }
}

impl std::error::Error for ResourceGraphError {}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(into = "ResourceGraphWire")]
pub struct ResourceGraph {
    desired: BTreeMap<ResourceId, DesiredResource>,
    observed: BTreeMap<ResourceId, ObservedResource>,
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
        let desired = collect_unique(desired, GraphCollection::Desired, |resource| &resource.id)?;
        let observed =
            collect_unique(observed, GraphCollection::Observed, |resource| &resource.id)?;
        validate_dependencies(
            GraphCollection::Desired,
            desired
                .iter()
                .map(|(id, resource)| (id, &resource.dependencies)),
        )?;
        validate_dependencies(
            GraphCollection::Observed,
            observed
                .iter()
                .map(|(id, resource)| (id, &resource.dependencies)),
        )?;
        let mut findings = findings.into_iter().collect::<Vec<_>>();
        findings.sort_by(finding_order);
        Ok(Self {
            desired,
            observed,
            findings,
        })
    }

    pub fn desired(&self) -> &BTreeMap<ResourceId, DesiredResource> {
        &self.desired
    }

    pub fn observed(&self) -> &BTreeMap<ResourceId, ObservedResource> {
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
    let mut remaining = BTreeMap::new();
    let mut dependents: BTreeMap<&ResourceId, BTreeSet<&ResourceId>> = BTreeMap::new();

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
            dependents.entry(dependency).or_default().insert(resource);
        }
        remaining.insert(resource, dependencies.len());
    }

    let mut ready = remaining
        .iter()
        .filter_map(|(&id, &count)| (count == 0).then_some(id))
        .collect::<BTreeSet<_>>();
    let mut visited = 0;
    while let Some(resource) = ready.pop_first() {
        visited += 1;
        if let Some(children) = dependents.get(resource) {
            for child in children {
                let count = remaining
                    .get_mut(child)
                    .expect("validated dependent belongs to graph");
                *count -= 1;
                if *count == 0 {
                    ready.insert(child);
                }
            }
        }
    }

    if visited != resources.len() {
        let resources = remaining
            .into_iter()
            .filter(|(_, count)| *count > 0)
            .map(|(id, _)| id.clone())
            .collect();
        return Err(ResourceGraphError::DependencyCycle {
            collection,
            resources,
        });
    }
    Ok(())
}

fn finding_order(left: &ObservationFinding, right: &ObservationFinding) -> std::cmp::Ordering {
    finding_key(left).cmp(&finding_key(right))
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
mod tests {
    use serde_json::json;

    use super::*;
    use crate::domain::{AbsolutePath, FingerprintAlgorithm};

    fn id(value: &str) -> ResourceId {
        ResourceId::new(value).unwrap()
    }

    fn harness(value: &str) -> HarnessId {
        HarnessId::new(value).unwrap()
    }

    fn desired(value: &str, dependencies: &[&str]) -> DesiredResource {
        DesiredResource {
            id: id(value),
            kind: ResourceKind::Plugin,
            scope: Scope::Global,
            targets: HarnessSet::new([harness("codex"), harness("claude")]).unwrap(),
            source: None,
            update: UpdateIntent::Track,
            components: BTreeSet::from([ComponentKind::Skill]),
            dependencies: dependencies.iter().copied().map(id).collect(),
        }
    }

    fn observed(value: &str, dependencies: &[&str]) -> ObservedResource {
        ObservedResource {
            id: id(value),
            kind: ResourceKind::Plugin,
            scope: Scope::Global,
            provenance: Provenance::Native,
            ownership: Ownership::Harness,
            health: ResourceHealth::Healthy,
            components: BTreeSet::from([ComponentKind::Skill]),
            dependencies: dependencies.iter().copied().map(id).collect(),
            native_identities: BTreeMap::from([(
                harness("claude"),
                NativeId::new(format!("{value}@catalog")).unwrap(),
            )]),
            revisions: BTreeMap::from([(
                harness("claude"),
                ResolvedRevision::Native(NativeId::new("1.2.3").unwrap()),
            )]),
            fingerprints: BTreeMap::from([(
                harness("claude"),
                Fingerprint::new(FingerprintAlgorithm::Sha256, "a".repeat(64)).unwrap(),
            )]),
            metadata: OpaqueHarnessMetadata::new([(
                harness("claude"),
                json!({"future_field": {"nested": [1, true, null]}}),
            )]),
        }
    }

    #[test]
    fn graph_rejects_duplicates_in_each_layer_but_aligns_desired_and_observed_ids() {
        assert!(
            ResourceGraph::new([desired("plugin:a", &[])], [observed("plugin:a", &[])], []).is_ok()
        );

        for error in [
            ResourceGraph::new([desired("plugin:a", &[]), desired("plugin:a", &[])], [], [])
                .unwrap_err(),
            ResourceGraph::new(
                [],
                [observed("plugin:a", &[]), observed("plugin:a", &[])],
                [],
            )
            .unwrap_err(),
        ] {
            assert!(matches!(
                error,
                ResourceGraphError::DuplicateResource { .. }
            ));
        }
    }

    #[test]
    fn graph_rejects_dangling_self_and_cyclic_dependencies_in_both_layers() {
        let dangling =
            ResourceGraph::new([desired("plugin:a", &["plugin:missing"])], [], []).unwrap_err();
        assert!(matches!(
            dangling,
            ResourceGraphError::DanglingDependency {
                collection: GraphCollection::Desired,
                ..
            }
        ));

        let self_edge =
            ResourceGraph::new([], [observed("plugin:a", &["plugin:a"])], []).unwrap_err();
        assert!(matches!(
            self_edge,
            ResourceGraphError::SelfDependency {
                collection: GraphCollection::Observed,
                ..
            }
        ));

        for dependencies in [
            vec![
                desired("plugin:a", &["plugin:b"]),
                desired("plugin:b", &["plugin:a"]),
            ],
            vec![
                desired("plugin:a", &["plugin:b"]),
                desired("plugin:b", &["plugin:c"]),
                desired("plugin:c", &["plugin:a"]),
            ],
        ] {
            let error = ResourceGraph::new(dependencies, [], []).unwrap_err();
            assert!(matches!(error, ResourceGraphError::DependencyCycle { .. }));
        }
    }

    #[test]
    fn graph_serialization_is_deterministic_and_round_trips_opaque_metadata() {
        let finding = ObservationFinding::new(
            harness("claude"),
            Scope::Project(AbsolutePath::new("/tmp/project").unwrap()),
            ObservationFindingKind::MalformedUnmanagedEntry,
            Some(NativeId::new("broken-plugin@catalog").unwrap()),
            "native plugin record omitted its source",
            json!({"raw": {"unknown": "preserved"}}),
        )
        .unwrap();
        let graph = ResourceGraph::new(
            [desired("plugin:z", &["plugin:a"]), desired("plugin:a", &[])],
            [
                observed("plugin:z", &["plugin:a"]),
                observed("plugin:a", &[]),
            ],
            [finding],
        )
        .unwrap();

        let json = serde_json::to_string(&graph).unwrap();
        assert!(json.find("plugin:a").unwrap() < json.find("plugin:z").unwrap());
        let decoded = serde_json::from_str::<ResourceGraph>(&json).unwrap();
        assert_eq!(decoded, graph);
        assert_eq!(serde_json::to_string(&decoded).unwrap(), json);
        assert_eq!(
            decoded
                .observed()
                .get(&id("plugin:a"))
                .unwrap()
                .metadata
                .get(&harness("claude"))
                .unwrap()["future_field"]["nested"][0],
            1
        );
    }

    #[test]
    fn malformed_unmanaged_entries_are_findings_not_fabricated_resources() {
        let finding = ObservationFinding::new(
            harness("codex"),
            Scope::Global,
            ObservationFindingKind::MalformedUnmanagedEntry,
            None,
            "marketplace entry has no usable identity",
            json!({"raw_entry": {"source": 7}}),
        )
        .unwrap();
        let graph = ResourceGraph::new([], [], [finding]).unwrap();

        assert!(graph.observed().is_empty());
        assert_eq!(graph.findings().len(), 1);
        assert!(graph.findings()[0].native_identity().is_none());
        assert_eq!(graph.findings()[0].metadata()["raw_entry"]["source"], 7);
    }

    #[test]
    fn owned_boundaries_reject_unknown_fields_and_invalid_finding_messages() {
        assert!(
            serde_json::from_value::<DesiredResource>(json!({
                "id": "plugin:a",
                "kind": "plugin",
                "scope": {"kind": "global"},
                "targets": ["codex"],
                "update": "track",
                "unexpected": true
            }))
            .is_err()
        );
        assert!(serde_json::from_value::<ResourceGraph>(json!({"unexpected": []})).is_err());
        assert!(
            ObservationFinding::new(
                harness("codex"),
                Scope::Global,
                ObservationFindingKind::MalformedUnmanagedEntry,
                None,
                " bad message ",
                Value::Null,
            )
            .is_err()
        );
        assert!(
            serde_json::from_value::<ObservationFinding>(json!({
                "harness": "codex",
                "scope": {"kind": "global"},
                "kind": "malformed_unmanaged_entry",
                "message": " bad message "
            }))
            .is_err()
        );
    }

    #[test]
    fn enums_have_stable_snake_case_forms() {
        assert_eq!(
            serde_json::to_string(&ResourceKind::StandaloneSkill).unwrap(),
            r#""standalone_skill""#
        );
        assert_eq!(
            serde_json::to_string(&ComponentKind::McpServer).unwrap(),
            r#"{"kind":"mcp_server"}"#
        );
        assert_eq!(
            serde_json::to_string(&ComponentKind::HarnessSpecific(
                NativeId::new("prompt-fragment").unwrap()
            ))
            .unwrap(),
            r#"{"kind":"harness_specific","native_kind":"prompt-fragment"}"#
        );
    }
}
