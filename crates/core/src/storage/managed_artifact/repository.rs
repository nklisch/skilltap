use std::sync::atomic::{AtomicU64, Ordering};

use crate::{
    domain::{Fingerprint, ResourceKey},
    runtime::DirectoryPublishOutcome,
};

use super::{
    ArtifactPublication, ArtifactRole, ArtifactTree, FileManagedArtifactRepository, LoadedArtifact,
    ManagedArtifactAction, ManagedArtifactError, ManagedArtifactFailure, ManagedArtifactHandle,
    ManagedArtifactRecord, ManagedArtifactRepository,
};

static BACKUP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

impl FileManagedArtifactRepository<'_> {
    fn publish_at(
        &self,
        record: ManagedArtifactRecord,
        tree: &ArtifactTree,
        action: ManagedArtifactAction,
    ) -> Result<ArtifactPublication, ManagedArtifactError> {
        let owner = record.owner().clone();
        let path = record.path().clone();
        match self
            .filesystem
            .publish_tree_no_follow(&self.managed_root, &path, tree.files())
            .map_err(|error| ManagedArtifactError::runtime(action, &owner, &path, error))?
        {
            DirectoryPublishOutcome::Published(identity) => {
                Ok(ArtifactPublication::Published(ManagedArtifactHandle {
                    record,
                    identity,
                }))
            }
            DirectoryPublishOutcome::AlreadyExists => {
                let loaded = self.load_with_action(&owner, &record, action)?;
                if loaded.tree() == tree {
                    Ok(ArtifactPublication::Existing(loaded.handle))
                } else {
                    Err(ManagedArtifactError::new(
                        action,
                        &owner,
                        Some(&path),
                        ManagedArtifactFailure::Conflict,
                    ))
                }
            }
        }
    }

    fn load_with_action(
        &self,
        owner: &ResourceKey,
        record: &ManagedArtifactRecord,
        action: ManagedArtifactAction,
    ) -> Result<LoadedArtifact, ManagedArtifactError> {
        record.validate_for_owner(owner).map_err(|_| {
            ManagedArtifactError::new(
                action,
                owner,
                Some(record.path()),
                ManagedArtifactFailure::InvalidRecord,
            )
        })?;
        let (identity, files) = self
            .filesystem
            .load_tree_no_follow(&self.managed_root, record.path())
            .map_err(|error| ManagedArtifactError::runtime(action, owner, record.path(), error))?;
        let tree = ArtifactTree::from_validated(files).map_err(|_| {
            ManagedArtifactError::new(
                action,
                owner,
                Some(record.path()),
                ManagedArtifactFailure::Conflict,
            )
        })?;
        Ok(LoadedArtifact {
            handle: ManagedArtifactHandle {
                record: record.clone(),
                identity,
            },
            tree,
        })
    }
}

impl ManagedArtifactRepository for FileManagedArtifactRepository<'_> {
    fn publish(
        &self,
        owner: &ResourceKey,
        role: ArtifactRole,
        fingerprint: &Fingerprint,
        tree: &ArtifactTree,
    ) -> Result<ArtifactPublication, ManagedArtifactError> {
        if role == ArtifactRole::Backup {
            return Err(ManagedArtifactError::new(
                ManagedArtifactAction::Publish,
                owner,
                None,
                ManagedArtifactFailure::InvalidRecord,
            ));
        }
        let record = ManagedArtifactRecord::for_artifact(owner.clone(), role, fingerprint.clone())
            .map_err(|_| {
                ManagedArtifactError::new(
                    ManagedArtifactAction::Publish,
                    owner,
                    None,
                    ManagedArtifactFailure::InvalidRecord,
                )
            })?;
        self.publish_at(record, tree, ManagedArtifactAction::Publish)
    }

    fn backup(
        &self,
        owner: &ResourceKey,
        tree: &ArtifactTree,
    ) -> Result<ManagedArtifactHandle, ManagedArtifactError> {
        for _ in 0..32 {
            let sequence = BACKUP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let record =
                ManagedArtifactRecord::for_backup(owner.clone(), std::process::id(), sequence)
                    .map_err(|_| {
                        ManagedArtifactError::new(
                            ManagedArtifactAction::Backup,
                            owner,
                            None,
                            ManagedArtifactFailure::InvalidRecord,
                        )
                    })?;
            let path = record.path().clone();
            match self
                .filesystem
                .publish_tree_no_follow(&self.managed_root, &path, tree.files())
                .map_err(|error| {
                    ManagedArtifactError::runtime(
                        ManagedArtifactAction::Backup,
                        owner,
                        &path,
                        error,
                    )
                })? {
                DirectoryPublishOutcome::Published(identity) => {
                    return Ok(ManagedArtifactHandle { record, identity });
                }
                DirectoryPublishOutcome::AlreadyExists => {
                    let _ = self.load_with_action(owner, &record, ManagedArtifactAction::Backup);
                }
            }
        }
        Err(ManagedArtifactError::new(
            ManagedArtifactAction::Backup,
            owner,
            None,
            ManagedArtifactFailure::Conflict,
        ))
    }

    fn load(
        &self,
        owner: &ResourceKey,
        record: &ManagedArtifactRecord,
    ) -> Result<LoadedArtifact, ManagedArtifactError> {
        self.load_with_action(owner, record, ManagedArtifactAction::Load)
    }

    fn remove(
        &self,
        owner: &ResourceKey,
        handle: &ManagedArtifactHandle,
    ) -> Result<(), ManagedArtifactError> {
        handle.record().validate_for_owner(owner).map_err(|_| {
            ManagedArtifactError::new(
                ManagedArtifactAction::Remove,
                owner,
                Some(handle.record().path()),
                ManagedArtifactFailure::InvalidRecord,
            )
        })?;
        self.filesystem
            .remove_tree_no_follow(
                &self.managed_root,
                handle.record().path(),
                handle.identity(),
            )
            .map(|_| ())
            .map_err(|error| {
                ManagedArtifactError::runtime(
                    ManagedArtifactAction::Remove,
                    owner,
                    handle.record().path(),
                    error,
                )
            })
    }
}
