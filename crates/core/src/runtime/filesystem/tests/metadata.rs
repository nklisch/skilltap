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
fn descriptor_bound_read_handles_missing_regular_and_symlink_paths_without_following() {
    let temporary = TempDirectory::new();
    let filesystem = SystemFileSystem;
    let file = temporary.path("document");
    let link = temporary.path("document-link");
    filesystem.atomic_write(&file, b"complete").unwrap();
    filesystem
        .create_relative_symlink(&RelativeSymlinkTarget::new("document").unwrap(), &link)
        .unwrap();

    assert_eq!(
        filesystem
            .read_regular_no_follow(&temporary.path("missing"))
            .unwrap(),
        None
    );
    assert_eq!(
        filesystem.read_regular_no_follow(&file).unwrap(),
        Some(b"complete".to_vec())
    );
    assert_eq!(filesystem.read(&link).unwrap(), b"complete");
    assert!(matches!(
        filesystem.read_regular_no_follow(&link),
        Err(RuntimeError::UnsafeSymlink {
            action: FileSystemAction::Read,
            ..
        })
    ));
}

#[test]
fn descriptor_bound_read_rejects_path_swaps_after_open() {
    let temporary = TempDirectory::new();
    let filesystem = SystemFileSystem;
    let source = temporary.path("document");
    let displaced = temporary.path("opened-document");
    let attacker = temporary.path("attacker");
    filesystem.atomic_write(&source, b"trusted").unwrap();
    filesystem.atomic_write(&attacker, b"attacker").unwrap();

    let error = read_regular_no_follow_with(&source, || {
        fs::rename(source.as_str(), displaced.as_str()).unwrap();
        std::os::unix::fs::symlink("attacker", source.as_str()).unwrap();
    })
    .unwrap_err();

    assert!(matches!(
        error,
        RuntimeError::FileIdentityChanged {
            action: FileSystemAction::Read,
            ..
        }
    ));
    assert_eq!(filesystem.read(&attacker).unwrap(), b"attacker");
}

#[cfg(unix)]
#[test]
fn descriptor_bound_read_rejects_fifos_without_waiting_for_a_writer() {
    let temporary = TempDirectory::new();
    let fifo = temporary.path("document-fifo");
    let fifo_path = CString::new(fifo.as_str()).unwrap();
    // SAFETY: `fifo_path` is a live NUL-terminated path and the mode is valid.
    assert_eq!(unsafe { libc::mkfifo(fifo_path.as_ptr(), 0o600) }, 0);
    let (sender, receiver) = mpsc::channel();

    thread::spawn(move || {
        sender
            .send(SystemFileSystem.read_regular_no_follow(&fifo))
            .unwrap();
    });

    let result = receiver
        .recv_timeout(Duration::from_secs(2))
        .expect("FIFO read must fail without waiting for a writer");
    assert!(matches!(
        result,
        Err(RuntimeError::FileSystem {
            action: FileSystemAction::Read,
            ..
        })
    ));
}
