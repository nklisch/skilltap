use std::{collections::BTreeMap, fmt};

use crate::{
    domain::{
        AbsolutePath, ArtifactFile, Fingerprint, RelativeArtifactPath, ResourceId, ResourceKey,
        Scope,
    },
    runtime::{
        DirectoryContentState, DirectoryIdentity, DirectoryPathState, DirectorySyncState,
        DirectoryTreeFileSystem,
    },
};

use super::{ArtifactRole, ManagedArtifactRecord};

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
    files: BTreeMap<RelativeArtifactPath, ArtifactFile>,
}

impl ArtifactTree {
    pub fn new<P, F>(files: impl IntoIterator<Item = (P, F)>) -> Result<Self, ArtifactTreeError>
    where
        P: Into<String>,
        F: Into<ArtifactFile>,
    {
        Ok(Self {
            files: tree_validation::validate(files)?,
        })
    }

    pub const fn files(&self) -> &BTreeMap<RelativeArtifactPath, ArtifactFile> {
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
    PartialRemoval,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedRemovalResidual {
    owner: ResourceKey,
    path: RelativeArtifactPath,
    expected_identity: DirectoryIdentity,
    observed_identity: Option<DirectoryIdentity>,
    presence: DirectoryPathState,
    content: DirectoryContentState,
    parent_sync: DirectorySyncState,
}

impl ManagedRemovalResidual {
    pub fn owner(&self) -> &ResourceKey {
        &self.owner
    }

    pub fn path(&self) -> &RelativeArtifactPath {
        &self.path
    }

    pub const fn expected_identity(&self) -> DirectoryIdentity {
        self.expected_identity
    }

    pub const fn observed_identity(&self) -> Option<DirectoryIdentity> {
        self.observed_identity
    }

    pub const fn presence(&self) -> DirectoryPathState {
        self.presence
    }

    pub const fn content(&self) -> DirectoryContentState {
        self.content
    }

    pub const fn parent_sync(&self) -> DirectorySyncState {
        self.parent_sync
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedArtifactResidual {
    owner: ResourceKey,
    path: RelativeArtifactPath,
    identity: Option<DirectoryIdentity>,
    presence: crate::runtime::DirectoryPathState,
    parent_sync: crate::runtime::DirectorySyncState,
}

impl ManagedArtifactResidual {
    pub fn owner(&self) -> &ResourceKey {
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
    owner: ResourceKey,
    path: Option<RelativeArtifactPath>,
    failure: ManagedArtifactFailure,
    residual: Option<Box<ManagedArtifactResidual>>,
    removal_residual: Option<Box<ManagedRemovalResidual>>,
}

impl ManagedArtifactError {
    pub const fn action(&self) -> ManagedArtifactAction {
        self.action
    }

    pub fn owner(&self) -> &ResourceKey {
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

    pub fn removal_residual(&self) -> Option<&ManagedRemovalResidual> {
        self.removal_residual.as_deref()
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
            .field("removal_residual", &self.removal_residual)
            .finish()
    }
}

impl fmt::Display for ManagedArtifactError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "managed artifact {:?} for `{:?}` failed",
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
        owner: &ResourceKey,
        role: ArtifactRole,
        fingerprint: &Fingerprint,
        tree: &ArtifactTree,
    ) -> Result<ArtifactPublication, ManagedArtifactError>;

    fn backup(
        &self,
        owner: &ResourceKey,
        tree: &ArtifactTree,
    ) -> Result<ManagedArtifactHandle, ManagedArtifactError>;

    fn load(
        &self,
        owner: &ResourceKey,
        record: &ManagedArtifactRecord,
    ) -> Result<LoadedArtifact, ManagedArtifactError>;

    fn remove(
        &self,
        owner: &ResourceKey,
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
        let owner = ResourceKey::new(
            ResourceId::new("skilltap:managed").expect("static managed-root owner is valid"),
            Scope::Global,
        );
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
}

mod error_translation;
mod repository;
mod tree_validation;

#[cfg(test)]
mod tests;
