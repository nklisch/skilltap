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
