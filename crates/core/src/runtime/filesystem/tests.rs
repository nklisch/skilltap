use std::{
    cell::Cell,
    sync::{Arc, atomic::AtomicBool},
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

use super::*;

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

struct TempDirectory(PathBuf);

impl TempDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "skilltap-filesystem-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn path(&self, child: &str) -> AbsolutePath {
        AbsolutePath::new(self.0.join(child).to_str().unwrap()).unwrap()
    }
}

impl Drop for TempDirectory {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn inspection_distinguishes_files_directories_and_live_or_dangling_links() {
    let temporary = TempDirectory::new();
    let filesystem = SystemFileSystem;
    let directory = temporary.path("directory");
    let file = temporary.path("file");
    let live_link = temporary.path("live-link");
    let dangling_link = temporary.path("dangling-link");
    filesystem.create_directory_all(&directory).unwrap();
    filesystem.atomic_write(&file, b"content").unwrap();
    filesystem
        .create_relative_symlink(&RelativeSymlinkTarget::new("file").unwrap(), &live_link)
        .unwrap();
    filesystem
        .create_relative_symlink(
            &RelativeSymlinkTarget::new("missing").unwrap(),
            &dangling_link,
        )
        .unwrap();

    assert_eq!(
        filesystem.inspect(&file).unwrap().kind(),
        FileKind::RegularFile
    );
    assert_eq!(
        filesystem.inspect(&directory).unwrap().kind(),
        FileKind::Directory
    );
    assert_eq!(
        filesystem
            .inspect(&temporary.path("missing"))
            .unwrap()
            .kind(),
        FileKind::Missing
    );
    let live = filesystem.inspect(&live_link).unwrap();
    assert_eq!(live.kind(), FileKind::Symlink);
    assert_eq!(live.link_target(), Some(Path::new("file")));
    assert_eq!(live.link_target_exists(), Some(true));
    assert_eq!(
        filesystem
            .inspect(&dangling_link)
            .unwrap()
            .link_target_exists(),
        Some(false)
    );
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
fn backup_failures_report_exact_residual_paths_and_independent_sync_state() {
    let temporary = TempDirectory::new();
    let filesystem = SystemFileSystem;
    let source = temporary.path("source");
    filesystem.atomic_write(&source, b"complete").unwrap();

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

    let rendered = both_error.to_string();
    assert!(rendered.contains("temporary `"));
    assert!(rendered.contains("destination `"));
    assert!(rendered.contains("directory sync: synced"));
    assert!(!rendered.contains("complete"));
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

#[test]
fn ownership_sensitive_operations_do_not_follow_symlinks() {
    let temporary = TempDirectory::new();
    let filesystem = SystemFileSystem;
    let target = temporary.path("user-content");
    let link = temporary.path("managed");
    filesystem.atomic_write(&target, b"preserve").unwrap();
    filesystem
        .create_relative_symlink(&RelativeSymlinkTarget::new("user-content").unwrap(), &link)
        .unwrap();

    assert!(matches!(
        filesystem.atomic_write(&link, b"replace"),
        Err(RuntimeError::UnsafeSymlink { .. })
    ));
    filesystem.remove(&link).unwrap();
    assert_eq!(filesystem.read(&target).unwrap(), b"preserve");
    assert_eq!(filesystem.inspect(&link).unwrap().kind(), FileKind::Missing);
}

#[test]
fn relative_targets_allow_parent_bridges_but_reject_absolute_or_unnormalized_paths() {
    assert_eq!(
        RelativeSymlinkTarget::new("../AGENTS.md")
            .unwrap()
            .as_path(),
        Path::new("../AGENTS.md")
    );
    assert_eq!(
        RelativeSymlinkTarget::new("../../shared/AGENTS.md")
            .unwrap()
            .as_path(),
        Path::new("../../shared/AGENTS.md")
    );
    for invalid in [
        "",
        "/tmp/AGENTS.md",
        "./AGENTS.md",
        "dir//AGENTS.md",
        "dir/../AGENTS.md",
        "../dir/../AGENTS.md",
        "..",
        "AGENTS.md\n",
    ] {
        assert!(RelativeSymlinkTarget::new(invalid).is_err(), "{invalid}");
    }
}

#[test]
fn configuration_lock_fails_fast_and_releases_explicitly_or_on_drop() {
    let temporary = TempDirectory::new();
    let path = temporary.path("skilltap.lock");
    let lock = SystemConfigurationLock;
    let first = lock.try_acquire(&path).unwrap();
    assert_eq!(first.path(), &path);
    assert!(matches!(
        lock.try_acquire(&path),
        Err(RuntimeError::LockContended { .. })
    ));
    first.release().unwrap();
    let second = lock.try_acquire(&path).unwrap();
    drop(second);
    lock.try_acquire(&path).unwrap().release().unwrap();
}

#[test]
fn lock_path_swaps_cannot_create_two_successful_guards() {
    let temporary = TempDirectory::new();
    let path = temporary.path("skilltap.lock");
    let displaced = temporary.path("displaced.lock");
    let lock = SystemConfigurationLock;

    let acquisition_error = try_acquire_with(&path, || {
        fs::rename(path.as_str(), displaced.as_str()).unwrap();
        fs::write(path.as_str(), b"replacement").unwrap();
    })
    .unwrap_err();
    assert!(matches!(
        acquisition_error,
        RuntimeError::LockIdentityChanged { .. }
    ));

    let first = lock.try_acquire(&path).unwrap();
    fs::remove_file(path.as_str()).unwrap();
    fs::write(path.as_str(), b"another inode").unwrap();
    assert!(matches!(
        lock.try_acquire(&path),
        Err(RuntimeError::LockContended { .. })
    ));
    drop(first);
    lock.try_acquire(&path).unwrap().release().unwrap();
}

#[test]
fn lock_open_refuses_symlink_paths_without_touching_the_target() {
    let temporary = TempDirectory::new();
    let target = temporary.path("user-file");
    let link = temporary.path("skilltap.lock");
    fs::write(target.as_str(), b"preserve").unwrap();
    std::os::unix::fs::symlink("user-file", link.as_str()).unwrap();

    assert!(SystemConfigurationLock.try_acquire(&link).is_err());
    assert_eq!(fs::read(target.as_str()).unwrap(), b"preserve");
}
