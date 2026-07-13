use std::{
    collections::BTreeMap,
    time::{Duration, UNIX_EPOCH},
};

use super::*;
use crate::bootstrap::BootstrapUpdateMode;
use crate::domain::{
    ComponentGraph, DesiredOrigin, DesiredResource, EvidenceCode, Fingerprint,
    FingerprintAlgorithm, HarnessId, HarnessSet, NativeId, OperationId, OperationOutcome,
    OperationResult, Ownership, Provenance, RelativeArtifactPath, ResolvedRevision, ResourceId,
    ResourceKey, ResourceKind, Scope, Source, SourceKind, SourceLocator, UpdateIntent,
};

fn resource(
    id: &str,
    kind: ResourceKind,
    scope: Scope,
    dependencies: impl IntoIterator<Item = ResourceKey>,
) -> DesiredResource {
    let key = ResourceKey::new(ResourceId::new(id).unwrap(), scope);
    DesiredResource::new(
        key,
        kind,
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
        [harness.key().clone()],
    );
    let plugin = resource(
        "plugin:tools",
        ResourceKind::Plugin,
        Scope::Global,
        [marketplace.key().clone()],
    );
    let skill = resource(
        "skill:review",
        ResourceKind::StandaloneSkill,
        Scope::Project(project.clone()),
        [plugin.key().clone()],
    );
    let instructions = resource(
        "instructions:global",
        ResourceKind::InstructionLocation,
        Scope::Global,
        [skill.key().clone()],
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

fn codex() -> HarnessId {
    HarnessId::new("codex").unwrap()
}

fn managed_resource(id: &str) -> ResourceState {
    managed_resource_at(id, Scope::Global)
}

fn managed_resource_at(id: &str, scope: Scope) -> ResourceState {
    let key = ResourceKey::new(ResourceId::new(id).unwrap(), scope);
    let artifact_fingerprint = fingerprint();
    let target = TargetResourceState::new(
        codex(),
        None,
        Provenance::Direct,
        Ownership::Skilltap,
        None,
        Some(
            ManagedArtifactRecord::for_artifact(
                key.clone(),
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
    .unwrap();
    ResourceState::new(key, [target]).unwrap()
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
fn state_journal_updates_exact_resource_without_touching_siblings() {
    let first = managed_resource("skill:first");
    let second = managed_resource("skill:second");
    let document = StateDocument::new(
        STATE_SCHEMA_VERSION,
        [],
        [first.clone(), second.clone()],
        None,
        None,
        None,
    )
    .unwrap();
    let updated = document
        .with_operation_result(
            first.key(),
            &codex(),
            Timestamp::new(200, 0).unwrap(),
            OperationResult::new(
                OperationId::new("update:first").unwrap(),
                OperationOutcome::Applied,
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(updated.resources().get(second.key()), Some(&second));
    let operations = updated
        .resources()
        .get(first.key())
        .unwrap()
        .target(&codex())
        .unwrap()
        .last_apply()
        .unwrap()
        .operations();
    assert!(operations.contains_key(&OperationId::new("install:skill").unwrap()));
    assert!(operations.contains_key(&OperationId::new("update:first").unwrap()));
    assert_eq!(
        updated.last_successful_application(),
        Some(Timestamp::new(200, 0).unwrap())
    );
}

#[test]
fn available_revision_cache_preserves_apply_history_and_siblings() {
    let first = managed_resource("skill:first");
    let second = managed_resource("skill:second");
    let document = StateDocument::new(
        STATE_SCHEMA_VERSION,
        [],
        [first.clone(), second.clone()],
        None,
        None,
        None,
    )
    .unwrap();
    let available = crate::domain::ResolvedRevision::GitCommit(
        crate::domain::GitCommit::new("b".repeat(40)).unwrap(),
    );
    let checked_at = Timestamp::new(300, 0).unwrap();
    let updated = document
        .with_available_revision(first.key(), &codex(), Some(available.clone()), checked_at)
        .unwrap();
    let resource = updated.resources().get(first.key()).unwrap();
    let resource = resource.target(&codex()).unwrap();
    assert_eq!(resource.available_revision(), Some(&available));
    assert_eq!(
        resource.last_apply(),
        first.target(&codex()).unwrap().last_apply()
    );
    assert_eq!(updated.resources().get(second.key()), Some(&second));
    assert_eq!(updated.last_update_check(), Some(checked_at));
}

#[test]
fn config_defaults_are_explicit_strict_and_golden() {
    let config = ConfigDocument::defaults();
    assert_eq!(config.schema(), CONFIG_SCHEMA_VERSION);
    assert!(!config.harnesses().codex.enabled);
    assert!(!config.harnesses().claude.enabled);
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
fn harness_policy_updates_one_target_and_preserves_the_other() {
    let config = ConfigDocument::defaults();
    let codex = HarnessId::new("codex").unwrap();
    let claude = HarnessId::new("claude").unwrap();
    let custom = HarnessBinary::new("/opt/bin/codex").unwrap();

    let updated = config
        .with_harness_policy(&codex, true, Some(&custom))
        .unwrap();
    assert!(updated.harnesses().codex.enabled);
    assert_eq!(updated.harnesses().codex.binary, custom);
    assert!(!updated.harnesses().claude.enabled);
    assert_eq!(updated.harnesses().claude, config.harnesses().claude);

    let disabled = updated.with_harness_enabled(&codex, false).unwrap();
    assert!(!disabled.harnesses().codex.enabled);
    assert_eq!(disabled.harnesses().codex.binary.as_str(), "/opt/bin/codex");
    assert!(disabled.with_harness_enabled(&claude, true).is_ok());
    assert!(
        disabled
            .with_harness_enabled(&HarnessId::new("pi").unwrap(), true)
            .is_err()
    );
}

#[test]
fn harness_policy_updates_preserve_binary_update_policy() {
    let config = ConfigDocument::defaults().with_bootstrap_policy(BinaryUpdatePolicy {
        mode: BootstrapUpdateMode::Check,
        allow_major: true,
    });
    let codex = HarnessId::new("codex").unwrap();

    let enabled = config.with_harness_enabled(&codex, true).unwrap();
    assert_eq!(enabled.bootstrap(), config.bootstrap());

    let disabled = enabled.with_harness_enabled(&codex, false).unwrap();
    assert_eq!(disabled.bootstrap(), config.bootstrap());

    let binary = HarnessBinary::new("/opt/bin/codex").unwrap();
    let updated = disabled
        .with_harness_policy(&codex, true, Some(&binary))
        .unwrap();
    assert_eq!(updated.bootstrap(), config.bootstrap());
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
        [ResourceKey::new(
            ResourceId::new("missing").unwrap(),
            Scope::Global,
        )],
    );
    assert!(InventoryDocument::new(INVENTORY_SCHEMA_VERSION, [], [dangling]).is_err());
    let left = resource(
        "plugin:left",
        ResourceKind::Plugin,
        Scope::Global,
        [ResourceKey::new(
            ResourceId::new("plugin:right").unwrap(),
            Scope::Global,
        )],
    );
    let right = resource(
        "plugin:right",
        ResourceKind::Plugin,
        Scope::Global,
        [ResourceKey::new(
            ResourceId::new("plugin:left").unwrap(),
            Scope::Global,
        )],
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

    let old_shape = encoded.replacen(
        "[resources.key]\nid = \"harness:codex\"\n\n[resources.key.scope]",
        "id = \"harness:codex\"\n\n[resources.scope]",
        1,
    );
    assert_ne!(old_shape, encoded);
    assert!(toml::from_str::<InventoryDocument>(&old_shape).is_err());
}

#[test]
fn equal_ids_in_distinct_scopes_coexist_in_inventory_and_state() {
    let project = AbsolutePath::new("/work/skilltap").unwrap();
    let global_desired = resource(
        "skill:shared",
        ResourceKind::StandaloneSkill,
        Scope::Global,
        [],
    );
    let project_desired = resource(
        "skill:shared",
        ResourceKind::StandaloneSkill,
        Scope::Project(project.clone()),
        [],
    );
    let inventory = InventoryDocument::new(
        INVENTORY_SCHEMA_VERSION,
        [project.clone()],
        [global_desired, project_desired],
    )
    .unwrap();
    assert_eq!(inventory.resources().len(), 2);

    let global_state = managed_resource_at("skill:shared", Scope::Global);
    let project_state = managed_resource_at("skill:shared", Scope::Project(project));
    assert_ne!(
        global_state
            .target(&codex())
            .unwrap()
            .managed_artifact()
            .unwrap()
            .path(),
        project_state
            .target(&codex())
            .unwrap()
            .managed_artifact()
            .unwrap()
            .path()
    );
    let state = StateDocument::new(
        STATE_SCHEMA_VERSION,
        [],
        [global_state, project_state],
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(state.resources().len(), 2);
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
            "\"schema\": 2",
            "\"schema\": 1",
            1
        ))
        .is_err()
    );

    let mut old_shape: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    let resource = old_shape["resources"][0].as_object_mut().unwrap();
    let key = resource.remove("key").unwrap();
    resource.insert("resource_id".into(), key["id"].clone());
    assert!(serde_json::from_value::<StateDocument>(old_shape).is_err());
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

    let key = ResourceKey::new(ResourceId::new("skill:bad").unwrap(), Scope::Global);
    let wrong_owner = ManagedArtifactRecord::for_artifact(
        ResourceKey::new(ResourceId::new("skill:other").unwrap(), Scope::Global),
        ArtifactRole::DirectSkill,
        fingerprint(),
    )
    .unwrap();
    let wrong_owner_target = TargetResourceState::new(
        codex(),
        None,
        Provenance::Direct,
        Ownership::Skilltap,
        None,
        Some(wrong_owner),
        None,
        None,
        None,
        Timestamp::new(1, 0).unwrap(),
        None,
    )
    .unwrap();
    assert!(ResourceState::new(key.clone(), [wrong_owner_target]).is_err());
    assert!(
        TargetResourceState::new(
            codex(),
            None,
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
        key.clone(),
        ArtifactRole::MaterializedPlugin,
        fingerprint(),
    )
    .unwrap();
    assert!(
        TargetResourceState::new(
            codex(),
            None,
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
    let second = duplicate_path["resources"][0].clone();
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
    value["targets"][0]["binding"]["ownership"] = serde_json::json!("unmanaged");
    assert!(serde_json::from_value::<ResourceState>(value).is_err());

    let apply = valid.target(&codex()).unwrap().managed_artifact().unwrap();
    assert_eq!(apply.owner(), valid.key());
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

    let mut old_artifact_owner = serde_json::to_value(apply).unwrap();
    old_artifact_owner["owner"] = serde_json::json!("skill:review");
    assert!(serde_json::from_value::<ManagedArtifactRecord>(old_artifact_owner).is_err());
}

#[test]
fn daemon_run_record_round_trips_and_survives_state_updates() {
    let state = StateDocument::new(STATE_SCHEMA_VERSION, [], [], None, None, None).unwrap();
    let record = DaemonRunRecord::new(
        Timestamp::new(4, 0).unwrap(),
        DaemonRunResult::Pending,
        2,
        1,
        Some(EvidenceCode::new("daemon.update_failed").unwrap()),
    )
    .unwrap();
    let recorded = state.with_daemon_run(record.clone()).unwrap();
    assert_eq!(recorded.daemon_run(), Some(&record));
    let round_trip: StateDocument =
        serde_json::from_value(serde_json::to_value(&recorded).unwrap()).unwrap();
    assert_eq!(round_trip, recorded);
    let state_with_resource = StateDocument::new(
        STATE_SCHEMA_VERSION,
        [],
        [managed_resource("skill:review")],
        None,
        None,
        None,
    )
    .unwrap()
    .with_daemon_run(record.clone())
    .unwrap();
    let preserved = state_with_resource
        .with_available_revision(
            &ResourceKey::new(ResourceId::new("skill:review").unwrap(), Scope::Global),
            &codex(),
            None,
            Timestamp::new(5, 0).unwrap(),
        )
        .unwrap();
    assert_eq!(preserved.daemon_run(), Some(&record));
    let mut invalid = serde_json::to_value(record).unwrap();
    invalid["failure_code"] = serde_json::json!("daemon.raw_secret");
    assert!(serde_json::from_value::<DaemonRunRecord>(invalid).is_err());
}

#[test]
fn target_bindings_preserve_distinct_lifecycle_evidence_and_project_exactly() {
    let key = ResourceKey::new(ResourceId::new("skill:dual").unwrap(), Scope::Global);
    let codex = HarnessId::new("codex").unwrap();
    let claude = HarnessId::new("claude").unwrap();
    let apply = |id: &str, at| {
        ApplyRecord::new(
            Timestamp::new(at, 0).unwrap(),
            [
                OperationResult::new(OperationId::new(id).unwrap(), OperationOutcome::Applied)
                    .unwrap(),
            ],
        )
        .unwrap()
    };
    let native = TargetResourceState::new(
        codex.clone(),
        Some(NativeId::new("dual@catalog").unwrap()),
        Provenance::Native,
        Ownership::Harness,
        Some(
            Source::new(
                SourceKind::RemoteCatalog,
                SourceLocator::new("https://example.test/catalog.json").unwrap(),
                None,
            )
            .unwrap(),
        ),
        None,
        None,
        Some(ResolvedRevision::Native(NativeId::new("1.0.0").unwrap())),
        Some(ResolvedRevision::Native(NativeId::new("1.1.0").unwrap())),
        Timestamp::new(10, 0).unwrap(),
        Some(apply("codex:install", 11)),
    )
    .unwrap();
    let artifact =
        ManagedArtifactRecord::for_artifact(key.clone(), ArtifactRole::DirectSkill, fingerprint())
            .unwrap();
    let managed = TargetResourceState::new(
        claude.clone(),
        Some(NativeId::new("dual").unwrap()),
        Provenance::Direct,
        Ownership::Skilltap,
        Some(
            Source::new(
                SourceKind::Local,
                SourceLocator::new("/tmp/dual-skill").unwrap(),
                None,
            )
            .unwrap(),
        ),
        Some(artifact),
        Some(fingerprint()),
        None,
        None,
        Timestamp::new(20, 0).unwrap(),
        Some(apply("claude:install", 21)),
    )
    .unwrap();
    let resource = ResourceState::new(key.clone(), [native.clone(), managed.clone()]).unwrap();
    assert_eq!(resource.target(&codex), Some(&native));
    assert_eq!(resource.target(&claude), Some(&managed));

    let selected = HarnessSet::new([codex.clone()]).unwrap();
    let projected = resource.without_targets(&selected).unwrap().unwrap();
    assert_eq!(projected.targets().len(), 1);
    assert_eq!(projected.target(&claude), Some(&managed));
    assert!(projected.target(&codex).is_none());

    let encoded = serde_json::to_value(&resource).unwrap();
    let mut mismatch = encoded.clone();
    let binding_harness = mismatch["targets"][0]["binding"]["harness"]
        .as_str()
        .unwrap();
    mismatch["targets"][0]["target"] = if binding_harness == "codex" {
        serde_json::json!("claude")
    } else {
        serde_json::json!("codex")
    };
    assert!(serde_json::from_value::<ResourceState>(mismatch).is_err());
    let mut duplicate = encoded.clone();
    duplicate["targets"]
        .as_array_mut()
        .unwrap()
        .push(encoded["targets"][0].clone());
    assert!(serde_json::from_value::<ResourceState>(duplicate).is_err());
    let mut unknown = encoded;
    unknown["targets"][0]["binding"]["unexpected"] = serde_json::json!(true);
    assert!(serde_json::from_value::<ResourceState>(unknown).is_err());
}
