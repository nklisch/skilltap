struct PartialFileSystem;

impl DirectoryTreeFileSystem for PartialFileSystem {
    fn publish_tree_no_follow(
        &self,
        managed_root: &AbsolutePath,
        _destination: &RelativeArtifactPath,
        _files: &BTreeMap<RelativeArtifactPath, ArtifactFile>,
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
    ) -> Result<(DirectoryIdentity, BTreeMap<RelativeArtifactPath, ArtifactFile>), RuntimeError> {
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

struct PartialRemovalFileSystem;

impl DirectoryTreeFileSystem for PartialRemovalFileSystem {
    fn publish_tree_no_follow(
        &self,
        _managed_root: &AbsolutePath,
        _destination: &RelativeArtifactPath,
        _files: &BTreeMap<RelativeArtifactPath, ArtifactFile>,
    ) -> Result<DirectoryPublishOutcome, RuntimeError> {
        unreachable!()
    }

    fn load_tree_no_follow(
        &self,
        _managed_root: &AbsolutePath,
        _destination: &RelativeArtifactPath,
    ) -> Result<(DirectoryIdentity, BTreeMap<RelativeArtifactPath, ArtifactFile>), RuntimeError> {
        unreachable!()
    }

    fn remove_tree_no_follow(
        &self,
        managed_root: &AbsolutePath,
        _destination: &RelativeArtifactPath,
        expected: DirectoryIdentity,
    ) -> Result<DirectoryIdentity, RuntimeError> {
        Err(RuntimeError::PartialDirectoryRemoval {
            path: managed_root.clone(),
            expected,
            observed: Some(expected),
            presence: DirectoryPathState::Present,
            content: DirectoryContentState::Partial,
            parent_sync: DirectorySyncState::NotRequired,
            source: io::Error::other("remove failed after one entry"),
        })
    }
}

#[test]
fn partial_removal_maps_to_safe_owned_residual_context() {
    let repository = FileManagedArtifactRepository::new(
        &PartialRemovalFileSystem,
        AbsolutePath::new("/machine/skilltap").unwrap(),
    )
    .unwrap();
    let owner = owner("skill:partial-removal");
    let record = ManagedArtifactRecord::for_artifact(
        owner.clone(),
        ArtifactRole::DirectSkill,
        fingerprint('f'),
    )
    .unwrap();
    let expected = DirectoryIdentity::new(17, 23);
    let handle = ManagedArtifactHandle {
        record,
        identity: expected,
    };

    let error = repository.remove(&owner, &handle).unwrap_err();
    assert_eq!(error.failure(), ManagedArtifactFailure::PartialRemoval);
    assert!(error.residual().is_none());
    let residual = error.removal_residual().unwrap();
    assert_eq!(residual.owner(), &owner);
    assert_eq!(residual.path(), error.path().unwrap());
    assert_eq!(residual.expected_identity(), expected);
    assert_eq!(residual.observed_identity(), Some(expected));
    assert_eq!(residual.presence(), DirectoryPathState::Present);
    assert_eq!(residual.content(), DirectoryContentState::Partial);
    assert_eq!(residual.parent_sync(), DirectorySyncState::NotRequired);
}

struct CountingFileSystem {
    calls: Cell<usize>,
}

impl DirectoryTreeFileSystem for CountingFileSystem {
    fn publish_tree_no_follow(
        &self,
        _managed_root: &AbsolutePath,
        _destination: &RelativeArtifactPath,
        _files: &BTreeMap<RelativeArtifactPath, ArtifactFile>,
    ) -> Result<DirectoryPublishOutcome, RuntimeError> {
        self.calls.set(self.calls.get() + 1);
        unreachable!("owner mismatch must fail before publication")
    }

    fn load_tree_no_follow(
        &self,
        _managed_root: &AbsolutePath,
        _destination: &RelativeArtifactPath,
    ) -> Result<(DirectoryIdentity, BTreeMap<RelativeArtifactPath, ArtifactFile>), RuntimeError> {
        self.calls.set(self.calls.get() + 1);
        unreachable!("owner mismatch must fail before loading")
    }

    fn remove_tree_no_follow(
        &self,
        _managed_root: &AbsolutePath,
        _destination: &RelativeArtifactPath,
        _expected: DirectoryIdentity,
    ) -> Result<DirectoryIdentity, RuntimeError> {
        self.calls.set(self.calls.get() + 1);
        unreachable!("owner mismatch must fail before removal")
    }
}

#[test]
fn owner_mismatch_fails_before_filesystem_io() {
    let filesystem = CountingFileSystem {
        calls: Cell::new(0),
    };
    let repository = FileManagedArtifactRepository::new(
        &filesystem,
        AbsolutePath::new("/machine/skilltap").unwrap(),
    )
    .unwrap();
    let id = ResourceId::new("skill:shared").unwrap();
    let global = ResourceKey::new(id.clone(), Scope::Global);
    let project = ResourceKey::new(
        id,
        Scope::Project(AbsolutePath::new("/work/project").unwrap()),
    );
    let record =
        ManagedArtifactRecord::for_artifact(global, ArtifactRole::DirectSkill, fingerprint('f'))
            .unwrap();

    let load_error = repository.load(&project, &record).unwrap_err();
    assert_eq!(load_error.failure(), ManagedArtifactFailure::InvalidRecord);
    assert_eq!(load_error.owner(), &project);
    assert_eq!(filesystem.calls.get(), 0);

    let handle = ManagedArtifactHandle {
        record,
        identity: DirectoryIdentity::new(17, 23),
    };
    let remove_error = repository.remove(&project, &handle).unwrap_err();
    assert_eq!(
        remove_error.failure(),
        ManagedArtifactFailure::InvalidRecord
    );
    assert_eq!(remove_error.owner(), &project);
    assert_eq!(filesystem.calls.get(), 0);
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
        _files: &BTreeMap<RelativeArtifactPath, ArtifactFile>,
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
    ) -> Result<(DirectoryIdentity, BTreeMap<RelativeArtifactPath, ArtifactFile>), RuntimeError> {
        if self.load_fails {
            Err(RuntimeError::FileSystem {
                action: crate::runtime::FileSystemAction::Read,
                path: managed_root.clone(),
                source: io::Error::other("occupied path cannot be loaded"),
            })
        } else {
            Ok((
                DirectoryIdentity::new(2, 2),
                BTreeMap::from([(
                    RelativeArtifactPath::new("different").unwrap(),
                    ArtifactFile::new(vec![9], false),
                )]),
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
