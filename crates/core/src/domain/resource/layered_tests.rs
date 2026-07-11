use serde_json::{Value, json};

use super::*;
use crate::domain::{
    ConsequenceCode, ConsequenceSummary, FingerprintAlgorithm, GitCommit, ResourceId, SourceKind,
    SourceLocator,
};

fn id(value: &str) -> ResourceId {
    ResourceId::new(value).unwrap()
}

fn key(value: &str) -> ResourceKey {
    ResourceKey::new(id(value), Scope::Global)
}

fn project_key(value: &str, path: &str) -> ResourceKey {
    ResourceKey::new(
        id(value),
        Scope::Project(crate::domain::AbsolutePath::new(path).unwrap()),
    )
}

fn component_id(value: &str) -> ComponentId {
    ComponentId::new(value).unwrap()
}

fn harness(value: &str) -> HarnessId {
    HarnessId::new(value).unwrap()
}

fn component(value: &str, dependencies: &[&str]) -> ResourceComponent {
    ResourceComponent {
        id: component_id(value),
        kind: if value.starts_with("hook:") {
            ComponentKind::Hook
        } else {
            ComponentKind::Skill
        },
        requiredness: ComponentRequiredness::Required,
        dependencies: dependencies.iter().copied().map(component_id).collect(),
    }
}

fn components() -> ComponentGraph {
    ComponentGraph::new([
        component("skill:main", &[]),
        component("hook:format", &["skill:main"]),
    ])
    .unwrap()
}

fn choices() -> BTreeMap<ComponentId, ComponentChoice> {
    BTreeMap::from([
        (component_id("hook:format"), ComponentChoice::Exclude),
        (component_id("skill:main"), ComponentChoice::Default),
    ])
}

fn consequence(component: &str) -> MaterialConsequence {
    MaterialConsequence::new(
        ConsequenceCode::new("component.omitted").unwrap(),
        [component_id(component)],
        ConsequenceSummary::new("The component will not be installed").unwrap(),
    )
}

fn desired_with(
    value: &str,
    origin: DesiredOrigin,
    targets: HarnessSet,
    component_choices: BTreeMap<ComponentId, ComponentChoice>,
    accepted: BTreeMap<HarnessId, BTreeSet<MaterialConsequence>>,
    dependencies: &[&str],
) -> Result<DesiredResource, ResourceContractError> {
    DesiredResource::new(
        key(value),
        ResourceKind::Plugin,
        targets,
        origin,
        Some(
            Source::new(
                SourceKind::Git,
                SourceLocator::new("https://example.test/plugin.git").unwrap(),
                None,
            )
            .unwrap(),
        ),
        UpdateIntent::Track,
        components(),
        component_choices,
        accepted,
        dependencies.iter().copied().map(key).collect(),
    )
}

fn desired(value: &str, dependencies: &[&str]) -> DesiredResource {
    desired_with(
        value,
        DesiredOrigin::Direct,
        HarnessSet::new([harness("claude"), harness("codex")]).unwrap(),
        choices(),
        BTreeMap::new(),
        dependencies,
    )
    .unwrap()
}

fn desired_at(
    resource: ResourceKey,
    dependencies: impl IntoIterator<Item = ResourceKey>,
) -> DesiredResource {
    DesiredResource::new(
        resource,
        ResourceKind::Plugin,
        HarnessSet::new([harness("claude"), harness("codex")]).unwrap(),
        DesiredOrigin::Direct,
        None,
        UpdateIntent::Track,
        components(),
        choices(),
        BTreeMap::new(),
        dependencies.into_iter().collect(),
    )
    .unwrap()
}

fn observed(
    value: &str,
    harness_name: &str,
    layer: ObservationLayer,
    dependencies: &[&str],
) -> ObservedResource {
    ObservedResource::new(
        ObservationKey::new(key(value), harness(harness_name), layer),
        ResourceKind::Plugin,
        Provenance::Native,
        Ownership::Harness,
        ResourceHealth::Healthy,
        Some(
            Source::new(
                SourceKind::Git,
                SourceLocator::new("https://example.test/plugin.git").unwrap(),
                None,
            )
            .unwrap(),
        ),
        components(),
        dependencies
            .iter()
            .copied()
            .map(|value| ObservedDependency::Resolved {
                resource: key(value),
            })
            .collect(),
        NativeId::new(format!("{value}@native")).unwrap(),
        Some(ResolvedRevision::GitCommit(
            GitCommit::new("a".repeat(40)).unwrap(),
        )),
        Some(Fingerprint::new(FingerprintAlgorithm::Sha256, "b".repeat(64)).unwrap()),
    )
}

#[test]
fn desired_contract_validates_choices_and_accepted_consequence_contexts() {
    let codex = harness("codex");
    let targets = HarnessSet::new([codex.clone()]).unwrap();
    let mut missing = choices();
    missing.remove(&component_id("hook:format"));
    assert!(matches!(
        desired_with(
            "plugin:a",
            DesiredOrigin::Direct,
            targets.clone(),
            missing,
            BTreeMap::new(),
            &[],
        ),
        Err(ResourceContractError::MissingComponentChoice { .. })
    ));

    let mut unknown = choices();
    unknown.insert(component_id("skill:missing"), ComponentChoice::Include);
    assert!(matches!(
        desired_with(
            "plugin:a",
            DesiredOrigin::Direct,
            targets.clone(),
            unknown,
            BTreeMap::new(),
            &[],
        ),
        Err(ResourceContractError::UnknownComponentChoice { .. })
    ));

    assert!(matches!(
        desired_with(
            "plugin:a",
            DesiredOrigin::Direct,
            targets.clone(),
            choices(),
            BTreeMap::from([(
                harness("claude"),
                BTreeSet::from([consequence("hook:format")])
            )]),
            &[],
        ),
        Err(ResourceContractError::ConsequenceTargetNotTargeted { .. })
    ));
    assert!(matches!(
        desired_with(
            "plugin:a",
            DesiredOrigin::Direct,
            targets,
            choices(),
            BTreeMap::from([(codex, BTreeSet::from([consequence("skill:missing")]))]),
            &[],
        ),
        Err(ResourceContractError::ConsequenceComponentUnknown { .. })
    ));
}

#[test]
fn adopted_origin_is_independent_of_current_targets_and_round_trips() {
    let adopted = desired_with(
        "plugin:a",
        DesiredOrigin::Adopted(harness("claude")),
        HarnessSet::new([harness("codex")]).unwrap(),
        choices(),
        BTreeMap::new(),
        &[],
    )
    .unwrap();
    let json = serde_json::to_string(&adopted).unwrap();
    let decoded = serde_json::from_str::<DesiredResource>(&json).unwrap();
    assert_eq!(decoded, adopted);
    assert_eq!(decoded.origin(), &DesiredOrigin::Adopted(harness("claude")));
    assert_eq!(
        decoded
            .targets()
            .iter()
            .map(HarnessId::as_str)
            .collect::<Vec<_>>(),
        ["codex"]
    );
    assert_eq!(serde_json::to_string(&decoded).unwrap(), json);
}

#[test]
fn serde_cannot_bypass_desired_context_validation_or_owned_wires() {
    let valid = desired_with(
        "plugin:a",
        DesiredOrigin::Adopted(harness("claude")),
        HarnessSet::new([harness("claude"), harness("codex")]).unwrap(),
        choices(),
        BTreeMap::from([(
            harness("codex"),
            BTreeSet::from([consequence("hook:format")]),
        )]),
        &[],
    )
    .unwrap();
    let mut wire = serde_json::to_value(&valid).unwrap();
    wire["component_choices"]
        .as_object_mut()
        .unwrap()
        .insert("skill:missing".into(), json!("include"));
    assert!(serde_json::from_value::<DesiredResource>(wire).is_err());

    let mut wire = serde_json::to_value(&valid).unwrap();
    wire["unexpected"] = Value::Bool(true);
    assert!(serde_json::from_value::<DesiredResource>(wire).is_err());

    let mut legacy = serde_json::to_value(&valid).unwrap();
    legacy.as_object_mut().unwrap().remove("key");
    legacy["id"] = json!("plugin:a");
    legacy["scope"] = json!({"kind":"global"});
    assert!(serde_json::from_value::<DesiredResource>(legacy).is_err());
}

#[test]
fn observation_key_preserves_resource_harness_and_layer() {
    let resource = id("plugin:a");
    let key = ObservationKey::new(
        ResourceKey::new(resource.clone(), Scope::Global),
        harness("claude"),
        ObservationLayer::Effective,
    );
    assert_eq!(key.resource().id(), &resource);
    assert_eq!(key.harness().as_str(), "claude");
    assert_eq!(key.layer(), ObservationLayer::Effective);
    assert_eq!(
        serde_json::to_string(&key).unwrap(),
        r#"{"resource":{"id":"plugin:a","scope":{"kind":"global"}},"harness":"claude","layer":"effective"}"#
    );
    assert!(
        serde_json::from_str::<ObservationKey>(
            r#"{"resource":{"id":"plugin:a","scope":{"kind":"global"}},"harness":"claude","layer":"effective","extra":true}"#
        )
        .is_err()
    );
}

#[test]
fn graph_preserves_multi_harness_and_two_layer_observations_deterministically() {
    let observations = [
        observed("plugin:a", "codex", ObservationLayer::Effective, &[]),
        observed("plugin:a", "claude", ObservationLayer::Declared, &[]),
        observed("plugin:a", "claude", ObservationLayer::Effective, &[]),
        observed("plugin:a", "codex", ObservationLayer::Declared, &[]),
    ];
    let forward = ResourceGraph::new([desired("plugin:a", &[])], observations.clone(), []).unwrap();
    let reversed = ResourceGraph::new(
        [desired("plugin:a", &[])],
        observations.into_iter().rev(),
        [],
    )
    .unwrap();

    assert_eq!(forward.observed().len(), 4);
    let json = serde_json::to_string(&forward).unwrap();
    assert_eq!(json, serde_json::to_string(&reversed).unwrap());
    assert_eq!(
        serde_json::from_str::<ResourceGraph>(&json).unwrap(),
        forward
    );
    let observation = forward
        .observed()
        .get(&ObservationKey::new(
            key("plugin:a"),
            harness("codex"),
            ObservationLayer::Effective,
        ))
        .unwrap();
    assert_eq!(observation.native_identity().as_str(), "plugin:a@native");
    assert!(observation.source().is_some());
}

#[test]
fn equal_ids_in_global_and_project_scopes_coexist_and_cross_scope_edges_resolve() {
    let global = key("plugin:shared");
    let project_a = project_key("plugin:shared", "/work/a");
    let project_b = project_key("plugin:shared", "/work/b");
    let graph = ResourceGraph::new(
        [
            desired_at(global.clone(), []),
            desired_at(project_a.clone(), [global.clone()]),
            desired_at(project_b.clone(), [project_a.clone()]),
        ],
        [],
        [],
    )
    .unwrap();

    assert_eq!(graph.desired().len(), 3);
    assert_eq!(
        graph.desired()[&project_a].dependencies(),
        &BTreeSet::from([global])
    );
    let json = serde_json::to_string(&graph).unwrap();
    assert_eq!(
        serde_json::to_string(&serde_json::from_str::<ResourceGraph>(&json).unwrap()).unwrap(),
        json
    );
}

#[test]
fn exact_scope_is_preserved_in_dangling_self_and_cycle_diagnostics() {
    let project = project_key("plugin:a", "/work/a");
    let missing = project_key("plugin:missing", "/work/b");
    assert_eq!(
        ResourceGraph::new([desired_at(project.clone(), [missing.clone()])], [], []).unwrap_err(),
        ResourceGraphError::DanglingDependency {
            collection: GraphCollection::Desired,
            resource: project.clone(),
            dependency: missing,
        }
    );
    assert_eq!(
        ResourceGraph::new([desired_at(project.clone(), [project.clone()])], [], []).unwrap_err(),
        ResourceGraphError::SelfDependency {
            collection: GraphCollection::Desired,
            key: project.clone()
        }
    );
    let global = key("plugin:a");
    assert_eq!(
        ResourceGraph::new(
            [
                desired_at(global.clone(), [project.clone()]),
                desired_at(project.clone(), [global.clone()])
            ],
            [],
            [],
        )
        .unwrap_err(),
        ResourceGraphError::DependencyCycle {
            collection: GraphCollection::Desired,
            resources: BTreeSet::from([global, project]),
        }
    );
}

#[test]
fn representative_adopted_desired_state_round_trips_all_explicit_context() {
    let desired = desired_with(
        "plugin:a",
        DesiredOrigin::Adopted(harness("claude")),
        HarnessSet::new([harness("claude"), harness("codex")]).unwrap(),
        choices(),
        BTreeMap::from([(
            harness("codex"),
            BTreeSet::from([consequence("hook:format")]),
        )]),
        &[],
    )
    .unwrap();
    let graph = ResourceGraph::new([desired], [], []).unwrap();
    let json = serde_json::to_string(&graph).unwrap();
    let decoded = serde_json::from_str::<ResourceGraph>(&json).unwrap();
    assert_eq!(decoded, graph);
    let desired = decoded.desired().get(&key("plugin:a")).unwrap();
    assert_eq!(desired.origin(), &DesiredOrigin::Adopted(harness("claude")));
    assert_eq!(
        desired
            .component_choices()
            .get(&component_id("hook:format")),
        Some(&ComponentChoice::Exclude)
    );
    assert_eq!(
        desired
            .accepted_consequences()
            .get(&harness("codex"))
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn graph_rejects_only_duplicate_exact_observation_keys() {
    let declared = observed("plugin:a", "codex", ObservationLayer::Declared, &[]);
    assert!(matches!(
        ResourceGraph::new([], [declared.clone(), declared], []),
        Err(ResourceGraphError::DuplicateObservation { .. })
    ));
    assert!(
        ResourceGraph::new(
            [],
            [
                observed("plugin:a", "codex", ObservationLayer::Declared, &[]),
                observed("plugin:a", "codex", ObservationLayer::Effective, &[]),
            ],
            [],
        )
        .is_ok()
    );
}

#[test]
fn unresolved_observed_dependencies_remain_evidence_without_aborting_siblings() {
    let mut dependent = observed("plugin:b", "codex", ObservationLayer::Effective, &[]);
    dependent
        .dependencies
        .insert(ObservedDependency::Unresolved {
            native_identity: NativeId::new("native/missing").unwrap(),
        });
    let graph = ResourceGraph::new(
        [],
        [
            observed("plugin:a", "codex", ObservationLayer::Effective, &[]),
            dependent,
        ],
        [],
    )
    .unwrap();

    let dependency = graph
        .observed()
        .get(&ObservationKey::new(
            key("plugin:b"),
            harness("codex"),
            ObservationLayer::Effective,
        ))
        .unwrap()
        .dependencies()
        .iter()
        .next()
        .unwrap();
    assert!(
        matches!(dependency, ObservedDependency::Unresolved { native_identity } if native_identity.as_str() == "native/missing")
    );
}

#[test]
fn cycle_diagnostics_exclude_downstream_non_cycle_nodes() {
    let component_error = ComponentGraph::new([
        component("skill:a", &["skill:b"]),
        component("skill:b", &["skill:a"]),
        component("skill:downstream", &["skill:a"]),
    ])
    .unwrap_err();
    assert_eq!(
        component_error,
        ComponentGraphError::DependencyCycle {
            components: BTreeSet::from([component_id("skill:a"), component_id("skill:b")])
        }
    );

    let desired_error = ResourceGraph::new(
        [
            desired("plugin:a", &["plugin:b"]),
            desired("plugin:b", &["plugin:a"]),
            desired("plugin:downstream", &["plugin:a"]),
        ],
        [],
        [],
    )
    .unwrap_err();
    assert_eq!(
        desired_error,
        ResourceGraphError::DependencyCycle {
            collection: GraphCollection::Desired,
            resources: BTreeSet::from([key("plugin:a"), key("plugin:b")]),
        }
    );
}

#[test]
fn resource_graphs_select_the_first_of_multiple_disjoint_cycles() {
    let component_error = ComponentGraph::new([
        component("skill:a", &["skill:b"]),
        component("skill:b", &["skill:a"]),
        component("skill:c", &["skill:d"]),
        component("skill:d", &["skill:c"]),
    ])
    .unwrap_err();
    assert_eq!(
        component_error,
        ComponentGraphError::DependencyCycle {
            components: BTreeSet::from([component_id("skill:a"), component_id("skill:b")])
        }
    );

    let desired_error = ResourceGraph::new(
        [
            desired("plugin:a", &["plugin:b"]),
            desired("plugin:b", &["plugin:a"]),
            desired("plugin:c", &["plugin:d"]),
            desired("plugin:d", &["plugin:c"]),
        ],
        [],
        [],
    )
    .unwrap_err();
    assert_eq!(
        desired_error,
        ResourceGraphError::DependencyCycle {
            collection: GraphCollection::Desired,
            resources: BTreeSet::from([key("plugin:a"), key("plugin:b")]),
        }
    );

    let observed_error = ResourceGraph::new(
        [],
        [
            observed(
                "plugin:a",
                "claude",
                ObservationLayer::Effective,
                &["plugin:b"],
            ),
            observed(
                "plugin:b",
                "claude",
                ObservationLayer::Effective,
                &["plugin:a"],
            ),
            observed(
                "plugin:c",
                "claude",
                ObservationLayer::Effective,
                &["plugin:d"],
            ),
            observed(
                "plugin:d",
                "claude",
                ObservationLayer::Effective,
                &["plugin:c"],
            ),
        ],
        [],
    )
    .unwrap_err();
    assert_eq!(
        observed_error,
        ResourceGraphError::ObservedDependencyCycle {
            harness: harness("claude"),
            layer: ObservationLayer::Effective,
            resources: BTreeSet::from([key("plugin:a"), key("plugin:b")]),
        }
    );
}

#[test]
fn observed_resources_reject_removed_scope_and_metadata_wires() {
    let mut wire = serde_json::to_value(observed(
        "plugin:a",
        "codex",
        ObservationLayer::Effective,
        &[],
    ))
    .unwrap();
    wire["metadata"] = json!(3);
    assert!(serde_json::from_value::<ObservedResource>(wire).is_err());

    let mut wire = serde_json::to_value(observed(
        "plugin:a",
        "codex",
        ObservationLayer::Effective,
        &[],
    ))
    .unwrap();
    wire["scope"] = json!({"kind":"global"});
    assert!(serde_json::from_value::<ObservedResource>(wire).is_err());
}

#[test]
fn component_graph_rejects_invalid_constructor_and_wire_edges() {
    for error in [
        ComponentGraph::new([component("skill:a", &[]), component("skill:a", &[])]).unwrap_err(),
        ComponentGraph::new([component("skill:a", &["skill:missing"])]).unwrap_err(),
        ComponentGraph::new([component("skill:a", &["skill:a"])]).unwrap_err(),
        ComponentGraph::new([
            component("skill:a", &["skill:b"]),
            component("skill:b", &["skill:a"]),
        ])
        .unwrap_err(),
    ] {
        assert!(matches!(
            error,
            ComponentGraphError::DuplicateComponent { .. }
                | ComponentGraphError::DanglingDependency { .. }
                | ComponentGraphError::SelfDependency { .. }
                | ComponentGraphError::DependencyCycle { .. }
        ));
    }
    assert!(serde_json::from_value::<ComponentGraph>(json!([
        {"id":"skill:a","kind":{"kind":"skill"},"requiredness":"required","dependencies":["skill:b"]},
        {"id":"skill:b","kind":{"kind":"skill"},"requiredness":"required","dependencies":["skill:a"]}
    ])).is_err());
    assert!(serde_json::from_str::<ComponentId>(r#"" Skill:a""#).is_err());
}

#[test]
fn desired_graph_rejects_duplicate_dangling_and_self_edges_at_both_boundaries() {
    assert!(matches!(
        ResourceGraph::new([desired("plugin:a", &[]), desired("plugin:a", &[])], [], []),
        Err(ResourceGraphError::DuplicateResource { .. })
    ));
    assert!(matches!(
        ResourceGraph::new([desired("plugin:a", &["plugin:missing"])], [], []),
        Err(ResourceGraphError::DanglingDependency { .. })
    ));
    assert!(matches!(
        ResourceGraph::new([desired("plugin:a", &["plugin:a"])], [], []),
        Err(ResourceGraphError::SelfDependency { .. })
    ));

    let invalid = json!({"desired": [
        serde_json::to_value(desired("plugin:a", &["plugin:b"])).unwrap(),
        serde_json::to_value(desired("plugin:b", &["plugin:a"])).unwrap()
    ]});
    assert!(serde_json::from_value::<ResourceGraph>(invalid).is_err());
}

#[test]
fn observed_cycle_diagnostics_are_contextual_and_exact() {
    let error = ResourceGraph::new(
        [],
        [
            observed(
                "plugin:a",
                "claude",
                ObservationLayer::Effective,
                &["plugin:b"],
            ),
            observed(
                "plugin:b",
                "claude",
                ObservationLayer::Effective,
                &["plugin:a"],
            ),
            observed(
                "plugin:downstream",
                "claude",
                ObservationLayer::Effective,
                &["plugin:a"],
            ),
        ],
        [],
    )
    .unwrap_err();
    assert_eq!(
        error,
        ResourceGraphError::ObservedDependencyCycle {
            harness: harness("claude"),
            layer: ObservationLayer::Effective,
            resources: BTreeSet::from([key("plugin:a"), key("plugin:b")]),
        }
    );
}

#[test]
fn dangling_resolved_observed_edges_remain_visible_during_deserialization() {
    let graph = ResourceGraph::new(
        [],
        [
            observed("plugin:a", "codex", ObservationLayer::Effective, &[]),
            observed(
                "plugin:b",
                "codex",
                ObservationLayer::Effective,
                &["plugin:a"],
            ),
        ],
        [],
    )
    .unwrap();
    let mut wire = serde_json::to_value(graph).unwrap();
    wire["observed"]
        .as_array_mut()
        .unwrap()
        .retain(|observation| observation["key"]["resource"]["id"] != "plugin:a");
    let decoded = serde_json::from_value::<ResourceGraph>(wire).unwrap();
    assert_eq!(decoded.observed().len(), 1);
}

#[test]
fn findings_remain_separate_and_deterministically_ordered_by_metadata() {
    let finding = |metadata| {
        ObservationFinding::new(
            harness("codex"),
            Scope::Global,
            ObservationFindingKind::MalformedUnmanagedEntry,
            None,
            "unmanaged entry has no stable identity",
            metadata,
        )
        .unwrap()
    };
    let first = finding(json!({"z": 1, "nested": {"b": 2, "a": 1}}));
    let second = finding(json!({"a": 2}));
    let forward = ResourceGraph::new([], [], [first.clone(), second.clone()]).unwrap();
    let reversed = ResourceGraph::new([], [], [second, first]).unwrap();
    assert!(forward.observed().is_empty());
    assert_eq!(
        serde_json::to_string(&forward).unwrap(),
        serde_json::to_string(&reversed).unwrap()
    );
    assert!(
        serde_json::from_value::<ObservationFinding>(json!({
            "harness":"codex","scope":{"kind":"global"},
            "kind":"malformed_unmanaged_entry","message":" bad "
        }))
        .is_err()
    );
}

#[test]
fn enum_wire_forms_are_stable() {
    assert_eq!(
        serde_json::to_string(&ResourceKind::StandaloneSkill).unwrap(),
        r#""standalone_skill""#
    );
    assert_eq!(
        serde_json::to_string(&ComponentChoice::Default).unwrap(),
        r#""default""#
    );
    assert_eq!(
        serde_json::to_string(&ObservationLayer::Declared).unwrap(),
        r#""declared""#
    );
    assert_eq!(
        serde_json::to_string(&DesiredOrigin::Adopted(harness("claude"))).unwrap(),
        r#"{"kind":"adopted","source_harness":"claude"}"#
    );
}
