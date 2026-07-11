use std::{
    collections::BTreeMap,
    fmt,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use sha2::{Digest, Sha256};

use crate::{
    domain::{AbsolutePath, Fingerprint, RelativeArtifactPath, ResourceId},
    runtime::{DirectoryIdentity, DirectoryPublishOutcome, DirectoryTreeFileSystem, RuntimeError},
};

use super::{ArtifactRole, ManagedArtifactRecord};

static BACKUP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArtifactTreeError {
    Empty,
    InvalidPath,
    DuplicatePath { path: String },
    FileIsAncestor { path: RelativeArtifactPath },
}

impl fmt::Display for ArtifactTreeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("artifact tree must contain at least one file"),
            Self::InvalidPath => formatter.write_str("artifact tree contains an invalid file path"),
            Self::DuplicatePath { path } => write!(formatter, "duplicate artifact file `{path}`"),
            Self::FileIsAncestor { path } => {
                write!(
                    formatter,
                    "artifact file `{path}` is also a directory ancestor"
                )
            }
        }
    }
}

impl std::error::Error for ArtifactTreeError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactTree {
    files: BTreeMap<RelativeArtifactPath, Vec<u8>>,
}

impl ArtifactTree {
    pub fn new<P, B>(files: impl IntoIterator<Item = (P, B)>) -> Result<Self, ArtifactTreeError>
    where
        P: Into<String>,
        B: Into<Vec<u8>>,
    {
        let mut collected = BTreeMap::new();
        for (path, contents) in files {
            let raw = path.into();
            let path = RelativeArtifactPath::new(raw.clone())
                .map_err(|_| ArtifactTreeError::InvalidPath)?;
            if collected.insert(path, contents.into()).is_some() {
                return Err(ArtifactTreeError::DuplicatePath { path: raw });
            }
        }
        if collected.is_empty() {
            return Err(ArtifactTreeError::Empty);
        }
        for path in collected.keys() {
            let mut ancestor = Path::new(path.as_str()).parent().map(PathBuf::from);
            while let Some(candidate) = ancestor {
                if candidate.as_os_str().is_empty() {
                    break;
                }
                let candidate = RelativeArtifactPath::new(candidate.to_string_lossy().into_owned())
                    .map_err(|_| ArtifactTreeError::InvalidPath)?;
                if collected.contains_key(&candidate) {
                    return Err(ArtifactTreeError::FileIsAncestor { path: candidate });
                }
                ancestor = Path::new(candidate.as_str()).parent().map(PathBuf::from);
            }
        }
        Ok(Self { files: collected })
    }

    fn from_validated(
        files: BTreeMap<RelativeArtifactPath, Vec<u8>>,
    ) -> Result<Self, ArtifactTreeError> {
        Self::new(
            files
                .into_iter()
                .map(|(path, contents)| (path.as_str().to_owned(), contents)),
        )
    }

    pub const fn files(&self) -> &BTreeMap<RelativeArtifactPath, Vec<u8>> {
        &self.files
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ManagedArtifactAction {
    Publish,
    Backup,
    Load,
    Remove,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ManagedArtifactFailure {
    InvalidRecord,
    Conflict,
    Runtime,
    PartialPublication,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedArtifactResidual {
    owner: ResourceId,
    path: RelativeArtifactPath,
    identity: Option<DirectoryIdentity>,
    presence: crate::runtime::DirectoryPathState,
    parent_sync: crate::runtime::DirectorySyncState,
}

impl ManagedArtifactResidual {
    pub fn owner(&self) -> &ResourceId {
        &self.owner
    }

    pub fn path(&self) -> &RelativeArtifactPath {
        &self.path
    }

    pub const fn identity(&self) -> Option<DirectoryIdentity> {
        self.identity
    }

    pub const fn presence(&self) -> crate::runtime::DirectoryPathState {
        self.presence
    }

    pub const fn parent_sync(&self) -> crate::runtime::DirectorySyncState {
        self.parent_sync
    }
}

pub struct ManagedArtifactError {
    action: ManagedArtifactAction,
    owner: ResourceId,
    path: Option<RelativeArtifactPath>,
    failure: ManagedArtifactFailure,
    residual: Option<Box<ManagedArtifactResidual>>,
}

impl ManagedArtifactError {
    pub const fn action(&self) -> ManagedArtifactAction {
        self.action
    }

    pub fn owner(&self) -> &ResourceId {
        &self.owner
    }

    pub const fn path(&self) -> Option<&RelativeArtifactPath> {
        self.path.as_ref()
    }

    pub const fn failure(&self) -> ManagedArtifactFailure {
        self.failure
    }

    pub fn residual(&self) -> Option<&ManagedArtifactResidual> {
        self.residual.as_deref()
    }

    fn new(
        action: ManagedArtifactAction,
        owner: &ResourceId,
        path: Option<&RelativeArtifactPath>,
        failure: ManagedArtifactFailure,
    ) -> Self {
        Self {
            action,
            owner: owner.clone(),
            path: path.cloned(),
            failure,
            residual: None,
        }
    }

    fn runtime(
        action: ManagedArtifactAction,
        owner: &ResourceId,
        path: &RelativeArtifactPath,
        error: RuntimeError,
    ) -> Self {
        match error {
            RuntimeError::PartialDirectoryPublication {
                identity,
                presence,
                parent_sync,
                ..
            } => Self {
                action,
                owner: owner.clone(),
                path: Some(path.clone()),
                failure: ManagedArtifactFailure::PartialPublication,
                residual: Some(Box::new(ManagedArtifactResidual {
                    owner: owner.clone(),
                    path: path.clone(),
                    identity,
                    presence,
                    parent_sync,
                })),
            },
            _ => Self::new(action, owner, Some(path), ManagedArtifactFailure::Runtime),
        }
    }
}

impl fmt::Debug for ManagedArtifactError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedArtifactError")
            .field("action", &self.action)
            .field("owner", &self.owner)
            .field("path", &self.path)
            .field("failure", &self.failure)
            .field("residual", &self.residual)
            .finish()
    }
}

impl fmt::Display for ManagedArtifactError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "managed artifact {:?} for `{}` failed",
            self.action, self.owner
        )?;
        if let Some(path) = &self.path {
            write!(formatter, " at `{path}`")?;
        }
        write!(formatter, ": {:?}", self.failure)
    }
}

impl std::error::Error for ManagedArtifactError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedArtifactHandle {
    record: ManagedArtifactRecord,
    identity: DirectoryIdentity,
}

impl ManagedArtifactHandle {
    pub const fn record(&self) -> &ManagedArtifactRecord {
        &self.record
    }

    pub const fn identity(&self) -> DirectoryIdentity {
        self.identity
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoadedArtifact {
    handle: ManagedArtifactHandle,
    tree: ArtifactTree,
}

impl LoadedArtifact {
    pub const fn handle(&self) -> &ManagedArtifactHandle {
        &self.handle
    }

    pub const fn tree(&self) -> &ArtifactTree {
        &self.tree
    }

    pub fn into_parts(self) -> (ManagedArtifactHandle, ArtifactTree) {
        (self.handle, self.tree)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArtifactPublication {
    Published(ManagedArtifactHandle),
    Existing(ManagedArtifactHandle),
}

pub trait ManagedArtifactRepository {
    fn publish(
        &self,
        owner: &ResourceId,
        role: ArtifactRole,
        fingerprint: &Fingerprint,
        tree: &ArtifactTree,
    ) -> Result<ArtifactPublication, ManagedArtifactError>;

    fn backup(
        &self,
        owner: &ResourceId,
        tree: &ArtifactTree,
    ) -> Result<ManagedArtifactHandle, ManagedArtifactError>;

    fn load(
        &self,
        owner: &ResourceId,
        record: &ManagedArtifactRecord,
    ) -> Result<LoadedArtifact, ManagedArtifactError>;

    fn remove(
        &self,
        owner: &ResourceId,
        handle: &ManagedArtifactHandle,
    ) -> Result<(), ManagedArtifactError>;
}

pub struct FileManagedArtifactRepository<'a> {
    filesystem: &'a dyn DirectoryTreeFileSystem,
    managed_root: AbsolutePath,
}

impl<'a> FileManagedArtifactRepository<'a> {
    pub fn new(
        filesystem: &'a dyn DirectoryTreeFileSystem,
        config_root: AbsolutePath,
    ) -> Result<Self, ManagedArtifactError> {
        let owner =
            ResourceId::new("skilltap:managed").expect("static managed-root owner is valid");
        let managed_root =
            AbsolutePath::new(format!("{}/managed", config_root.as_str())).map_err(|_| {
                ManagedArtifactError::new(
                    ManagedArtifactAction::Load,
                    &owner,
                    None,
                    ManagedArtifactFailure::InvalidRecord,
                )
            })?;
        Ok(Self {
            filesystem,
            managed_root,
        })
    }

    pub const fn managed_root(&self) -> &AbsolutePath {
        &self.managed_root
    }

    fn publish_at(
        &self,
        owner: &ResourceId,
        role: ArtifactRole,
        fingerprint: Option<&Fingerprint>,
        path: RelativeArtifactPath,
        tree: &ArtifactTree,
        action: ManagedArtifactAction,
    ) -> Result<ArtifactPublication, ManagedArtifactError> {
        let record =
            ManagedArtifactRecord::new(owner.clone(), role, path.clone(), fingerprint.cloned());
        match self
            .filesystem
            .publish_tree_no_follow(&self.managed_root, &path, tree.files())
            .map_err(|error| ManagedArtifactError::runtime(action, owner, &path, error))?
        {
            DirectoryPublishOutcome::Published(identity) => {
                Ok(ArtifactPublication::Published(ManagedArtifactHandle {
                    record,
                    identity,
                }))
            }
            DirectoryPublishOutcome::AlreadyExists => {
                let loaded = self.load_with_action(owner, &record, action)?;
                if loaded.tree() == tree {
                    Ok(ArtifactPublication::Existing(loaded.handle))
                } else {
                    Err(ManagedArtifactError::new(
                        action,
                        owner,
                        Some(&path),
                        ManagedArtifactFailure::Conflict,
                    ))
                }
            }
        }
    }

    fn load_with_action(
        &self,
        owner: &ResourceId,
        record: &ManagedArtifactRecord,
        action: ManagedArtifactAction,
    ) -> Result<LoadedArtifact, ManagedArtifactError> {
        validate_record(owner, record).map_err(|()| {
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
        owner: &ResourceId,
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
        let path = artifact_path(owner, role, fingerprint).map_err(|_| {
            ManagedArtifactError::new(
                ManagedArtifactAction::Publish,
                owner,
                None,
                ManagedArtifactFailure::InvalidRecord,
            )
        })?;
        self.publish_at(
            owner,
            role,
            Some(fingerprint),
            path,
            tree,
            ManagedArtifactAction::Publish,
        )
    }

    fn backup(
        &self,
        owner: &ResourceId,
        tree: &ArtifactTree,
    ) -> Result<ManagedArtifactHandle, ManagedArtifactError> {
        for _ in 0..32 {
            let sequence = BACKUP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = RelativeArtifactPath::new(format!(
                "backup-{}-{}-{sequence}",
                owner_key(owner),
                std::process::id()
            ))
            .map_err(|_| {
                ManagedArtifactError::new(
                    ManagedArtifactAction::Backup,
                    owner,
                    None,
                    ManagedArtifactFailure::InvalidRecord,
                )
            })?;
            let record =
                ManagedArtifactRecord::new(owner.clone(), ArtifactRole::Backup, path.clone(), None);
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
        owner: &ResourceId,
        record: &ManagedArtifactRecord,
    ) -> Result<LoadedArtifact, ManagedArtifactError> {
        self.load_with_action(owner, record, ManagedArtifactAction::Load)
    }

    fn remove(
        &self,
        owner: &ResourceId,
        handle: &ManagedArtifactHandle,
    ) -> Result<(), ManagedArtifactError> {
        validate_record(owner, handle.record()).map_err(|()| {
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

fn artifact_path(
    owner: &ResourceId,
    role: ArtifactRole,
    fingerprint: &Fingerprint,
) -> Result<RelativeArtifactPath, ()> {
    RelativeArtifactPath::new(format!(
        "artifact-{}-{}-{}-{}",
        role_component(role),
        owner_key(owner),
        fingerprint.algorithm(),
        fingerprint.digest()
    ))
    .map_err(|_| ())
}

fn validate_record(owner: &ResourceId, record: &ManagedArtifactRecord) -> Result<(), ()> {
    if record.owner() != owner {
        return Err(());
    }
    match record.role() {
        ArtifactRole::Backup => {
            let prefix = format!("backup-{}-", owner_key(owner));
            let suffix = record.path().as_str().strip_prefix(&prefix);
            let generated = suffix.and_then(|value| value.split_once('-')).is_some_and(
                |(process, sequence)| {
                    !process.is_empty()
                        && !sequence.is_empty()
                        && process.bytes().all(|byte| byte.is_ascii_digit())
                        && sequence.bytes().all(|byte| byte.is_ascii_digit())
                },
            );
            if record.fingerprint().is_some() || record.path().as_str().contains('/') || !generated
            {
                return Err(());
            }
        }
        ArtifactRole::MaterializedPlugin | ArtifactRole::DirectSkill => {
            let fingerprint = record.fingerprint().ok_or(())?;
            if record.path() != &artifact_path(owner, record.role(), fingerprint)? {
                return Err(());
            }
        }
    }
    Ok(())
}

const fn role_component(role: ArtifactRole) -> &'static str {
    match role {
        ArtifactRole::MaterializedPlugin => "materialized-plugin",
        ArtifactRole::DirectSkill => "direct-skill",
        ArtifactRole::Backup => "backup",
    }
}

fn owner_key(owner: &ResourceId) -> String {
    format!("{:x}", Sha256::digest(owner.as_str().as_bytes()))
}

#[cfg(test)]
mod tests;
