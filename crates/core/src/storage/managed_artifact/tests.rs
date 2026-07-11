#![cfg(unix)]

use std::{cell::Cell, collections::BTreeMap, fs, io, path::PathBuf};

use skilltap_test_support::TempRoot;

use super::*;
use crate::{
    domain::FingerprintAlgorithm,
    runtime::{DirectoryPathState, DirectoryPublishOutcome, DirectorySyncState, SystemFileSystem},
};

fn setup() -> (TempRoot, FileManagedArtifactRepository<'static>) {
    let temporary = TempRoot::new("skilltap-managed-artifacts-test").unwrap();
    let root = AbsolutePath::new(temporary.path().to_str().unwrap()).unwrap();
    let filesystem = Box::leak(Box::new(SystemFileSystem));
    let repository = FileManagedArtifactRepository::new(filesystem, root).unwrap();
    (temporary, repository)
}

fn owner(value: &str) -> ResourceId {
    ResourceId::new(value).unwrap()
}

fn fingerprint(byte: char) -> Fingerprint {
    Fingerprint::new(FingerprintAlgorithm::Sha256, byte.to_string().repeat(64)).unwrap()
}

fn skill_tree() -> ArtifactTree {
    ArtifactTree::new([
        ("SKILL.md", b"not semantically validated".to_vec()),
        ("scripts/run.sh", b"#!/bin/sh\nexit 0\n".to_vec()),
        ("references/guide.md", vec![0, 1, 2, 255]),
    ])
    .unwrap()
}

fn absolute(
    repository: &FileManagedArtifactRepository<'_>,
    path: &RelativeArtifactPath,
) -> PathBuf {
    PathBuf::from(repository.managed_root().as_str()).join(path.as_str())
}

#[test]
fn artifact_trees_validate_and_sort_complete_relative_files() {
    let tree = ArtifactTree::new([
        ("z/file", vec![3]),
        ("SKILL.md", vec![1]),
        ("a/nested/file", vec![2]),
    ])
    .unwrap();
    assert_eq!(
        tree.files()
            .keys()
            .map(RelativeArtifactPath::as_str)
            .collect::<Vec<_>>(),
        ["SKILL.md", "a/nested/file", "z/file"]
    );
    assert!(matches!(
        ArtifactTree::new(Vec::<(String, Vec<u8>)>::new()),
        Err(ArtifactTreeError::Empty)
    ));
    for invalid in ["", "/absolute", "../escape", "a/../b", "a//b", "./a"] {
        assert!(matches!(
            ArtifactTree::new([(invalid, Vec::new())]),
            Err(ArtifactTreeError::InvalidPath)
        ));
    }
    assert!(matches!(
        ArtifactTree::new([("same", vec![1]), ("same", vec![2])]),
        Err(ArtifactTreeError::DuplicatePath { .. })
    ));
    assert!(matches!(
        ArtifactTree::new([("file", vec![1]), ("file/child", vec![2])]),
        Err(ArtifactTreeError::FileIsAncestor { .. })
    ));
}

#[test]
fn whole_skill_directory_publishes_immutably_and_round_trips_exact_bytes() {
    let (_temporary, repository) = setup();
    let owner = owner("skill:review");
    let fingerprint = fingerprint('a');
    let tree = skill_tree();

    let first = repository
        .publish(&owner, ArtifactRole::DirectSkill, &fingerprint, &tree)
        .unwrap();
    let handle = match first {
        ArtifactPublication::Published(handle) => handle,
        ArtifactPublication::Existing(_) => panic!("first publication must create the tree"),
    };
    assert_eq!(handle.record().owner(), &owner);
    assert_eq!(handle.record().role(), ArtifactRole::DirectSkill);
    assert_eq!(handle.record().fingerprint(), Some(&fingerprint));
    assert_eq!(
        fs::read(absolute(&repository, handle.record().path()).join("SKILL.md")).unwrap(),
        b"not semantically validated"
    );

    let repeated = repository
        .publish(&owner, ArtifactRole::DirectSkill, &fingerprint, &tree)
        .unwrap();
    assert!(matches!(repeated, ArtifactPublication::Existing(_)));
    let loaded = repository.load(&owner, handle.record()).unwrap();
    assert_eq!(loaded.tree(), &tree);

    let changed = ArtifactTree::new([("SKILL.md", b"changed".to_vec())]).unwrap();
    let error = repository
        .publish(&owner, ArtifactRole::DirectSkill, &fingerprint, &changed)
        .unwrap_err();
    assert_eq!(error.failure(), ManagedArtifactFailure::Conflict);
    assert_eq!(
        repository.load(&owner, handle.record()).unwrap().tree(),
        &tree
    );
}

#[test]
fn backups_are_unique_exclusive_complete_trees() {
    let (_temporary, repository) = setup();
    let owner = owner("plugin:tools");
    let tree = skill_tree();
    let first = repository.backup(&owner, &tree).unwrap();
    let second = repository.backup(&owner, &tree).unwrap();
    assert_ne!(first.record().path(), second.record().path());
    assert_eq!(first.record().role(), ArtifactRole::Backup);
    assert_eq!(first.record().fingerprint(), None);
    assert_eq!(
        repository.load(&owner, first.record()).unwrap().tree(),
        &tree
    );
    assert_eq!(
        repository.load(&owner, second.record()).unwrap().tree(),
        &tree
    );
    repository.remove(&owner, &first).unwrap();
    assert!(repository.load(&owner, first.record()).is_err());
    assert_eq!(
        repository.load(&owner, second.record()).unwrap().tree(),
        &tree
    );

    let maximum_owner = ResourceId::new("a".repeat(256)).unwrap();
    let maximum = repository
        .publish(
            &maximum_owner,
            ArtifactRole::DirectSkill,
            &fingerprint('f'),
            &tree,
        )
        .unwrap();
    let maximum = match maximum {
        ArtifactPublication::Published(handle) => handle,
        ArtifactPublication::Existing(_) => unreachable!(),
    };
    let distinct_owner = ResourceId::new("b".repeat(256)).unwrap();
    let distinct = match repository
        .publish(
            &distinct_owner,
            ArtifactRole::DirectSkill,
            &fingerprint('f'),
            &tree,
        )
        .unwrap()
    {
        ArtifactPublication::Published(handle) => handle,
        ArtifactPublication::Existing(_) => unreachable!(),
    };
    assert!(maximum.record().path().as_str().len() <= 255);
    assert_ne!(maximum.record().path(), distinct.record().path());
}

#[test]
fn owner_path_and_loaded_inode_are_required_for_removal() {
    let (_temporary, repository) = setup();
    let owner = owner("skill:owned");
    let other = ResourceId::new("skill:other").unwrap();
    let fingerprint = fingerprint('b');
    let handle = match repository
        .publish(
            &owner,
            ArtifactRole::DirectSkill,
            &fingerprint,
            &skill_tree(),
        )
        .unwrap()
    {
        ArtifactPublication::Published(handle) => handle,
        ArtifactPublication::Existing(_) => unreachable!(),
    };
    assert_eq!(
        repository
            .load(&other, handle.record())
            .unwrap_err()
            .failure(),
        ManagedArtifactFailure::InvalidRecord
    );
    let wrong_path = ManagedArtifactRecord::new(
        owner.clone(),
        ArtifactRole::DirectSkill,
        RelativeArtifactPath::new("unowned-path").unwrap(),
        Some(fingerprint.clone()),
    );
    assert_eq!(
        repository.load(&owner, &wrong_path).unwrap_err().failure(),
        ManagedArtifactFailure::InvalidRecord
    );

    let loaded = repository.load(&owner, handle.record()).unwrap();
    let path = absolute(&repository, handle.record().path());
    let displaced = path.with_extension("displaced");
    fs::rename(&path, &displaced).unwrap();
    fs::create_dir(&path).unwrap();
    fs::write(path.join("victim"), b"preserve").unwrap();
    let error = repository.remove(&owner, loaded.handle()).unwrap_err();
    assert_eq!(error.failure(), ManagedArtifactFailure::Runtime);
    assert_eq!(fs::read(path.join("victim")).unwrap(), b"preserve");
}

#[test]
fn live_and_dangling_managed_or_owned_ancestor_links_are_never_followed() {
    for dangling in [false, true] {
        let (temporary, repository) = setup();
        let outside = temporary.join("outside");
        fs::create_dir(&outside).unwrap();
        let managed = PathBuf::from(repository.managed_root().as_str());
        let target = if dangling {
            temporary.join("missing")
        } else {
            outside.clone()
        };
        std::os::unix::fs::symlink(&target, &managed).unwrap();
        assert!(
            repository
                .publish(
                    &owner("skill:linked"),
                    ArtifactRole::DirectSkill,
                    &fingerprint('c'),
                    &skill_tree(),
                )
                .is_err()
        );
        assert!(fs::read_dir(&outside).unwrap().next().is_none());
    }

    let (temporary, repository) = setup();
    let managed = PathBuf::from(repository.managed_root().as_str());
    fs::create_dir_all(&managed).unwrap();
    let outside = temporary.join("outside");
    fs::create_dir(&outside).unwrap();
    let owner = owner("skill:ancestor");
    let fingerprint = fingerprint('d');
    let destination = artifact_path(&owner, ArtifactRole::DirectSkill, &fingerprint).unwrap();
    std::os::unix::fs::symlink(&outside, absolute(&repository, &destination)).unwrap();
    assert!(
        repository
            .publish(
                &owner,
                ArtifactRole::DirectSkill,
                &fingerprint,
                &skill_tree(),
            )
            .is_err()
    );
    assert!(fs::read_dir(outside).unwrap().next().is_none());
}

struct PartialFileSystem;

impl DirectoryTreeFileSystem for PartialFileSystem {
    fn publish_tree_no_follow(
        &self,
        managed_root: &AbsolutePath,
        _destination: &RelativeArtifactPath,
        _files: &BTreeMap<RelativeArtifactPath, Vec<u8>>,
    ) -> Result<DirectoryPublishOutcome, RuntimeError> {
        Err(RuntimeError::PartialDirectoryPublication {
            path: managed_root.clone(),
            identity: Some(DirectoryIdentity::new(7, 11)),
            presence: DirectoryPathState::Present,
            parent_sync: DirectorySyncState::Synced,
            source: io::Error::other("publish failed"),
            cleanup: io::Error::other("cleanup failed"),
        })
    }

    fn load_tree_no_follow(
        &self,
        _managed_root: &AbsolutePath,
        _destination: &RelativeArtifactPath,
    ) -> Result<(DirectoryIdentity, BTreeMap<RelativeArtifactPath, Vec<u8>>), RuntimeError> {
        unreachable!()
    }

    fn remove_tree_no_follow(
        &self,
        _managed_root: &AbsolutePath,
        _destination: &RelativeArtifactPath,
        _expected: DirectoryIdentity,
    ) -> Result<DirectoryIdentity, RuntimeError> {
        unreachable!()
    }
}

#[test]
fn cleanup_failure_reports_exact_owned_residual_context() {
    let repository = FileManagedArtifactRepository::new(
        &PartialFileSystem,
        AbsolutePath::new("/machine/skilltap").unwrap(),
    )
    .unwrap();
    let owner = owner("skill:residual");
    let error = repository
        .publish(
            &owner,
            ArtifactRole::DirectSkill,
            &fingerprint('e'),
            &skill_tree(),
        )
        .unwrap_err();
    assert_eq!(error.failure(), ManagedArtifactFailure::PartialPublication);
    let residual = error.residual().unwrap();
    assert_eq!(residual.owner(), &owner);
    assert_eq!(residual.path(), error.path().unwrap());
    assert_eq!(residual.identity(), Some(DirectoryIdentity::new(7, 11)));
    assert_eq!(residual.presence(), DirectoryPathState::Present);
    assert_eq!(residual.parent_sync(), DirectorySyncState::Synced);
}

struct OccupiedFileSystem {
    occupied: usize,
    calls: Cell<usize>,
    load_fails: bool,
}

impl OccupiedFileSystem {
    fn new(occupied: usize, load_fails: bool) -> Self {
        Self {
            occupied,
            calls: Cell::new(0),
            load_fails,
        }
    }
}

impl DirectoryTreeFileSystem for OccupiedFileSystem {
    fn publish_tree_no_follow(
        &self,
        _managed_root: &AbsolutePath,
        _destination: &RelativeArtifactPath,
        _files: &BTreeMap<RelativeArtifactPath, Vec<u8>>,
    ) -> Result<DirectoryPublishOutcome, RuntimeError> {
        let call = self.calls.get() + 1;
        self.calls.set(call);
        if call <= self.occupied {
            Ok(DirectoryPublishOutcome::AlreadyExists)
        } else {
            Ok(DirectoryPublishOutcome::Published(DirectoryIdentity::new(
                3,
                call as u64,
            )))
        }
    }

    fn load_tree_no_follow(
        &self,
        managed_root: &AbsolutePath,
        _destination: &RelativeArtifactPath,
    ) -> Result<(DirectoryIdentity, BTreeMap<RelativeArtifactPath, Vec<u8>>), RuntimeError> {
        if self.load_fails {
            Err(RuntimeError::FileSystem {
                action: crate::runtime::FileSystemAction::Read,
                path: managed_root.clone(),
                source: io::Error::other("occupied path cannot be loaded"),
            })
        } else {
            Ok((
                DirectoryIdentity::new(2, 2),
                BTreeMap::from([(RelativeArtifactPath::new("different").unwrap(), vec![9])]),
            ))
        }
    }

    fn remove_tree_no_follow(
        &self,
        _managed_root: &AbsolutePath,
        _destination: &RelativeArtifactPath,
        _expected: DirectoryIdentity,
    ) -> Result<DirectoryIdentity, RuntimeError> {
        unreachable!()
    }
}

#[test]
fn occupied_publish_errors_keep_the_publish_action_for_load_and_compare() {
    let owner = owner("skill:occupied");
    for load_fails in [false, true] {
        let filesystem = OccupiedFileSystem::new(usize::MAX, load_fails);
        let repository = FileManagedArtifactRepository::new(
            &filesystem,
            AbsolutePath::new("/machine/skilltap").unwrap(),
        )
        .unwrap();
        let error = repository
            .publish(
                &owner,
                ArtifactRole::DirectSkill,
                &fingerprint('a'),
                &skill_tree(),
            )
            .unwrap_err();
        assert_eq!(error.action(), ManagedArtifactAction::Publish);
        assert_eq!(
            error.failure(),
            if load_fails {
                ManagedArtifactFailure::Runtime
            } else {
                ManagedArtifactFailure::Conflict
            }
        );
    }
}

#[test]
fn backup_retries_stale_occupied_paths_and_exhausts_with_backup_context() {
    let owner = owner("skill:backup-collision");
    let filesystem = OccupiedFileSystem::new(2, false);
    let repository = FileManagedArtifactRepository::new(
        &filesystem,
        AbsolutePath::new("/machine/skilltap").unwrap(),
    )
    .unwrap();
    repository.backup(&owner, &skill_tree()).unwrap();
    assert_eq!(filesystem.calls.get(), 3);

    let exhausted = OccupiedFileSystem::new(usize::MAX, true);
    let repository = FileManagedArtifactRepository::new(
        &exhausted,
        AbsolutePath::new("/machine/skilltap").unwrap(),
    )
    .unwrap();
    let error = repository.backup(&owner, &skill_tree()).unwrap_err();
    assert_eq!(exhausted.calls.get(), 32);
    assert_eq!(error.action(), ManagedArtifactAction::Backup);
    assert_eq!(error.failure(), ManagedArtifactFailure::Conflict);
}
