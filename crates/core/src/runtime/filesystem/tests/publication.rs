struct InjectedPublication {
    fail_publish: bool,
    fail_remove_calls: BTreeSet<usize>,
    fail_sync_calls: BTreeSet<usize>,
    remove_calls: Cell<usize>,
    sync_calls: Cell<usize>,
}

impl InjectedPublication {
    fn new() -> Self {
        Self {
            fail_publish: false,
            fail_remove_calls: BTreeSet::new(),
            fail_sync_calls: BTreeSet::new(),
            remove_calls: Cell::new(0),
            sync_calls: Cell::new(0),
        }
    }
}

impl Publication for InjectedPublication {
    fn publish_no_clobber(&self, temporary: &Path, destination: &Path) -> io::Result<()> {
        if self.fail_publish {
            Err(io::Error::other("injected publication failure"))
        } else {
            fs::hard_link(temporary, destination)
        }
    }

    fn remove(&self, path: &Path) -> io::Result<()> {
        let call = self.remove_calls.get() + 1;
        self.remove_calls.set(call);
        if self.fail_remove_calls.contains(&call) {
            Err(io::Error::other("injected removal failure"))
        } else {
            fs::remove_file(path)
        }
    }

    fn sync_parent(&self, destination: &Path) -> io::Result<()> {
        let call = self.sync_calls.get() + 1;
        self.sync_calls.set(call);
        if self.fail_sync_calls.contains(&call) {
            Err(io::Error::other("injected directory sync failure"))
        } else {
            sync_parent_io(destination)
        }
    }
}

#[test]
fn atomic_write_replaces_whole_contents_and_cleans_failed_temporaries() {
    let temporary = TempDirectory::new();
    let path = temporary.path("state.json");
    let filesystem = SystemFileSystem;
    filesystem.atomic_write(&path, b"old").unwrap();
    filesystem.atomic_write(&path, b"new-complete").unwrap();
    assert_eq!(filesystem.read(&path).unwrap(), b"new-complete");

    let error = atomic_write_with(&path, |file| {
        file.write_all(b"partial")?;
        Err(io::Error::other("injected failure"))
    })
    .unwrap_err();
    assert_eq!(error.boundary(), super::super::RuntimeBoundary::FileSystem);
    assert_eq!(filesystem.read(&path).unwrap(), b"new-complete");
    assert_eq!(fs::read_dir(&temporary.0).unwrap().count(), 1);
}

#[test]
fn concurrent_readers_observe_only_old_or_new_complete_files() {
    let temporary = TempDirectory::new();
    let path = temporary.path("inventory.toml");
    let filesystem = SystemFileSystem;
    let old = vec![b'a'; 256 * 1024];
    let new = vec![b'b'; 256 * 1024];
    filesystem.atomic_write(&path, &old).unwrap();
    let path_for_reader = path.clone();
    let old_for_reader = old.clone();
    let new_for_reader = new.clone();
    let running = Arc::new(AtomicBool::new(true));
    let reader_running = Arc::clone(&running);
    let reader = thread::spawn(move || {
        while reader_running.load(Ordering::Relaxed) {
            let observed = fs::read(path_for_reader.as_str()).unwrap();
            assert!(observed == old_for_reader || observed == new_for_reader);
        }
    });
    filesystem.atomic_write(&path, &new).unwrap();
    running.store(false, Ordering::Relaxed);
    reader.join().unwrap();
}

#[test]
fn recoverable_copy_never_overwrites_and_rejects_symlink_sources() {
    let temporary = TempDirectory::new();
    let filesystem = SystemFileSystem;
    let source = temporary.path("AGENTS.md");
    let backup = temporary.path("AGENTS.md.backup");
    filesystem.atomic_write(&source, b"instructions").unwrap();
    filesystem.copy_recoverable(&source, &backup).unwrap();
    assert_eq!(filesystem.read(&backup).unwrap(), b"instructions");
    assert!(filesystem.copy_recoverable(&source, &backup).is_err());
    assert_eq!(filesystem.read(&backup).unwrap(), b"instructions");

    let link = temporary.path("link");
    filesystem
        .create_relative_symlink(&RelativeSymlinkTarget::new("AGENTS.md").unwrap(), &link)
        .unwrap();
    assert!(matches!(
        filesystem.copy_recoverable(&link, &temporary.path("link.backup")),
        Err(RuntimeError::UnsafeSymlink { .. })
    ));
}

#[test]
fn recoverable_copy_is_atomic_no_clobber_for_concurrent_readers() {
    let temporary = TempDirectory::new();
    let filesystem = SystemFileSystem;
    let source = temporary.path("source");
    let destination = temporary.path("backup");
    let complete = vec![b'x'; 512 * 1024];
    filesystem.atomic_write(&source, &complete).unwrap();

    let destination_for_reader = destination.clone();
    let complete_for_reader = complete.clone();
    let running = Arc::new(AtomicBool::new(true));
    let reader_running = Arc::clone(&running);
    let reader = thread::spawn(move || {
        while reader_running.load(Ordering::Relaxed) {
            match fs::read(destination_for_reader.as_str()) {
                Ok(observed) => assert_eq!(observed, complete_for_reader),
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => panic!("unexpected backup read error: {error}"),
            }
        }
    });
    filesystem.copy_recoverable(&source, &destination).unwrap();
    running.store(false, Ordering::Relaxed);
    reader.join().unwrap();
    assert_eq!(filesystem.read(&destination).unwrap(), complete);
}

#[test]
fn clean_rollback_leaves_no_residual() {
    let temporary = TempDirectory::new();
    let filesystem = SystemFileSystem;
    let source = publication_source(&temporary);

    let cleaned_destination = temporary.path("cleaned-backup");
    let mut cleaned = InjectedPublication::new();
    cleaned.fail_sync_calls.insert(1);
    let cleaned_error =
        copy_recoverable_with(&source, &cleaned_destination, &cleaned, || {}).unwrap_err();
    assert!(matches!(cleaned_error, RuntimeError::FileSystem { .. }));
    assert_eq!(
        filesystem.inspect(&cleaned_destination).unwrap().kind(),
        FileKind::Missing
    );
}

#[test]
fn prepublication_failure_reports_temporary_residual_without_sync() {
    let temporary = TempDirectory::new();
    let source = publication_source(&temporary);

    let prepublication_destination = temporary.path("prepublication-backup");
    let mut prepublication = InjectedPublication::new();
    prepublication.fail_publish = true;
    prepublication.fail_remove_calls.insert(1);
    let prepublication_error =
        copy_recoverable_with(&source, &prepublication_destination, &prepublication, || {})
            .unwrap_err();
    let prepublication_residuals = partial_residuals(&prepublication_error);
    assert_eq!(
        residual_roles(prepublication_residuals),
        BTreeSet::from([PublicationResidualRole::Temporary])
    );
    assert_eq!(
        prepublication_residuals.directory_sync(),
        DirectorySyncState::NotRequired
    );
    assert_residual_paths_exist(prepublication_residuals);
}

#[test]
fn rollback_reports_destination_only_residual() {
    let temporary = TempDirectory::new();
    let source = publication_source(&temporary);

    let destination_only_path = temporary.path("destination-only-backup");
    let mut destination_only = InjectedPublication::new();
    destination_only.fail_sync_calls.insert(1);
    destination_only.fail_remove_calls.insert(2);
    let destination_only_error =
        copy_recoverable_with(&source, &destination_only_path, &destination_only, || {})
            .unwrap_err();
    let destination_only_residuals = partial_residuals(&destination_only_error);
    assert_eq!(
        destination_only_residuals.paths(),
        &BTreeSet::from([PublicationResidual::new(
            PublicationResidualRole::Destination,
            destination_only_path.clone(),
        )])
    );
    assert_eq!(
        destination_only_residuals.directory_sync(),
        DirectorySyncState::Synced
    );
    assert_residual_paths_exist(destination_only_residuals);
}

#[test]
fn rollback_reports_temporary_only_residual() {
    let temporary = TempDirectory::new();
    let source = publication_source(&temporary);

    let temp_only_path = temporary.path("temp-only-backup");
    let mut temp_only = InjectedPublication::new();
    temp_only.fail_remove_calls.extend([1, 3]);
    let temp_only_error =
        copy_recoverable_with(&source, &temp_only_path, &temp_only, || {}).unwrap_err();
    let temp_only_residuals = partial_residuals(&temp_only_error);
    assert_eq!(
        residual_roles(temp_only_residuals),
        BTreeSet::from([PublicationResidualRole::Temporary])
    );
    assert_eq!(
        temp_only_residuals.directory_sync(),
        DirectorySyncState::Synced
    );
    assert_residual_paths_exist(temp_only_residuals);
}

#[test]
fn rollback_reports_both_residuals_and_safe_rendering() {
    let temporary = TempDirectory::new();
    let source = publication_source(&temporary);

    let both_path = temporary.path("both-backup");
    let mut both = InjectedPublication::new();
    both.fail_remove_calls.extend([1, 2, 3]);
    let both_error = copy_recoverable_with(&source, &both_path, &both, || {}).unwrap_err();
    let both_residuals = partial_residuals(&both_error);
    assert_eq!(
        residual_roles(both_residuals),
        BTreeSet::from([
            PublicationResidualRole::Temporary,
            PublicationResidualRole::Destination,
        ])
    );
    assert_eq!(both_residuals.directory_sync(), DirectorySyncState::Synced);
    assert_residual_paths_exist(both_residuals);

    let rendered = both_error.to_string();
    assert!(rendered.contains("temporary `"));
    assert!(rendered.contains("destination `"));
    assert!(rendered.contains("directory sync: synced"));
    assert!(!rendered.contains("complete"));
}

#[test]
fn sync_failure_without_residuals_reports_uncertainty() {
    let temporary = TempDirectory::new();
    let source = publication_source(&temporary);

    let uncertain_path = temporary.path("uncertain-sync-backup");
    let mut uncertain = InjectedPublication::new();
    uncertain.fail_sync_calls.extend([1, 2]);
    let uncertain_error =
        copy_recoverable_with(&source, &uncertain_path, &uncertain, || {}).unwrap_err();
    let uncertain_residuals = partial_residuals(&uncertain_error);
    assert!(uncertain_residuals.paths().is_empty());
    assert_eq!(
        uncertain_residuals.directory_sync(),
        DirectorySyncState::Uncertain
    );
}

fn publication_source(temporary: &TempDirectory) -> AbsolutePath {
    let source = temporary.path("source");
    SystemFileSystem.atomic_write(&source, b"complete").unwrap();
    source
}

fn partial_residuals(error: &RuntimeError) -> &PublicationResiduals {
    match error {
        RuntimeError::PartialPublication { residuals, .. } => residuals,
        error => panic!("expected partial publication error, got {error}"),
    }
}

fn residual_roles(residuals: &PublicationResiduals) -> BTreeSet<PublicationResidualRole> {
    residuals
        .paths()
        .iter()
        .map(PublicationResidual::role)
        .collect()
}

fn assert_residual_paths_exist(residuals: &PublicationResiduals) {
    for residual in residuals.paths() {
        let metadata = fs::symlink_metadata(residual.path().as_str()).unwrap();
        assert!(
            !metadata.file_type().is_symlink(),
            "owned residual must identify the generated inode without following a link"
        );
    }
}

#[test]
fn backup_source_swap_is_rejected_after_no_follow_open() {
    let temporary = TempDirectory::new();
    let filesystem = SystemFileSystem;
    let source = temporary.path("source");
    let opened = temporary.path("opened-source");
    let attacker = temporary.path("attacker");
    let destination = temporary.path("backup");
    filesystem.atomic_write(&source, b"original").unwrap();
    filesystem.atomic_write(&attacker, b"attacker").unwrap();

    let error = copy_recoverable_with(&source, &destination, &SystemPublication, || {
        fs::rename(source.as_str(), opened.as_str()).unwrap();
        std::os::unix::fs::symlink("attacker", source.as_str()).unwrap();
    })
    .unwrap_err();
    assert!(matches!(error, RuntimeError::FileIdentityChanged { .. }));
    assert_eq!(
        filesystem.inspect(&destination).unwrap().kind(),
        FileKind::Missing
    );
}
