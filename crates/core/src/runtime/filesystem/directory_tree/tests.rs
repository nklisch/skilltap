#![cfg(unix)]

use std::{
    collections::BTreeMap,
    ffi::CString,
    fs::{self, File},
    io,
    os::unix::fs::FileTypeExt,
};

use skilltap_test_support::TempRoot;

use super::*;

#[test]
fn cooperating_writer_lock_serializes_managed_namespace_publication() {
    let temporary = TempRoot::new("skilltap-directory-lock-test").unwrap();
    let managed = temporary.join("managed");
    fs::create_dir(&managed).unwrap();
    let held = File::open(&managed).unwrap();
    held.try_lock().unwrap();
    let managed = AbsolutePath::new(managed.to_str().unwrap()).unwrap();
    let destination = RelativeArtifactPath::new("artifact").unwrap();
    let files = BTreeMap::from([(RelativeArtifactPath::new("file").unwrap(), vec![1])]);

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
    let files = BTreeMap::from([(RelativeArtifactPath::new("file").unwrap(), vec![1])]);

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
