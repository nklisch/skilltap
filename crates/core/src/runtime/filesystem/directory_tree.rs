use std::collections::BTreeMap;

#[cfg(unix)]
use std::{ffi::CString, fs::File, io, os::fd::AsRawFd, path::Path};

use crate::{
    domain::{AbsolutePath, RelativeArtifactPath},
    runtime::{
        DirectoryContentState, DirectoryIdentity, DirectoryPathState, DirectorySyncState,
        FileSystemAction, RuntimeError,
    },
};

use super::{SystemFileSystem, filesystem_error};

#[cfg(unix)]
mod tree_io;
#[cfg(unix)]
mod unix_support;

#[cfg(unix)]
use tree_io::{
    TreeRemovalError, read_tree, remove_open_tree, remove_open_tree_tracked, write_tree,
};
#[cfg(all(unix, test))]
use tree_io::{read_tree_with, remove_open_tree_with};

#[cfg(unix)]
use unix_support::{
    create_dir_at_verified, lock_exclusive, open_absolute_directory, open_dir_at,
    open_relative_directory, open_relative_parent, require_directory, stat_identity_at, unlink_at,
    verify_at,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DirectoryPublishOutcome {
    Published(DirectoryIdentity),
    AlreadyExists,
}

pub trait DirectoryTreeFileSystem {
    fn publish_tree_no_follow(
        &self,
        managed_root: &AbsolutePath,
        destination: &RelativeArtifactPath,
        files: &BTreeMap<RelativeArtifactPath, Vec<u8>>,
    ) -> Result<DirectoryPublishOutcome, RuntimeError>;

    fn load_tree_no_follow(
        &self,
        managed_root: &AbsolutePath,
        destination: &RelativeArtifactPath,
    ) -> Result<(DirectoryIdentity, BTreeMap<RelativeArtifactPath, Vec<u8>>), RuntimeError>;

    fn remove_tree_no_follow(
        &self,
        managed_root: &AbsolutePath,
        destination: &RelativeArtifactPath,
        expected: DirectoryIdentity,
    ) -> Result<DirectoryIdentity, RuntimeError>;
}

impl DirectoryTreeFileSystem for SystemFileSystem {
    fn publish_tree_no_follow(
        &self,
        managed_root: &AbsolutePath,
        destination: &RelativeArtifactPath,
        files: &BTreeMap<RelativeArtifactPath, Vec<u8>>,
    ) -> Result<DirectoryPublishOutcome, RuntimeError> {
        publish_tree(managed_root, destination, files)
    }

    fn load_tree_no_follow(
        &self,
        managed_root: &AbsolutePath,
        destination: &RelativeArtifactPath,
    ) -> Result<(DirectoryIdentity, BTreeMap<RelativeArtifactPath, Vec<u8>>), RuntimeError> {
        load_tree(managed_root, destination)
    }

    fn remove_tree_no_follow(
        &self,
        managed_root: &AbsolutePath,
        destination: &RelativeArtifactPath,
        expected: DirectoryIdentity,
    ) -> Result<DirectoryIdentity, RuntimeError> {
        remove_tree(managed_root, destination, expected)
    }
}

#[cfg(unix)]
fn publish_tree(
    managed_root: &AbsolutePath,
    destination: &RelativeArtifactPath,
    files: &BTreeMap<RelativeArtifactPath, Vec<u8>>,
) -> Result<DirectoryPublishOutcome, RuntimeError> {
    let root = open_absolute_directory(managed_root, true).map_err(|source| {
        filesystem_error(FileSystemAction::CreateDirectory, managed_root, source)
    })?;
    let _lock = lock_exclusive(&root).map_err(|source| {
        filesystem_error(FileSystemAction::CreateDirectory, managed_root, source)
    })?;
    let (parent, name) = open_relative_parent(&root, destination, true).map_err(|source| {
        filesystem_error(FileSystemAction::CreateDirectory, managed_root, source)
    })?;
    let directory = match create_dir_at_verified(&parent, &name) {
        Ok(directory) => directory,
        Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {
            return Ok(DirectoryPublishOutcome::AlreadyExists);
        }
        Err(source) => {
            let (presence, identity) = observe_at(&parent, &name);
            return if presence == DirectoryPathState::Removed {
                Err(filesystem_error(
                    FileSystemAction::CreateDirectory,
                    managed_root,
                    source,
                ))
            } else {
                Err(RuntimeError::PartialDirectoryPublication {
                    path: join_absolute(managed_root, destination),
                    identity,
                    presence,
                    parent_sync: DirectorySyncState::Uncertain,
                    source,
                    cleanup: io::Error::other(
                        "created destination could not be proven safe for cleanup",
                    ),
                })
            };
        }
    };
    let identity = require_directory(&directory).map_err(|source| {
        RuntimeError::PartialDirectoryPublication {
            path: join_absolute(managed_root, destination),
            identity: None,
            presence: DirectoryPathState::Present,
            parent_sync: DirectorySyncState::Uncertain,
            source,
            cleanup: io::Error::other("opened destination is not an owned directory"),
        }
    })?;
    verify_at(parent.as_raw_fd(), &name, identity).map_err(|source| {
        let (presence, observed) = observe_at(&parent, &name);
        RuntimeError::PartialDirectoryPublication {
            path: join_absolute(managed_root, destination),
            identity: observed,
            presence,
            parent_sync: DirectorySyncState::Uncertain,
            source,
            cleanup: io::Error::other("destination changed before publication writes"),
        }
    })?;

    if let Err(source) = parent.sync_all() {
        return Err(clean_publication_failure(
            managed_root,
            destination,
            &parent,
            &name,
            &directory,
            identity,
            DirectorySyncState::Uncertain,
            source,
        ));
    }

    let result = write_tree(&directory, files).and_then(|()| {
        let actual = require_directory(&directory)?;
        if actual != identity {
            return Err(io::Error::other(
                "destination descriptor identity changed during publication",
            ));
        }
        verify_at(parent.as_raw_fd(), &name, identity)?;
        directory.sync_all()?;
        parent.sync_all()
    });
    match result {
        Ok(()) => Ok(DirectoryPublishOutcome::Published(identity)),
        Err(source) => Err(clean_publication_failure(
            managed_root,
            destination,
            &parent,
            &name,
            &directory,
            identity,
            DirectorySyncState::Synced,
            source,
        )),
    }
}

#[cfg(not(unix))]
fn publish_tree(
    _managed_root: &AbsolutePath,
    _destination: &RelativeArtifactPath,
    _files: &BTreeMap<RelativeArtifactPath, Vec<u8>>,
) -> Result<DirectoryPublishOutcome, RuntimeError> {
    Err(RuntimeError::UnsupportedPlatform {
        platform: std::env::consts::OS.to_owned(),
    })
}

#[cfg(unix)]
fn load_tree(
    managed_root: &AbsolutePath,
    destination: &RelativeArtifactPath,
) -> Result<(DirectoryIdentity, BTreeMap<RelativeArtifactPath, Vec<u8>>), RuntimeError> {
    let root = open_absolute_directory(managed_root, false)
        .map_err(|source| filesystem_error(FileSystemAction::Read, managed_root, source))?;
    let directory = open_relative_directory(&root, destination)
        .map_err(|source| filesystem_error(FileSystemAction::Read, managed_root, source))?;
    let identity = require_directory(&directory)
        .map_err(|source| filesystem_error(FileSystemAction::Read, managed_root, source))?;
    let mut files = BTreeMap::new();
    read_tree(&directory, None, &mut files)
        .map_err(|source| filesystem_error(FileSystemAction::Read, managed_root, source))?;
    Ok((identity, files))
}

#[cfg(not(unix))]
fn load_tree(
    _managed_root: &AbsolutePath,
    _destination: &RelativeArtifactPath,
) -> Result<(DirectoryIdentity, BTreeMap<RelativeArtifactPath, Vec<u8>>), RuntimeError> {
    Err(RuntimeError::UnsupportedPlatform {
        platform: std::env::consts::OS.to_owned(),
    })
}

#[cfg(unix)]
fn remove_tree(
    managed_root: &AbsolutePath,
    destination: &RelativeArtifactPath,
    expected: DirectoryIdentity,
) -> Result<DirectoryIdentity, RuntimeError> {
    remove_tree_with(
        managed_root,
        destination,
        expected,
        |parent, name| unlink_at(parent.as_raw_fd(), name, true),
        File::sync_all,
    )
}

#[cfg(unix)]
fn remove_tree_with(
    managed_root: &AbsolutePath,
    destination: &RelativeArtifactPath,
    expected: DirectoryIdentity,
    unlink_top: impl FnOnce(&File, &CString) -> io::Result<()>,
    sync_parent: impl FnOnce(&File) -> io::Result<()>,
) -> Result<DirectoryIdentity, RuntimeError> {
    let root = open_absolute_directory(managed_root, false)
        .map_err(|source| filesystem_error(FileSystemAction::Remove, managed_root, source))?;
    let _lock = lock_exclusive(&root)
        .map_err(|source| filesystem_error(FileSystemAction::Remove, managed_root, source))?;
    let (parent, name) = open_relative_parent(&root, destination, false)
        .map_err(|source| filesystem_error(FileSystemAction::Remove, managed_root, source))?;
    let path = join_absolute(managed_root, destination);
    let directory = open_dir_at(parent.as_raw_fd(), &name).map_err(|source| {
        let (presence, observed) = observe_at(&parent, &name);
        partial_removal(
            path.clone(),
            expected,
            observed,
            presence,
            DirectoryContentState::Unknown,
            DirectorySyncState::NotRequired,
            source,
        )
    })?;
    let identity = require_directory(&directory).map_err(|source| {
        let (presence, observed) = observe_at(&parent, &name);
        partial_removal(
            path.clone(),
            expected,
            observed,
            presence,
            DirectoryContentState::Unknown,
            DirectorySyncState::NotRequired,
            source,
        )
    })?;
    if identity != expected {
        let (presence, observed) = observe_at(&parent, &name);
        return Err(partial_removal(
            path,
            expected,
            observed,
            presence,
            DirectoryContentState::Intact,
            DirectorySyncState::NotRequired,
            io::Error::other("destination identity changed before removal"),
        ));
    }
    if let Err(failure) = remove_open_tree_tracked(&directory) {
        let (presence, observed) = observe_at(&parent, &name);
        let content = removal_content(&failure, expected, presence, observed);
        return Err(partial_removal(
            path,
            expected,
            observed,
            presence,
            content,
            DirectorySyncState::NotRequired,
            failure.source,
        ));
    }
    if let Err(source) = verify_at(parent.as_raw_fd(), &name, identity) {
        let (presence, observed) = observe_at(&parent, &name);
        let content = if presence == DirectoryPathState::Present && observed == Some(expected) {
            DirectoryContentState::Empty
        } else {
            DirectoryContentState::Unknown
        };
        return Err(partial_removal(
            path,
            expected,
            observed,
            presence,
            content,
            DirectorySyncState::NotRequired,
            source,
        ));
    }
    if let Err(source) = unlink_top(&parent, &name) {
        let (presence, observed) = observe_at(&parent, &name);
        let content = if presence == DirectoryPathState::Present && observed == Some(expected) {
            DirectoryContentState::Empty
        } else {
            DirectoryContentState::Unknown
        };
        return Err(partial_removal(
            path,
            expected,
            observed,
            presence,
            content,
            DirectorySyncState::NotRequired,
            source,
        ));
    }
    if let Err(source) = sync_parent(&parent) {
        let (presence, observed) = observe_at(&parent, &name);
        let content = if presence == DirectoryPathState::Removed {
            DirectoryContentState::Empty
        } else {
            DirectoryContentState::Unknown
        };
        return Err(partial_removal(
            path,
            expected,
            observed,
            presence,
            content,
            DirectorySyncState::Uncertain,
            source,
        ));
    }
    Ok(identity)
}

#[cfg(unix)]
fn removal_content(
    failure: &TreeRemovalError,
    expected: DirectoryIdentity,
    presence: DirectoryPathState,
    observed: Option<DirectoryIdentity>,
) -> DirectoryContentState {
    if presence != DirectoryPathState::Present || observed != Some(expected) {
        return DirectoryContentState::Unknown;
    }
    if failure.emptied {
        DirectoryContentState::Empty
    } else if failure.removed_any {
        DirectoryContentState::Partial
    } else {
        DirectoryContentState::Intact
    }
}

#[cfg(unix)]
#[allow(clippy::too_many_arguments)]
fn partial_removal(
    path: AbsolutePath,
    expected: DirectoryIdentity,
    observed: Option<DirectoryIdentity>,
    presence: DirectoryPathState,
    content: DirectoryContentState,
    parent_sync: DirectorySyncState,
    source: io::Error,
) -> RuntimeError {
    RuntimeError::PartialDirectoryRemoval {
        path,
        expected,
        observed,
        presence,
        content,
        parent_sync,
        source,
    }
}

#[cfg(not(unix))]
fn remove_tree(
    _managed_root: &AbsolutePath,
    _destination: &RelativeArtifactPath,
    _expected: DirectoryIdentity,
) -> Result<DirectoryIdentity, RuntimeError> {
    Err(RuntimeError::UnsupportedPlatform {
        platform: std::env::consts::OS.to_owned(),
    })
}

#[cfg(unix)]
#[allow(clippy::too_many_arguments)]
fn clean_publication_failure(
    managed_root: &AbsolutePath,
    destination: &RelativeArtifactPath,
    parent: &File,
    name: &CString,
    directory: &File,
    identity: DirectoryIdentity,
    prior_parent_sync: DirectorySyncState,
    source: io::Error,
) -> RuntimeError {
    clean_publication_failure_with_parent_sync(
        managed_root,
        destination,
        parent,
        name,
        directory,
        identity,
        prior_parent_sync,
        source,
        || parent.sync_all(),
    )
}

#[cfg(unix)]
#[allow(clippy::too_many_arguments)]
fn clean_publication_failure_with_parent_sync(
    managed_root: &AbsolutePath,
    destination: &RelativeArtifactPath,
    parent: &File,
    name: &CString,
    directory: &File,
    identity: DirectoryIdentity,
    prior_parent_sync: DirectorySyncState,
    source: io::Error,
    sync_parent: impl FnOnce() -> io::Result<()>,
) -> RuntimeError {
    let cleanup = remove_open_tree(directory)
        .and_then(|()| verify_at(parent.as_raw_fd(), name, identity))
        .and_then(|()| unlink_at(parent.as_raw_fd(), name, true));
    match cleanup {
        Ok(()) => match sync_parent() {
            Ok(()) => filesystem_error(FileSystemAction::Write, managed_root, source),
            Err(cleanup) => RuntimeError::PartialDirectoryPublication {
                path: join_absolute(managed_root, destination),
                identity: None,
                presence: DirectoryPathState::Removed,
                parent_sync: DirectorySyncState::Uncertain,
                source,
                cleanup,
            },
        },
        Err(cleanup) => {
            let (presence, observed) = observe_at(parent, name);
            RuntimeError::PartialDirectoryPublication {
                path: join_absolute(managed_root, destination),
                identity: observed,
                presence,
                parent_sync: prior_parent_sync,
                source,
                cleanup,
            }
        }
    }
}

#[cfg(unix)]
fn observe_at(parent: &File, name: &CString) -> (DirectoryPathState, Option<DirectoryIdentity>) {
    match stat_identity_at(parent.as_raw_fd(), name) {
        Ok(identity) => (DirectoryPathState::Present, Some(identity)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            (DirectoryPathState::Removed, None)
        }
        Err(_) => (DirectoryPathState::Unknown, None),
    }
}

fn ancestor_paths(path: &str) -> Vec<String> {
    let mut ancestors = Vec::new();
    let mut current = Path::new(path).parent();
    while let Some(path) = current {
        if path.as_os_str().is_empty() {
            break;
        }
        ancestors.push(path.to_string_lossy().into_owned());
        current = path.parent();
    }
    ancestors.reverse();
    ancestors
}

fn join_absolute(root: &AbsolutePath, path: &RelativeArtifactPath) -> AbsolutePath {
    AbsolutePath::new(format!("{}/{}", root.as_str(), path.as_str()))
        .expect("validated absolute root plus relative artifact path remains valid")
}

#[cfg(test)]
mod tests;
