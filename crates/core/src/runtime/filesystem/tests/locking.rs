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

    for _ in 0..128 {
        let acquisition_error = try_acquire_with(&path, || {
            fs::rename(path.as_str(), displaced.as_str()).unwrap();
            fs::write(path.as_str(), b"replacement").unwrap();
        })
        .unwrap_err();
        assert!(matches!(
            acquisition_error,
            RuntimeError::LockIdentityChanged { .. }
        ));
        lock.try_acquire(&path).unwrap().release().unwrap();
        fs::remove_file(path.as_str()).unwrap();
        fs::remove_file(displaced.as_str()).unwrap();
    }

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
