#![cfg(unix)]

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use skilltap_core::{
    domain::{
        AbsolutePath, ComponentGraph, DesiredOrigin, DesiredResource, Fingerprint,
        FingerprintAlgorithm, HarnessId, HarnessSet, Ownership, Provenance, ResourceId,
        ResourceKey, ResourceKind, Scope, UpdateIntent,
    },
    runtime::SystemFileSystem,
    storage::{
        ArtifactPublication, ArtifactRole, ArtifactTree, ConfigDocument, ConfigRepository,
        DocumentKind, DocumentState, FileConfigRepository, FileInventoryRepository,
        FileManagedArtifactRepository, FileStateRepository, INVENTORY_SCHEMA_VERSION,
        InventoryDocument, InventoryRepository, ManagedArtifactFailure, ManagedArtifactRepository,
        ResourceState, STATE_SCHEMA_VERSION, StateDocument, StateRepository, StorageFailure,
        Timestamp,
    },
};
use skilltap_test_support::TempRoot;

const AUTH_SENTINEL: &str = "sk-test-auth-material-must-not-persist";

struct Fixture {
    _temporary: TempRoot,
    root: AbsolutePath,
    owner: ResourceKey,
    fingerprint: Fingerprint,
    tree: ArtifactTree,
    config: ConfigDocument,
    inventory: InventoryDocument,
}

impl Fixture {
    fn new() -> Self {
        let temporary = TempRoot::new("skilltap-storage-integration").unwrap();
        let root = AbsolutePath::new(temporary.join("skilltap").to_str().unwrap()).unwrap();
        let owner = ResourceKey::new(ResourceId::new("skill:integration").unwrap(), Scope::Global);
        let fingerprint = Fingerprint::new(FingerprintAlgorithm::Sha256, "a".repeat(64)).unwrap();
        let tree = ArtifactTree::new([
            ("SKILL.md", b"---\nname: integration\n---\n".to_vec()),
            ("references/guide.md", b"complete guide\n".to_vec()),
            ("scripts/run.sh", b"#!/bin/sh\nexit 0\n".to_vec()),
        ])
        .unwrap();
        let desired = DesiredResource::new(
            owner.clone(),
            ResourceKind::StandaloneSkill,
            HarnessSet::new([HarnessId::new("codex").unwrap()]).unwrap(),
            DesiredOrigin::Direct,
            None,
            UpdateIntent::Track,
            ComponentGraph::new([]).unwrap(),
            BTreeMap::new(),
            BTreeMap::new(),
            BTreeSet::new(),
        )
        .unwrap();
        let inventory = InventoryDocument::new(INVENTORY_SCHEMA_VERSION, [], [desired]).unwrap();
        Self {
            _temporary: temporary,
            root,
            owner,
            fingerprint,
            tree,
            config: ConfigDocument::defaults(),
            inventory,
        }
    }

    fn config_repository(&self) -> FileConfigRepository<'static> {
        FileConfigRepository::new(&SystemFileSystem, self.root.clone()).unwrap()
    }

    fn inventory_repository(&self) -> FileInventoryRepository<'static> {
        FileInventoryRepository::new(&SystemFileSystem, self.root.clone()).unwrap()
    }

    fn state_repository(&self) -> FileStateRepository<'static> {
        FileStateRepository::new(&SystemFileSystem, self.root.clone()).unwrap()
    }

    fn artifact_repository(&self) -> FileManagedArtifactRepository<'static> {
        FileManagedArtifactRepository::new(&SystemFileSystem, self.root.clone()).unwrap()
    }

    fn empty_state(&self) -> StateDocument {
        StateDocument::new(STATE_SCHEMA_VERSION, [], [], None, None, None).unwrap()
    }

    fn initialize_documents(&self) -> StateDocument {
        let state = self.empty_state();
        self.config_repository().replace(&self.config).unwrap();
        self.inventory_repository()
            .replace(&self.inventory)
            .unwrap();
        self.state_repository().replace(&state).unwrap();
        state
    }

    fn publish(&self) -> skilltap_core::storage::ManagedArtifactHandle {
        match self
            .artifact_repository()
            .publish(
                &self.owner,
                ArtifactRole::DirectSkill,
                &self.fingerprint,
                &self.tree,
            )
            .unwrap()
        {
            ArtifactPublication::Published(handle) | ArtifactPublication::Existing(handle) => {
                handle
            }
        }
    }

    fn referenced_state(
        &self,
        record: skilltap_core::storage::ManagedArtifactRecord,
    ) -> StateDocument {
        let target = skilltap_core::storage::TargetResourceState::new(
            HarnessId::new("codex").unwrap(),
            None,
            Provenance::Direct,
            Ownership::Skilltap,
            None,
            Some(record),
            Some(self.fingerprint.clone()),
            None,
            None,
            Timestamp::new(10, 25).unwrap(),
            None,
        )
        .unwrap();
        let resource = ResourceState::new(self.owner.clone(), [target]).unwrap();
        StateDocument::new(
            STATE_SCHEMA_VERSION,
            [],
            [resource],
            None,
            Some(Timestamp::new(10, 25).unwrap()),
            Some(Timestamp::new(11, 0).unwrap()),
        )
        .unwrap()
    }

    fn root_path(&self) -> &Path {
        Path::new(self.root.as_str())
    }
}

#[test]
fn first_use_and_repeated_writes_have_only_documented_idempotent_surfaces() {
    let fixture = Fixture::new();
    let config = fixture.config_repository();
    let inventory = fixture.inventory_repository();
    let state = fixture.state_repository();
    let artifacts = fixture.artifact_repository();
    assert!(!fixture.root_path().exists());

    assert_eq!(config.load().unwrap(), DocumentState::Missing);
    assert_eq!(inventory.load().unwrap(), DocumentState::Missing);
    assert_eq!(state.load().unwrap(), DocumentState::Missing);
    assert!(!fixture.root_path().exists());
    assert_eq!(
        artifacts.managed_root().as_str(),
        format!("{}/managed", fixture.root)
    );
    assert!(!fixture.root_path().exists());

    let empty_state = fixture.initialize_documents();
    let handle = fixture.publish();
    assert_eq!(
        top_level_entries(fixture.root_path()),
        ["config.toml", "inventory.toml", "managed", "state.json"]
    );
    assert_eq!(
        config.load().unwrap(),
        DocumentState::Present(fixture.config.clone())
    );
    assert_eq!(
        inventory.load().unwrap(),
        DocumentState::Present(fixture.inventory.clone())
    );
    assert_eq!(
        state.load().unwrap(),
        DocumentState::Present(empty_state.clone())
    );
    assert_eq!(
        artifacts
            .load(&fixture.owner, handle.record())
            .unwrap()
            .tree(),
        &fixture.tree
    );
    let before = snapshot(fixture.root_path());

    config.replace(&fixture.config).unwrap();
    inventory.replace(&fixture.inventory).unwrap();
    state.replace(&empty_state).unwrap();
    assert!(matches!(
        artifacts
            .publish(
                &fixture.owner,
                ArtifactRole::DirectSkill,
                &fixture.fingerprint,
                &fixture.tree,
            )
            .unwrap(),
        ArtifactPublication::Existing(_)
    ));
    assert_eq!(snapshot(fixture.root_path()), before);
    assert_owned_files_exclude_authentication(fixture.root_path());
}

#[test]
fn corrupt_documents_fail_independently_without_masking_or_mutating_other_stores() {
    let fixture = Fixture::new();
    let empty_state = fixture.initialize_documents();
    let handle = fixture.publish();
    let config_path = fixture.root_path().join("config.toml");
    let inventory_path = fixture.root_path().join("inventory.toml");
    let state_path = fixture.root_path().join("state.json");

    for (kind, path, corrupt) in [
        (
            DocumentKind::Config,
            config_path.as_path(),
            b"schema = [".as_slice(),
        ),
        (
            DocumentKind::Inventory,
            inventory_path.as_path(),
            b"schema = 1\nunknown = true\n".as_slice(),
        ),
        (
            DocumentKind::State,
            state_path.as_path(),
            br#"{"schema":99}"#.as_slice(),
        ),
    ] {
        let original = fs::read(path).unwrap();
        fs::write(path, corrupt).unwrap();
        let error = match kind {
            DocumentKind::Config => fixture.config_repository().load().unwrap_err(),
            DocumentKind::Inventory => fixture.inventory_repository().load().unwrap_err(),
            DocumentKind::State => fixture.state_repository().load().unwrap_err(),
        };
        assert_eq!(error.document(), kind);
        assert!(matches!(
            error.failure(),
            StorageFailure::Malformed
                | StorageFailure::Invalid
                | StorageFailure::UnsupportedSchema { .. }
        ));
        assert_eq!(
            fixture
                .artifact_repository()
                .load(&fixture.owner, handle.record())
                .unwrap()
                .tree(),
            &fixture.tree
        );
        match kind {
            DocumentKind::Config => {
                assert_eq!(
                    fixture.inventory_repository().load().unwrap(),
                    DocumentState::Present(fixture.inventory.clone())
                );
                assert_eq!(
                    fixture.state_repository().load().unwrap(),
                    DocumentState::Present(empty_state.clone())
                );
            }
            DocumentKind::Inventory => {
                assert_eq!(
                    fixture.config_repository().load().unwrap(),
                    DocumentState::Present(fixture.config.clone())
                );
                assert_eq!(
                    fixture.state_repository().load().unwrap(),
                    DocumentState::Present(empty_state.clone())
                );
            }
            DocumentKind::State => {
                assert_eq!(
                    fixture.config_repository().load().unwrap(),
                    DocumentState::Present(fixture.config.clone())
                );
                assert_eq!(
                    fixture.inventory_repository().load().unwrap(),
                    DocumentState::Present(fixture.inventory.clone())
                );
            }
        }
        assert_eq!(fs::read(path).unwrap(), corrupt);
        fs::write(path, original).unwrap();
    }

    assert_owned_files_exclude_authentication(fixture.root_path());
}

#[test]
fn complete_tree_precedes_atomic_state_reference_and_failed_publish_preserves_both() {
    let fixture = Fixture::new();
    let empty_state = fixture.initialize_documents();
    let handle = fixture.publish();
    let referenced = fixture.referenced_state(handle.record().clone());
    let referenced: StateDocument =
        serde_json::from_slice(&serde_json::to_vec_pretty(&referenced).unwrap()).unwrap();
    let round_tripped_record = referenced
        .resources()
        .get(&fixture.owner)
        .unwrap()
        .target(&HarnessId::new("codex").unwrap())
        .unwrap()
        .managed_artifact()
        .unwrap();
    assert_eq!(round_tripped_record, handle.record());
    assert_eq!(
        fixture
            .artifact_repository()
            .load(&fixture.owner, round_tripped_record)
            .unwrap()
            .tree(),
        &fixture.tree
    );
    let reader_root = fixture.root.clone();
    let reader_owner = fixture.owner.clone();
    let reader_tree = fixture.tree.clone();
    let running = Arc::new(AtomicBool::new(true));
    let reader_running = Arc::clone(&running);
    let (sender, receiver) = mpsc::channel();

    let reader = thread::spawn(move || {
        while reader_running.load(Ordering::Relaxed) {
            let state_repository =
                FileStateRepository::new(&SystemFileSystem, reader_root.clone()).unwrap();
            let state = match state_repository.load() {
                Ok(DocumentState::Present(state)) => state,
                Ok(DocumentState::Missing) => panic!("initialized state must remain present"),
                Err(error) if matches!(error.failure(), StorageFailure::Runtime) => continue,
                Err(error) => panic!("state reader observed invalid contents: {error}"),
            };
            if let Some(resource) = state.resources().get(&reader_owner) {
                let record = resource
                    .target(&HarnessId::new("codex").unwrap())
                    .expect("referenced state carries target binding")
                    .managed_artifact()
                    .expect("referenced state carries artifact record");
                let artifacts =
                    FileManagedArtifactRepository::new(&SystemFileSystem, reader_root.clone())
                        .unwrap();
                let loaded = artifacts.load(&reader_owner, record).unwrap();
                assert_eq!(loaded.tree(), &reader_tree);
                sender.send(()).unwrap();
                return;
            }
            assert!(state.resources().is_empty());
        }
    });

    assert_eq!(
        fixture.state_repository().load().unwrap(),
        DocumentState::Present(empty_state)
    );
    fixture.state_repository().replace(&referenced).unwrap();
    receiver
        .recv_timeout(Duration::from_secs(2))
        .expect("reader must observe a complete referenced tree");
    running.store(false, Ordering::Relaxed);
    reader.join().unwrap();

    let state_before_failure = fs::read(fixture.root_path().join("state.json")).unwrap();
    let changed = ArtifactTree::new([("SKILL.md", b"incomplete replacement".to_vec())]).unwrap();
    let error = fixture
        .artifact_repository()
        .publish(
            &fixture.owner,
            ArtifactRole::DirectSkill,
            &fixture.fingerprint,
            &changed,
        )
        .unwrap_err();
    assert_eq!(error.failure(), ManagedArtifactFailure::Conflict);
    assert_eq!(
        fs::read(fixture.root_path().join("state.json")).unwrap(),
        state_before_failure
    );
    assert_eq!(
        fixture
            .artifact_repository()
            .load(&fixture.owner, handle.record())
            .unwrap()
            .tree(),
        &fixture.tree
    );
    assert_eq!(
        fixture.state_repository().load().unwrap(),
        DocumentState::Present(referenced)
    );
    assert_owned_files_exclude_authentication(fixture.root_path());
}

fn top_level_entries(root: &Path) -> Vec<String> {
    let mut entries = fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect::<Vec<_>>();
    entries.sort();
    entries
}

fn snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn visit(root: &Path, path: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
        let mut entries = fs::read_dir(path)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        entries.sort();
        for entry in entries {
            if entry.is_dir() {
                visit(root, &entry, files);
            } else {
                let relative = entry
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                files.insert(relative, fs::read(entry).unwrap());
            }
        }
    }

    let mut files = BTreeMap::new();
    visit(root, root, &mut files);
    files
}

fn assert_owned_files_exclude_authentication(root: &Path) {
    for (path, contents) in snapshot(root) {
        assert!(
            !contents
                .windows(AUTH_SENTINEL.len())
                .any(|window| window == AUTH_SENTINEL.as_bytes()),
            "authentication sentinel leaked into {path}"
        );
    }
}

#[cfg(unix)]
#[test]
fn persisted_documents_and_managed_artifacts_are_private() {
    let fixture = Fixture::new();
    fixture.initialize_documents();

    let root_mode = fs::metadata(fixture.root_path())
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(root_mode, 0o700);
    for document in ["config.toml", "inventory.toml", "state.json"] {
        let path = fixture.root_path().join(document);
        let mode = fs::metadata(path).unwrap().permissions().mode() & 0o777;
        assert_eq!(
            mode, 0o600,
            "document {document} must be user-readable only"
        );
    }

    let handle = fixture.publish();
    let managed = fixture.root_path().join("managed");
    assert_eq!(
        fs::metadata(&managed).unwrap().permissions().mode() & 0o777,
        0o700
    );
    let artifact = managed.join(handle.record().path().as_str());
    assert_eq!(
        fs::metadata(&artifact).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(artifact.join("SKILL.md"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
}
