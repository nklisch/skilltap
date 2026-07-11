use std::{
    collections::BTreeMap,
    time::{Duration, UNIX_EPOCH},
};

use super::*;
use crate::domain::{
    ComponentGraph, DesiredOrigin, DesiredResource, Fingerprint, FingerprintAlgorithm, HarnessId,
    HarnessSet, NativeId, OperationId, OperationOutcome, OperationResult, Ownership, Provenance,
    RelativeArtifactPath, ResourceId, ResourceKind, Scope, UpdateIntent,
};

fn resource(
    id: &str,
    kind: ResourceKind,
    scope: Scope,
    dependencies: impl IntoIterator<Item = ResourceId>,
) -> DesiredResource {
    DesiredResource::new(
        ResourceId::new(id).unwrap(),
        kind,
        scope,
        HarnessSet::new([HarnessId::new("codex").unwrap()]).unwrap(),
        DesiredOrigin::Direct,
        None,
        UpdateIntent::Track,
        ComponentGraph::new([]).unwrap(),
        BTreeMap::new(),
        BTreeMap::new(),
        dependencies.into_iter().collect(),
    )
    .unwrap()
}

fn representative_inventory() -> InventoryDocument {
    let project = AbsolutePath::new("/work/skilltap").unwrap();
    let harness = resource("harness:codex", ResourceKind::Harness, Scope::Global, []);
    let marketplace = resource(
        "marketplace:personal",
        ResourceKind::Marketplace,
        Scope::Global,
        [harness.id().clone()],
    );
    let plugin = resource(
        "plugin:tools",
        ResourceKind::Plugin,
        Scope::Global,
        [marketplace.id().clone()],
    );
    let skill = resource(
        "skill:review",
        ResourceKind::StandaloneSkill,
        Scope::Project(project.clone()),
        [plugin.id().clone()],
    );
    let instructions = resource(
        "instructions:global",
        ResourceKind::InstructionLocation,
        Scope::Global,
        [skill.id().clone()],
    );
    InventoryDocument::new(
        INVENTORY_SCHEMA_VERSION,
        [project],
        [instructions, skill, plugin, marketplace, harness],
    )
    .unwrap()
}

fn fingerprint() -> Fingerprint {
    Fingerprint::new(FingerprintAlgorithm::Sha256, "a".repeat(64)).unwrap()
}

fn managed_resource(id: &str) -> ResourceState {
    let id = ResourceId::new(id).unwrap();
    let artifact_fingerprint = fingerprint();
    ResourceState::new(
        id.clone(),
        BTreeMap::new(),
        Provenance::Direct,
        Ownership::Skilltap,
        None,
        Some(
            ManagedArtifactRecord::for_artifact(
                id,
                ArtifactRole::DirectSkill,
                artifact_fingerprint.clone(),
            )
            .unwrap(),
        ),
        Some(artifact_fingerprint),
        None,
        None,
        Timestamp::new(100, 25).unwrap(),
        Some(
            ApplyRecord::new(
                Timestamp::new(101, 0).unwrap(),
                [OperationResult::new(
                    OperationId::new("install:skill").unwrap(),
                    OperationOutcome::Applied,
                )
                .unwrap()],
            )
            .unwrap(),
        ),
    )
    .unwrap()
}

fn representative_state() -> StateDocument {
    StateDocument::new(
        STATE_SCHEMA_VERSION,
        [HarnessState {
            harness: HarnessId::new("codex").unwrap(),
            native_version: Some(NativeId::new("1.2.3").unwrap()),
            observed_at: Timestamp::new(99, 5).unwrap(),
        }],
        [managed_resource("skill:review")],
        Some(Timestamp::new(98, 0).unwrap()),
        Some(Timestamp::new(100, 25).unwrap()),
        Some(Timestamp::new(101, 0).unwrap()),
    )
    .unwrap()
}

#[test]
fn config_defaults_are_explicit_strict_and_golden() {
    let config = ConfigDocument::defaults();
    assert_eq!(config.schema(), CONFIG_SCHEMA_VERSION);
    assert_eq!(config.harnesses().codex.binary.as_str(), "codex");
    assert_eq!(config.harnesses().claude.binary.as_str(), "claude");
    assert_eq!(
        config.instructions().claude_mode,
        ClaudeInstructionMode::Symlink
    );
    assert_eq!(config.updates().mode, UpdateMode::ApplySafe);
    assert_eq!(config.updates().interval.to_string(), "6h");

    let encoded = toml::to_string_pretty(&config).unwrap();
    assert_eq!(encoded, include_str!("fixtures/config.toml"));
    assert_eq!(toml::from_str::<ConfigDocument>(&encoded).unwrap(), config);
    assert!(toml::from_str::<ConfigDocument>("schema = 1").is_err());
    assert!(toml::from_str::<ConfigDocument>(&format!("unknown = true\n{encoded}")).is_err());
    assert!(
        toml::from_str::<ConfigDocument>(&encoded.replacen("schema = 1", "schema = 2", 1)).is_err()
    );
    assert!(
        toml::from_str::<ConfigDocument>(&encoded.replacen(
            "binary = \"codex\"",
            "binary = \"relative/path/codex\"",
            1,
        ))
        .is_err()
    );
}

#[test]
fn intervals_are_positive_and_canonical_at_both_boundaries() {
    for invalid in ["", "0h", "06h", "+6h", "6H", "6", "6ms", "1💥"] {
        assert!(invalid.parse::<UpdateInterval>().is_err(), "{invalid}");
        assert!(serde_json::from_str::<UpdateInterval>(&format!("\"{invalid}\"")).is_err());
    }
    let interval = UpdateInterval::new(15, UpdateIntervalUnit::Minutes).unwrap();
    assert_eq!(interval.value(), 15);
    assert_eq!(interval.unit(), UpdateIntervalUnit::Minutes);
    assert_eq!(serde_json::to_string(&interval).unwrap(), "\"15m\"");
}

#[test]
fn harness_binaries_accept_only_path_names_or_normalized_absolute_paths() {
    for valid in ["codex", "claude-code", "codex.exe", "/usr/local/bin/codex"] {
        let binary = HarnessBinary::new(valid).unwrap();
        assert_eq!(binary.as_str(), valid);
        assert_eq!(
            toml::from_str::<HarnessPolicy>(&format!("enabled = true\nbinary = {valid:?}\n"))
                .unwrap()
                .binary,
            binary
        );
    }

    for invalid in [
        "relative/path/codex",
        "./codex",
        "../codex",
        "codex/",
        "codex//",
        "/usr//bin/codex",
        "/usr/bin/../codex",
        "/usr/bin/codex/",
    ] {
        assert!(HarnessBinary::new(invalid).is_err(), "{invalid}");
        assert!(
            toml::from_str::<HarnessPolicy>(&format!("enabled = true\nbinary = {invalid:?}\n"))
                .is_err(),
            "{invalid}"
        );
    }
}

#[cfg(unix)]
#[test]
fn harness_binaries_reject_non_utf8_os_values_without_rendering_bytes() {
    use std::os::unix::ffi::OsStringExt;

    let error = HarnessBinary::try_from(std::ffi::OsString::from_vec(vec![0xff])).unwrap_err();
    assert!(matches!(error, SchemaError::NonUtf8HarnessBinary));
    assert_eq!(error.to_string(), "harness binary is not valid UTF-8");
}

#[test]
fn complete_inventory_is_readable_deterministic_golden_and_round_trips() {
    let inventory = representative_inventory();
    assert_eq!(inventory.schema(), INVENTORY_SCHEMA_VERSION);
    let encoded = toml::to_string_pretty(&inventory).unwrap();
    assert_eq!(encoded, include_str!("fixtures/inventory.toml"));
    assert_eq!(
        toml::from_str::<InventoryDocument>(&encoded).unwrap(),
        inventory
    );
    assert_eq!(inventory.resources().len(), 5);
    assert_eq!(toml::to_string_pretty(&inventory).unwrap(), encoded);
}

#[test]
fn inventory_constructor_and_toml_reject_graph_and_project_violations() {
    let project = AbsolutePath::new("/work/skilltap").unwrap();
    let standalone = resource(
        "skill:review",
        ResourceKind::StandaloneSkill,
        Scope::Project(project.clone()),
        [],
    );
    assert!(matches!(
        InventoryDocument::new(INVENTORY_SCHEMA_VERSION, [], [standalone.clone()]),
        Err(SchemaError::UndeclaredProject { .. })
    ));
    assert!(
        InventoryDocument::new(
            INVENTORY_SCHEMA_VERSION,
            [project.clone(), project],
            [standalone.clone()]
        )
        .is_err()
    );
    assert!(
        InventoryDocument::new(
            INVENTORY_SCHEMA_VERSION,
            [],
            [standalone.clone(), standalone]
        )
        .is_err()
    );

    let dangling = resource(
        "plugin:dangling",
        ResourceKind::Plugin,
        Scope::Global,
        [ResourceId::new("missing").unwrap()],
    );
    assert!(InventoryDocument::new(INVENTORY_SCHEMA_VERSION, [], [dangling]).is_err());
    let left = resource(
        "plugin:left",
        ResourceKind::Plugin,
        Scope::Global,
        [ResourceId::new("plugin:right").unwrap()],
    );
    let right = resource(
        "plugin:right",
        ResourceKind::Plugin,
        Scope::Global,
        [ResourceId::new("plugin:left").unwrap()],
    );
    assert!(InventoryDocument::new(INVENTORY_SCHEMA_VERSION, [], [left, right]).is_err());

    let encoded = toml::to_string_pretty(&representative_inventory()).unwrap();
    assert!(toml::from_str::<InventoryDocument>(&format!("unknown = true\n{encoded}")).is_err());
    assert!(
        toml::from_str::<InventoryDocument>(&encoded.replacen("schema = 1", "schema = 9", 1))
            .is_err()
    );

    let mut duplicate: toml::Value = toml::from_str(&encoded).unwrap();
    let resources = duplicate["resources"].as_array_mut().unwrap();
    resources.push(resources[0].clone());
    assert!(toml::from_str::<InventoryDocument>(&toml::to_string(&duplicate).unwrap()).is_err());

    let mut undeclared: toml::Value = toml::from_str(&encoded).unwrap();
    undeclared["projects"] = toml::Value::Array(Vec::new());
    assert!(toml::from_str::<InventoryDocument>(&toml::to_string(&undeclared).unwrap()).is_err());
}

#[test]
fn timestamps_round_trip_system_time_and_reject_invalid_values() {
    let time = UNIX_EPOCH + Duration::new(4_000_000_000, 123_456_789);
    let timestamp = Timestamp::from_system_time(time).unwrap();
    assert_eq!(timestamp.seconds(), 4_000_000_000);
    assert_eq!(timestamp.nanoseconds(), 123_456_789);
    assert_eq!(timestamp.to_system_time().unwrap(), time);
    assert!(Timestamp::new(0, 1_000_000_000).is_err());
    assert!(Timestamp::from_system_time(UNIX_EPOCH - Duration::from_nanos(1)).is_err());
    assert!(
        serde_json::from_str::<Timestamp>(r#"{"seconds":0,"nanoseconds":1000000000}"#).is_err()
    );
}

#[test]
fn state_is_strict_golden_and_excludes_desired_policy() {
    let state = representative_state();
    assert_eq!(state.schema(), STATE_SCHEMA_VERSION);
    let encoded = serde_json::to_string_pretty(&state).unwrap() + "\n";
    assert_eq!(encoded, include_str!("fixtures/state.json"));
    assert_eq!(
        serde_json::from_str::<StateDocument>(&encoded).unwrap(),
        state
    );

    let mut unknown: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    unknown
        .as_object_mut()
        .unwrap()
        .insert("targets".into(), serde_json::json!(["codex"]));
    assert!(serde_json::from_value::<StateDocument>(unknown).is_err());

    let mut desired: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    desired["resources"][0]
        .as_object_mut()
        .unwrap()
        .insert("update".into(), serde_json::json!("track"));
    assert!(serde_json::from_value::<StateDocument>(desired).is_err());
    assert!(
        serde_json::from_str::<StateDocument>(&encoded.replacen(
            "\"schema\": 1",
            "\"schema\": 2",
            1
        ))
        .is_err()
    );
}

#[test]
fn state_validates_duplicate_ids_ownership_roles_and_apply_records() {
    let first = managed_resource("skill:first");
    assert!(
        StateDocument::new(
            STATE_SCHEMA_VERSION,
            [],
            [first.clone(), first],
            None,
            None,
            None
        )
        .is_err()
    );

    let operation = OperationResult::new(
        OperationId::new("install:skill").unwrap(),
        OperationOutcome::Applied,
    )
    .unwrap();
    assert!(
        ApplyRecord::new(
            Timestamp::new(1, 0).unwrap(),
            [operation.clone(), operation]
        )
        .is_err()
    );

    let id = ResourceId::new("skill:bad").unwrap();
    let wrong_owner = ManagedArtifactRecord::for_artifact(
        ResourceId::new("skill:other").unwrap(),
        ArtifactRole::DirectSkill,
        fingerprint(),
    )
    .unwrap();
    assert!(
        ResourceState::new(
            id.clone(),
            BTreeMap::new(),
            Provenance::Direct,
            Ownership::Skilltap,
            None,
            Some(wrong_owner),
            None,
            None,
            None,
            Timestamp::new(1, 0).unwrap(),
            None
        )
        .is_err()
    );
    assert!(
        ResourceState::new(
            id.clone(),
            BTreeMap::new(),
            Provenance::Direct,
            Ownership::Unmanaged,
            None,
            None,
            None,
            None,
            None,
            Timestamp::new(1, 0).unwrap(),
            None
        )
        .is_err()
    );
    let wrong_role = ManagedArtifactRecord::for_artifact(
        id.clone(),
        ArtifactRole::MaterializedPlugin,
        fingerprint(),
    )
    .unwrap();
    assert!(
        ResourceState::new(
            id,
            BTreeMap::new(),
            Provenance::Direct,
            Ownership::Skilltap,
            None,
            Some(wrong_role),
            None,
            None,
            None,
            Timestamp::new(1, 0).unwrap(),
            None
        )
        .is_err()
    );

    assert!(RelativeArtifactPath::new("../escape").is_err());

    let state = StateDocument::new(
        STATE_SCHEMA_VERSION,
        [],
        [managed_resource("skill:first")],
        None,
        None,
        None,
    )
    .unwrap();
    let mut duplicate_path = serde_json::to_value(state).unwrap();
    let mut second = duplicate_path["resources"][0].clone();
    second["resource_id"] = serde_json::json!("skill:second");
    second["managed_artifact"]["owner"] = serde_json::json!("skill:second");
    duplicate_path["resources"]
        .as_array_mut()
        .unwrap()
        .push(second);
    assert!(serde_json::from_value::<StateDocument>(duplicate_path).is_err());
}

#[test]
fn constructor_and_deserialization_enforce_state_invariants_equally() {
    let valid = managed_resource("skill:review");
    let mut value = serde_json::to_value(&valid).unwrap();
    value["ownership"] = serde_json::json!("unmanaged");
    assert!(serde_json::from_value::<ResourceState>(value).is_err());

    let apply = valid.managed_artifact().unwrap();
    assert_eq!(apply.owner(), valid.resource_id());
    assert_eq!(apply.role(), ArtifactRole::DirectSkill);
    assert!(apply.path().as_str().starts_with("artifact-direct-skill-"));

    let mut duplicate_apply = serde_json::to_value(
        ApplyRecord::new(
            Timestamp::new(1, 0).unwrap(),
            [OperationResult::new(
                OperationId::new("op:one").unwrap(),
                OperationOutcome::Applied,
            )
            .unwrap()],
        )
        .unwrap(),
    )
    .unwrap();
    let operation = duplicate_apply["operations"][0].clone();
    duplicate_apply["operations"]
        .as_array_mut()
        .unwrap()
        .push(operation);
    assert!(serde_json::from_value::<ApplyRecord>(duplicate_apply).is_err());

    let mut unknown_artifact = serde_json::to_value(apply).unwrap();
    unknown_artifact
        .as_object_mut()
        .unwrap()
        .insert("target".into(), serde_json::json!("codex"));
    assert!(serde_json::from_value::<ManagedArtifactRecord>(unknown_artifact).is_err());
}
