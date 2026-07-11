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
    let backup_owner = owner("plugin:tools");
    let tree = skill_tree();
    let first = repository.backup(&backup_owner, &tree).unwrap();
    let second = repository.backup(&backup_owner, &tree).unwrap();
    assert_ne!(first.record().path(), second.record().path());
    assert_eq!(first.record().role(), ArtifactRole::Backup);
    assert_eq!(first.record().fingerprint(), None);
    assert_eq!(
        repository
            .load(&backup_owner, first.record())
            .unwrap()
            .tree(),
        &tree
    );
    assert_eq!(
        repository
            .load(&backup_owner, second.record())
            .unwrap()
            .tree(),
        &tree
    );
    repository.remove(&backup_owner, &first).unwrap();
    assert!(repository.load(&backup_owner, first.record()).is_err());
    assert_eq!(
        repository
            .load(&backup_owner, second.record())
            .unwrap()
            .tree(),
        &tree
    );

    let maximum_owner = owner(&"a".repeat(256));
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
    let distinct_owner = owner(&"b".repeat(256));
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
    let managed_owner = owner("skill:owned");
    let other = owner("skill:other");
    let fingerprint = fingerprint('b');
    let handle = match repository
        .publish(
            &managed_owner,
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
        managed_owner.clone(),
        ArtifactRole::DirectSkill,
        RelativeArtifactPath::new("unowned-path").unwrap(),
        Some(fingerprint.clone()),
    );
    assert!(matches!(
        wrong_path,
        Err(SchemaError::InvalidManagedArtifactRecord { .. })
    ));

    let loaded = repository.load(&managed_owner, handle.record()).unwrap();
    let path = absolute(&repository, handle.record().path());
    let displaced = path.with_extension("displaced");
    fs::rename(&path, &displaced).unwrap();
    fs::create_dir(&path).unwrap();
    fs::write(path.join("victim"), b"preserve").unwrap();
    let error = repository
        .remove(&managed_owner, loaded.handle())
        .unwrap_err();
    assert_eq!(error.failure(), ManagedArtifactFailure::PartialRemoval);
    let residual = error.removal_residual().unwrap();
    assert_eq!(residual.expected_identity(), loaded.handle().identity());
    assert_ne!(
        residual.observed_identity(),
        Some(loaded.handle().identity())
    );
    assert_eq!(residual.presence(), DirectoryPathState::Present);
    assert_eq!(residual.content(), DirectoryContentState::Intact);
    assert_eq!(residual.parent_sync(), DirectorySyncState::NotRequired);
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
    let destination = ManagedArtifactRecord::for_artifact(
        owner.clone(),
        ArtifactRole::DirectSkill,
        fingerprint.clone(),
    )
    .unwrap()
    .path()
    .clone();
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
