use std::collections::{BTreeMap, BTreeSet};

use skilltap_core::{
    domain::{
        AbsolutePath, AcknowledgmentRequirement, CapabilityId, CapabilityProfileId,
        CapabilityProfileSelection, CapabilitySet, CapabilitySupport, CompatibilityClass,
        CompatibilityResult, ComponentGraph, ConfiguredBinary, DesiredOrigin, DesiredResource,
        EvidenceCode, EvidenceDetail, ExecutableFileIdentity, ExecutableIdentity, HarnessId,
        HarnessInstallation, HarnessReachability, HarnessSet, NativeId, NativeVersion,
        ObservationBatch, ObservationEvidence, ObservationField, ObservationFields,
        ObservationFinding, ObservationFindingCode, ObservationKey, ObservationLayer,
        ObservationRequest, ObservationSeverity, ObservationSubject, ObservationSummary,
        ObservedEnvironment, ObservedResource, Operation, OperationAction, OperationClass,
        OperationId, OperationReason, OperationSelector, OperationSemantics, Ownership, Plan,
        ProfileAuthority, Provenance, ResourceHealth, ResourceId, ResourceKey, ResourceKind,
        Reversibility, Scope, ScopedCapabilitySets, TransferFidelity, UpdateIntent,
    },
    storage::{
        INVENTORY_SCHEMA_VERSION, InventoryDocument, ResourceState, STATE_SCHEMA_VERSION,
        StateDocument, Timestamp,
    },
};

fn harness(value: &str) -> HarnessId {
    HarnessId::new(value).unwrap()
}

fn key(value: &str, scope: Scope) -> ResourceKey {
    ResourceKey::new(ResourceId::new(value).unwrap(), scope)
}

fn desired(key: ResourceKey) -> DesiredResource {
    DesiredResource::new(
        key,
        ResourceKind::StandaloneSkill,
        HarnessSet::new([harness("codex")]).unwrap(),
        DesiredOrigin::Direct,
        None,
        UpdateIntent::Track,
        ComponentGraph::default(),
        BTreeMap::new(),
        BTreeMap::new(),
        BTreeSet::new(),
    )
    .unwrap()
}

fn state_resource(key: ResourceKey) -> ResourceState {
    ResourceState::new(
        key,
        BTreeMap::new(),
        Provenance::Adopted,
        Ownership::Harness,
        None,
        None,
        None,
        None,
        None,
        Timestamp::new(10, 0).unwrap(),
        None,
    )
    .unwrap()
}

fn planned_operation(id: &str, key: ResourceKey) -> Operation {
    let target = harness("codex");
    let semantics = OperationSemantics::new(
        OperationAction::SkillInstall,
        key.scope().clone(),
        OperationReason::new(
            EvidenceCode::new("desired.skill.present").unwrap(),
            EvidenceDetail::new("The desired skill is already present").unwrap(),
        ),
        CompatibilityResult::new(
            target.clone(),
            CompatibilityClass::Compatible,
            TransferFidelity::Faithful,
            [],
            [],
        )
        .unwrap(),
        Provenance::Direct,
        [],
    );
    Operation::new(
        OperationId::new(id).unwrap(),
        target,
        OperationSelector::Resource { resource: key },
        semantics,
        OperationClass::NoOp,
        Reversibility::NotApplicable,
        [],
        AcknowledgmentRequirement::not_required(),
        None,
    )
    .unwrap()
}

fn observation_request(scope: Scope) -> ObservationRequest {
    let capability = CapabilityId::new("plugin.observe").unwrap();
    let capabilities = ScopedCapabilitySets::new(
        CapabilitySet::new([(capability.clone(), CapabilitySupport::Supported)]),
        CapabilitySet::new([(capability, CapabilitySupport::Supported)]),
    );
    let installation = HarnessInstallation::new(
        harness("codex"),
        ConfiguredBinary::path_lookup(NativeId::new("codex").unwrap()).unwrap(),
        HarnessReachability::Reachable {
            executable: ExecutableIdentity::new(
                AbsolutePath::new("/usr/bin/codex").unwrap(),
                ExecutableFileIdentity::new(7, 11),
            ),
            native_version: NativeVersion::new("3.7.0").unwrap(),
        },
    );
    let evidence = ObservationEvidence::new(
        &installation,
        CapabilityProfileSelection::verified(
            CapabilityProfileId::new("codex-v3").unwrap(),
            capabilities,
        ),
    )
    .unwrap();
    assert_eq!(
        evidence.profile_authority(),
        ProfileAuthority::VerifiedCompiled
    );
    ObservationRequest::new(scope, evidence)
}

fn observed_resource(request: &ObservationRequest, layer: ObservationLayer) -> ObservedResource {
    let resource = key("skill:shared", request.scope().clone());
    ObservedResource::new(
        ObservationKey::new(resource, harness("codex"), layer),
        ResourceKind::StandaloneSkill,
        Provenance::Native,
        Ownership::Harness,
        ResourceHealth::Healthy,
        None,
        ComponentGraph::default(),
        BTreeSet::new(),
        NativeId::new("skill:shared@native").unwrap(),
        None,
        None,
    )
}

fn complete_observation(request: ObservationRequest) -> skilltap_core::domain::HarnessObservation {
    let subject_key = key("skill:shared", request.scope().clone());
    let finding = ObservationFinding::new(
        ObservationFindingCode::ResourceUnmanaged,
        ObservationSummary::ResourceUnmanaged,
        ObservationSeverity::Info,
        ObservationSubject::Resource {
            harness: harness("codex"),
            resource: subject_key,
        },
        ObservationFields::new([ObservationField::Adoptable(true)]).unwrap(),
    );
    skilltap_core::domain::HarnessObservation::new(
        request.clone(),
        [
            observed_resource(&request, ObservationLayer::Declared),
            observed_resource(&request, ObservationLayer::Effective),
        ],
        [finding],
    )
    .unwrap()
}

#[test]
fn state_wire_excludes_fresh_observation_and_profile_payloads() {
    let state = StateDocument::new(
        STATE_SCHEMA_VERSION,
        [],
        [state_resource(key("skill:shared", Scope::Global))],
        None,
        Some(Timestamp::new(10, 0).unwrap()),
        None,
    )
    .unwrap();
    let encoded = serde_json::to_value(&state).unwrap();
    let resource = encoded["resources"][0].as_object().unwrap();
    for ephemeral in [
        "declared",
        "effective",
        "findings",
        "profile",
        "executable",
        "observation",
    ] {
        assert!(!resource.contains_key(ephemeral), "persisted {ephemeral}");
    }

    let snapshot =
        serde_json::to_value(complete_observation(observation_request(Scope::Global))).unwrap();
    let declared = snapshot["resources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|resource| resource["key"]["layer"] == "declared")
        .unwrap()
        .clone();
    let effective = snapshot["resources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|resource| resource["key"]["layer"] == "effective")
        .unwrap()
        .clone();
    let ephemeral_snapshots = [
        ("declared", declared),
        ("effective", effective),
        ("findings", snapshot["findings"].clone()),
        (
            "profile",
            snapshot["request"]["evidence"]["profile"].clone(),
        ),
    ];
    for (ephemeral, snapshot) in ephemeral_snapshots {
        assert!(!snapshot.is_null());
        let mut injected = encoded.clone();
        injected["resources"][0][ephemeral] = snapshot;
        assert!(serde_json::from_value::<StateDocument>(injected).is_err());
    }
}

#[test]
fn equal_ids_remain_exact_across_inventory_state_and_plan() {
    let project_path = AbsolutePath::new("/work/project").unwrap();
    let global = key("skill:shared", Scope::Global);
    let project = key("skill:shared", Scope::Project(project_path.clone()));

    let inventory = InventoryDocument::new(
        INVENTORY_SCHEMA_VERSION,
        [project_path],
        [desired(project.clone()), desired(global.clone())],
    )
    .unwrap();
    let inventory: InventoryDocument =
        toml::from_str(&toml::to_string(&inventory).unwrap()).unwrap();
    assert_eq!(inventory.resources().len(), 2);
    assert!(inventory.resources().contains_key(&global));
    assert!(inventory.resources().contains_key(&project));

    let state = StateDocument::new(
        STATE_SCHEMA_VERSION,
        [],
        [
            state_resource(project.clone()),
            state_resource(global.clone()),
        ],
        None,
        None,
        None,
    )
    .unwrap();
    let state: StateDocument =
        serde_json::from_str(&serde_json::to_string(&state).unwrap()).unwrap();
    assert_eq!(state.resources().len(), 2);
    assert!(state.resources().contains_key(&global));
    assert!(state.resources().contains_key(&project));

    let plan = Plan::new([
        planned_operation("global-skill", global.clone()),
        planned_operation("project-skill", project.clone()),
    ])
    .unwrap();
    let plan: Plan = serde_json::from_str(&serde_json::to_string(&plan).unwrap()).unwrap();
    let selectors = plan
        .iter()
        .map(|(_, operation)| operation.selector().resource().clone())
        .collect::<BTreeSet<_>>();
    assert_eq!(selectors, BTreeSet::from([global, project]));
}

#[test]
fn complete_observation_snapshot_is_normalized_and_rejects_raw_payload_channels() {
    const SECRET: &str = "sk-test-native-output-must-not-enter";
    let request = observation_request(Scope::Global);
    let batch = ObservationBatch::new([request.clone()]).unwrap();
    let environment = ObservedEnvironment::new(
        batch,
        [skilltap_core::domain::HarnessObservationOutcome::observed(
            complete_observation(request),
        )],
    )
    .unwrap();
    let encoded = serde_json::to_string(&environment).unwrap();
    assert!(!encoded.contains(SECRET));
    assert!(!format!("{environment:?}").contains(SECRET));
    assert_eq!(
        serde_json::from_str::<ObservedEnvironment>(&encoded).unwrap(),
        environment
    );

    for raw_channel in ["argv", "stdout", "stderr", "settings", "native_payload"] {
        let mut payload = serde_json::to_value(&environment).unwrap();
        payload["outcomes"][0]["observation"][raw_channel] = serde_json::json!({"secret": SECRET});
        assert!(serde_json::from_value::<ObservedEnvironment>(payload).is_err());
    }
}
