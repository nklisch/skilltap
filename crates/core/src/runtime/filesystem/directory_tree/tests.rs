#![cfg(unix)]

use std::{
    collections::BTreeMap,
    env,
    ffi::CString,
    fs::{self, File},
    io,
    os::unix::fs::{FileTypeExt, PermissionsExt},
    process::Command,
};

use skilltap_test_support::TempRoot;

use super::*;

#[test]
fn publication_normalizes_file_modes_and_loading_preserves_only_executable_intent() {
    let temporary = TempRoot::new("skilltap-directory-mode-test").unwrap();
    let managed = AbsolutePath::new(temporary.join("managed").to_str().unwrap()).unwrap();
    let destination = RelativeArtifactPath::new("artifact").unwrap();
    let files = BTreeMap::from([
        (
            RelativeArtifactPath::new("SKILL.md").unwrap(),
            ArtifactFile::new(b"#!/bin/sh\n".to_vec(), false),
        ),
        (
            RelativeArtifactPath::new("scripts/run").unwrap(),
            ArtifactFile::new(b"run".to_vec(), true),
        ),
    ]);

    assert!(matches!(
        publish_tree(&managed, &destination, &files).unwrap(),
        DirectoryPublishOutcome::Published(_)
    ));
    let root = temporary.join("managed/artifact");
    assert_eq!(
        fs::metadata(root.join("SKILL.md"))
            .unwrap()
            .permissions()
            .mode()
            & 0o7777,
        0o600
    );
    assert_eq!(
        fs::metadata(root.join("scripts/run"))
            .unwrap()
            .permissions()
            .mode()
            & 0o7777,
        0o700
    );
    let (_, loaded) = load_tree(&managed, &destination).unwrap();
    assert_eq!(loaded, files);

    assert_eq!(
        publish_tree(&managed, &destination, &files).unwrap(),
        DirectoryPublishOutcome::AlreadyExists
    );
    assert_eq!(
        fs::metadata(root.join("SKILL.md"))
            .unwrap()
            .permissions()
            .mode()
            & 0o7777,
        0o600
    );
    assert_eq!(
        fs::metadata(root.join("scripts/run"))
            .unwrap()
            .permissions()
            .mode()
            & 0o7777,
        0o700
    );
}

#[test]
fn descriptor_publication_normalizes_modes_under_a_restrictive_umask() {
    const CHILD_MARKER: &str = "SKILLTAP_RESTRICTIVE_UMASK_TEST_CHILD";

    if env::var_os(CHILD_MARKER).is_none() {
        let status = Command::new(env::current_exe().unwrap())
            .args([
                "descriptor_publication_normalizes_modes_under_a_restrictive_umask",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(CHILD_MARKER, "1")
            .status()
            .unwrap();
        assert!(status.success(), "restrictive-umask child test failed");
        return;
    }

    struct UmaskGuard(libc::mode_t);

    impl Drop for UmaskGuard {
        fn drop(&mut self) {
            unsafe {
                libc::umask(self.0);
            }
        }
    }

    let temporary = TempRoot::new("skilltap-restrictive-umask-test").unwrap();
    let root = temporary.join("artifact");
    fs::create_dir(&root).unwrap();
    let directory = File::open(&root).unwrap();
    let files = BTreeMap::from([
        (
            RelativeArtifactPath::new("SKILL.md").unwrap(),
            ArtifactFile::new(b"guidance".to_vec(), false),
        ),
        (
            RelativeArtifactPath::new("run").unwrap(),
            ArtifactFile::new(b"#!/bin/sh\n".to_vec(), true),
        ),
    ]);

    {
        let _umask = UmaskGuard(unsafe { libc::umask(0o777) });
        write_tree(&directory, &files).unwrap();
    }

    assert_eq!(
        fs::metadata(root.join("SKILL.md"))
            .unwrap()
            .permissions()
            .mode()
            & 0o7777,
        0o600
    );
    assert_eq!(
        fs::metadata(root.join("run")).unwrap().permissions().mode() & 0o7777,
        0o700
    );
    let mut loaded = BTreeMap::new();
    read_tree(&directory, None, &mut loaded).unwrap();
    assert_eq!(loaded, files);

    assert_eq!(
        write_tree(&directory, &files).unwrap_err().kind(),
        io::ErrorKind::AlreadyExists
    );
    assert_eq!(
        fs::metadata(root.join("SKILL.md"))
            .unwrap()
            .permissions()
            .mode()
            & 0o7777,
        0o600
    );
    assert_eq!(
        fs::metadata(root.join("run")).unwrap().permissions().mode() & 0o7777,
        0o700
    );
}

#[test]
fn cooperating_writer_lock_serializes_managed_namespace_publication() {
    let temporary = TempRoot::new("skilltap-directory-lock-test").unwrap();
    let managed = temporary.join("managed");
    fs::create_dir(&managed).unwrap();
    let held = File::open(&managed).unwrap();
    held.try_lock().unwrap();
    let managed = AbsolutePath::new(managed.to_str().unwrap()).unwrap();
    let destination = RelativeArtifactPath::new("artifact").unwrap();
    let files = BTreeMap::from([(
        RelativeArtifactPath::new("file").unwrap(),
        ArtifactFile::new(vec![1], false),
    )]);

    let error = publish_tree(&managed, &destination, &files).unwrap_err();
    assert!(matches!(
        error,
        RuntimeError::FileSystem { source, .. } if source.kind() == io::ErrorKind::WouldBlock
    ));
}

#[test]
fn successful_publication_explicitly_releases_writer_lock_before_return() {
    let temporary = TempRoot::new("skilltap-directory-lock-release-test").unwrap();
    let managed = temporary.join("managed");
    let managed_path = AbsolutePath::new(managed.to_str().unwrap()).unwrap();
    let files = BTreeMap::from([(
        RelativeArtifactPath::new("file").unwrap(),
        ArtifactFile::new(vec![1], false),
    )]);

    for sequence in 0..128 {
        let destination = RelativeArtifactPath::new(format!("artifact-{sequence}")).unwrap();
        assert!(matches!(
            publish_tree(&managed_path, &destination, &files).unwrap(),
            DirectoryPublishOutcome::Published(_)
        ));
        let probe = File::open(&managed).unwrap();
        probe.try_lock().unwrap();
        probe.unlock().unwrap();
    }
}

#[test]
fn post_stat_fifo_swap_is_rejected_before_reading() {
    let temporary = TempRoot::new("skilltap-directory-read-race-test").unwrap();
    let path = temporary.join("file");
    fs::write(&path, b"original").unwrap();
    let directory = File::open(temporary.path()).unwrap();
    let mut files = BTreeMap::new();
    let mut swapped = false;
    let error = read_tree_with(&directory, None, &mut files, &mut |_| {
        if !swapped {
            swapped = true;
            fs::remove_file(&path).unwrap();
            let path = CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
            assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
        }
    })
    .unwrap_err();
    assert!(error.to_string().contains("regular file"));
    assert!(fs::symlink_metadata(path).unwrap().file_type().is_fifo());
}

#[test]
fn post_stat_fifo_swap_is_rejected_before_removal() {
    let temporary = TempRoot::new("skilltap-directory-remove-race-test").unwrap();
    let path = temporary.join("file");
    fs::write(&path, b"original").unwrap();
    let directory = File::open(temporary.path()).unwrap();
    let mut swapped = false;
    let error = remove_open_tree_with(&directory, &mut |_| {
        if !swapped {
            swapped = true;
            fs::remove_file(&path).unwrap();
            let path = CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
            assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
        }
    })
    .unwrap_err();
    assert!(error.to_string().contains("regular file"));
    assert!(fs::symlink_metadata(path).unwrap().file_type().is_fifo());
}

#[test]
fn cleanup_reports_removed_path_when_parent_sync_fails_after_unlink() {
    let temporary = TempRoot::new("skilltap-directory-cleanup-sync-test").unwrap();
    let parent = File::open(temporary.path()).unwrap();
    let name = CString::new("artifact").unwrap();
    let directory = create_dir_at_verified(&parent, &name).unwrap();
    let identity = require_directory(&directory).unwrap();
    let root = AbsolutePath::new(temporary.path().to_str().unwrap()).unwrap();
    let destination = RelativeArtifactPath::new("artifact").unwrap();

    let error = clean_publication_failure_with_parent_sync(
        &root,
        &destination,
        &parent,
        &name,
        &directory,
        identity,
        DirectorySyncState::Synced,
        io::Error::other("write failed"),
        || Err(io::Error::other("parent sync failed")),
    );
    assert!(matches!(
        error,
        RuntimeError::PartialDirectoryPublication {
            identity: None,
            presence: DirectoryPathState::Removed,
            parent_sync: DirectorySyncState::Uncertain,
            ..
        }
    ));
    assert!(!temporary.join("artifact").exists());
}

#[test]
fn cleanup_reports_present_identity_when_owned_tree_cannot_be_removed() {
    let temporary = TempRoot::new("skilltap-directory-cleanup-present-test").unwrap();
    let parent = File::open(temporary.path()).unwrap();
    let name = CString::new("artifact").unwrap();
    let directory = create_dir_at_verified(&parent, &name).unwrap();
    let identity = require_directory(&directory).unwrap();
    let fifo = temporary.join("artifact/blocked");
    let fifo = CString::new(fifo.as_os_str().as_encoded_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
    let root = AbsolutePath::new(temporary.path().to_str().unwrap()).unwrap();
    let destination = RelativeArtifactPath::new("artifact").unwrap();

    let error = clean_publication_failure_with_parent_sync(
        &root,
        &destination,
        &parent,
        &name,
        &directory,
        identity,
        DirectorySyncState::Synced,
        io::Error::other("write failed"),
        || Ok(()),
    );
    assert!(matches!(
        error,
        RuntimeError::PartialDirectoryPublication {
            identity: Some(observed),
            presence: DirectoryPathState::Present,
            parent_sync: DirectorySyncState::Synced,
            ..
        } if observed == identity
    ));
}

fn removal_fixture(
    name: &str,
) -> (
    TempRoot,
    AbsolutePath,
    RelativeArtifactPath,
    DirectoryIdentity,
) {
    let temporary = TempRoot::new(name).unwrap();
    let managed = temporary.join("managed");
    let artifact = managed.join("artifact");
    fs::create_dir(&managed).unwrap();
    fs::create_dir(&artifact).unwrap();
    let identity = require_directory(&File::open(&artifact).unwrap()).unwrap();
    (
        temporary,
        AbsolutePath::new(managed.to_str().unwrap()).unwrap(),
        RelativeArtifactPath::new("artifact").unwrap(),
        identity,
    )
}

fn make_fifo(path: &std::path::Path) {
    let path = CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
}

#[test]
fn removal_failure_before_changes_reports_intact_present_identity() {
    let (temporary, managed, destination, expected) =
        removal_fixture("skilltap-removal-intact-test");
    make_fifo(&temporary.join("managed/artifact/blocked"));

    let error = remove_tree(&managed, &destination, expected).unwrap_err();
    assert!(matches!(
        error,
        RuntimeError::PartialDirectoryRemoval {
            expected: actual_expected,
            observed: Some(observed),
            presence: DirectoryPathState::Present,
            content: DirectoryContentState::Intact,
            parent_sync: DirectorySyncState::NotRequired,
            ..
        } if actual_expected == expected && observed == expected
    ));
}

#[test]
fn removal_failure_after_an_entry_reports_partial_content() {
    let (temporary, managed, destination, expected) =
        removal_fixture("skilltap-removal-partial-test");
    fs::write(temporary.join("managed/artifact/a-file"), b"removed first").unwrap();
    make_fifo(&temporary.join("managed/artifact/z-blocked"));

    let error = remove_tree(&managed, &destination, expected).unwrap_err();
    assert!(matches!(
        error,
        RuntimeError::PartialDirectoryRemoval {
            expected: actual_expected,
            observed: Some(observed),
            presence: DirectoryPathState::Present,
            content: DirectoryContentState::Partial,
            parent_sync: DirectorySyncState::NotRequired,
            ..
        } if actual_expected == expected && observed == expected
    ));
    assert!(!temporary.join("managed/artifact/a-file").exists());
    assert!(temporary.join("managed/artifact/z-blocked").exists());
}

#[test]
fn replacement_before_removal_reports_both_identities_without_guessing_contents() {
    let (temporary, managed, destination, expected) =
        removal_fixture("skilltap-removal-replacement-test");
    fs::rename(
        temporary.join("managed/artifact"),
        temporary.join("managed/original"),
    )
    .unwrap();
    fs::create_dir(temporary.join("managed/artifact")).unwrap();
    let replacement =
        require_directory(&File::open(temporary.join("managed/artifact")).unwrap()).unwrap();

    let error = remove_tree(&managed, &destination, expected).unwrap_err();
    assert!(matches!(
        error,
        RuntimeError::PartialDirectoryRemoval {
            expected: actual_expected,
            observed: Some(observed),
            presence: DirectoryPathState::Present,
            content: DirectoryContentState::Intact,
            parent_sync: DirectorySyncState::NotRequired,
            ..
        } if actual_expected == expected && observed == replacement && observed != expected
    ));
}

#[test]
fn top_unlink_failure_reports_empty_present_directory() {
    let (_temporary, managed, destination, expected) =
        removal_fixture("skilltap-removal-empty-present-test");

    let error = remove_tree_with(
        &managed,
        &destination,
        expected,
        |_parent, _name| Err(io::Error::other("injected top unlink failure")),
        |_parent| panic!("parent sync must not run when unlink fails"),
    )
    .unwrap_err();
    assert!(matches!(
        error,
        RuntimeError::PartialDirectoryRemoval {
            expected: actual_expected,
            observed: Some(observed),
            presence: DirectoryPathState::Present,
            content: DirectoryContentState::Empty,
            parent_sync: DirectorySyncState::NotRequired,
            ..
        } if actual_expected == expected && observed == expected
    ));
}

#[test]
fn parent_sync_failure_reports_removed_path_and_uncertain_durability() {
    let (temporary, managed, destination, expected) =
        removal_fixture("skilltap-removal-sync-uncertain-test");

    let error = remove_tree_with(
        &managed,
        &destination,
        expected,
        |parent, name| unlink_at(parent.as_raw_fd(), name, true),
        |_parent| Err(io::Error::other("injected parent sync failure")),
    )
    .unwrap_err();
    assert!(matches!(
        error,
        RuntimeError::PartialDirectoryRemoval {
            expected: actual_expected,
            observed: None,
            presence: DirectoryPathState::Removed,
            content: DirectoryContentState::Empty,
            parent_sync: DirectorySyncState::Uncertain,
            ..
        } if actual_expected == expected
    ));
    assert!(!temporary.join("managed/artifact").exists());
}

#[cfg(unix)]
#[test]
fn confined_file_operations_reject_symlink_ancestors_without_touching_the_target() {
    use std::os::unix::fs::symlink;

    let temporary = TempRoot::new("skilltap-confined-file-symlink-ancestor").unwrap();
    let project = temporary.join("project");
    let outside = temporary.join("outside");
    fs::create_dir(&project).unwrap();
    fs::create_dir(&outside).unwrap();
    fs::create_dir(outside.join("plugins")).unwrap();
    symlink(&outside, project.join(".agents")).unwrap();

    let project = AbsolutePath::new(project.to_str().unwrap()).unwrap();
    let destination = RelativeArtifactPath::new(".agents/plugins/marketplace.json").unwrap();
    let filesystem = SystemFileSystem;

    assert!(
        filesystem
            .read_regular_bounded_no_follow(&project, &destination, 4096)
            .is_err()
    );
    assert!(
        filesystem
            .atomic_write_beneath_no_follow(&project, &destination, b"catalog")
            .is_err()
    );
    assert!(
        filesystem
            .remove_file_beneath_no_follow(&project, &destination)
            .is_err()
    );
    assert!(!outside.join("plugins/marketplace.json").exists());
}
