use std::collections::BTreeMap;

#[cfg(unix)]
use std::{
    ffi::CString,
    fs::File,
    io::{self, Read, Write},
    os::fd::{AsRawFd, FromRawFd},
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

use crate::{
    domain::{AbsolutePath, ArtifactFile, RelativeArtifactPath},
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
    TreeRemovalError, read_tree, read_tree_bounded, remove_open_tree, remove_open_tree_tracked,
    write_tree,
};
#[cfg(all(unix, test))]
use tree_io::{read_tree_with, remove_open_tree_with};

#[cfg(unix)]
use unix_support::{
    create_dir_at_verified, lock_exclusive, open_absolute_directory,
    open_absolute_directory_preserve_mode, open_dir_at, open_relative_directory,
    open_relative_parent, require_directory, require_regular, stat_identity_at, unlink_at,
    verify_at,
};

#[cfg(unix)]
static CONFINED_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

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
        files: &BTreeMap<RelativeArtifactPath, ArtifactFile>,
    ) -> Result<DirectoryPublishOutcome, RuntimeError>;

    fn load_tree_no_follow(
        &self,
        managed_root: &AbsolutePath,
        destination: &RelativeArtifactPath,
    ) -> Result<
        (
            DirectoryIdentity,
            BTreeMap<RelativeArtifactPath, ArtifactFile>,
        ),
        RuntimeError,
    >;

    fn remove_tree_no_follow(
        &self,
        managed_root: &AbsolutePath,
        destination: &RelativeArtifactPath,
        expected: DirectoryIdentity,
    ) -> Result<DirectoryIdentity, RuntimeError>;
}

/// Descriptor-relative regular-file operations confined beneath an already
/// canonical root. Every descendant directory is opened with `O_NOFOLLOW`.
pub trait ConfinedFileSystem {
    fn read_regular_bounded_no_follow(
        &self,
        root: &AbsolutePath,
        destination: &RelativeArtifactPath,
        maximum_bytes: u64,
    ) -> Result<Option<Vec<u8>>, RuntimeError>;

    fn atomic_write_beneath_no_follow(
        &self,
        root: &AbsolutePath,
        destination: &RelativeArtifactPath,
        contents: &[u8],
    ) -> Result<(), RuntimeError>;

    fn remove_file_beneath_no_follow(
        &self,
        root: &AbsolutePath,
        destination: &RelativeArtifactPath,
    ) -> Result<(), RuntimeError>;

    fn load_tree_bounded_no_follow(
        &self,
        root: &AbsolutePath,
        destination: &RelativeArtifactPath,
        limits: crate::runtime::ExternalTreeLimits,
    ) -> Result<
        (
            DirectoryIdentity,
            BTreeMap<RelativeArtifactPath, ArtifactFile>,
        ),
        RuntimeError,
    >;
}

impl ConfinedFileSystem for SystemFileSystem {
    fn read_regular_bounded_no_follow(
        &self,
        root: &AbsolutePath,
        destination: &RelativeArtifactPath,
        maximum_bytes: u64,
    ) -> Result<Option<Vec<u8>>, RuntimeError> {
        read_confined_file(root, destination, maximum_bytes)
    }

    fn atomic_write_beneath_no_follow(
        &self,
        root: &AbsolutePath,
        destination: &RelativeArtifactPath,
        contents: &[u8],
    ) -> Result<(), RuntimeError> {
        write_confined_file(root, destination, contents)
    }

    fn remove_file_beneath_no_follow(
        &self,
        root: &AbsolutePath,
        destination: &RelativeArtifactPath,
    ) -> Result<(), RuntimeError> {
        remove_confined_file(root, destination)
    }

    fn load_tree_bounded_no_follow(
        &self,
        root: &AbsolutePath,
        destination: &RelativeArtifactPath,
        limits: crate::runtime::ExternalTreeLimits,
    ) -> Result<
        (
            DirectoryIdentity,
            BTreeMap<RelativeArtifactPath, ArtifactFile>,
        ),
        RuntimeError,
    > {
        load_confined_tree(root, destination, limits)
    }
}

#[cfg(unix)]
fn load_confined_tree(
    root: &AbsolutePath,
    destination: &RelativeArtifactPath,
    limits: crate::runtime::ExternalTreeLimits,
) -> Result<
    (
        DirectoryIdentity,
        BTreeMap<RelativeArtifactPath, ArtifactFile>,
    ),
    RuntimeError,
> {
    let root_directory = open_absolute_directory_preserve_mode(root, false)
        .map_err(|source| filesystem_error(FileSystemAction::Read, root, source))?;
    let directory = open_relative_directory(&root_directory, destination)
        .map_err(|source| filesystem_error(FileSystemAction::Read, root, source))?;
    let identity = require_directory(&directory)
        .map_err(|source| filesystem_error(FileSystemAction::Read, root, source))?;
    let mut files = BTreeMap::new();
    read_tree_bounded(&directory, &mut files, limits)
        .map_err(|source| filesystem_error(FileSystemAction::Read, root, source))?;
    Ok((identity, files))
}

#[cfg(not(unix))]
fn load_confined_tree(
    _root: &AbsolutePath,
    _destination: &RelativeArtifactPath,
    _limits: crate::runtime::ExternalTreeLimits,
) -> Result<
    (
        DirectoryIdentity,
        BTreeMap<RelativeArtifactPath, ArtifactFile>,
    ),
    RuntimeError,
> {
    Err(RuntimeError::UnsupportedPlatform {
        platform: std::env::consts::OS.to_owned(),
    })
}

impl DirectoryTreeFileSystem for SystemFileSystem {
    fn publish_tree_no_follow(
        &self,
        managed_root: &AbsolutePath,
        destination: &RelativeArtifactPath,
        files: &BTreeMap<RelativeArtifactPath, ArtifactFile>,
    ) -> Result<DirectoryPublishOutcome, RuntimeError> {
        publish_tree(managed_root, destination, files)
    }

    fn load_tree_no_follow(
        &self,
        managed_root: &AbsolutePath,
        destination: &RelativeArtifactPath,
    ) -> Result<
        (
            DirectoryIdentity,
            BTreeMap<RelativeArtifactPath, ArtifactFile>,
        ),
        RuntimeError,
    > {
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
fn read_confined_file(
    root: &AbsolutePath,
    destination: &RelativeArtifactPath,
    maximum_bytes: u64,
) -> Result<Option<Vec<u8>>, RuntimeError> {
    let root_directory = open_absolute_directory_preserve_mode(root, false)
        .map_err(|source| filesystem_error(FileSystemAction::Read, root, source))?;
    let (parent, name) = match open_relative_parent(&root_directory, destination, false) {
        Ok(value) => value,
        Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(filesystem_error(FileSystemAction::Read, root, source)),
    };
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        )
    };
    if fd == -1 {
        let source = io::Error::last_os_error();
        return if source.kind() == io::ErrorKind::NotFound {
            Ok(None)
        } else {
            Err(filesystem_error(FileSystemAction::Read, root, source))
        };
    }
    let mut file = unsafe { File::from_raw_fd(fd) };
    let identity = require_regular(&file)
        .map_err(|source| filesystem_error(FileSystemAction::Read, root, source))?;
    let length = file
        .metadata()
        .map_err(|source| filesystem_error(FileSystemAction::Read, root, source))?
        .len();
    if length > maximum_bytes {
        return Err(filesystem_error(
            FileSystemAction::Read,
            root,
            io::Error::new(
                io::ErrorKind::InvalidData,
                "confined file exceeds its byte limit",
            ),
        ));
    }
    verify_at(parent.as_raw_fd(), &name, identity)
        .map_err(|source| filesystem_error(FileSystemAction::Read, root, source))?;
    let mut contents = Vec::with_capacity(usize::try_from(length).unwrap_or(0));
    Read::by_ref(&mut file)
        .take(maximum_bytes.saturating_add(1))
        .read_to_end(&mut contents)
        .map_err(|source| filesystem_error(FileSystemAction::Read, root, source))?;
    if u64::try_from(contents.len()).unwrap_or(u64::MAX) > maximum_bytes {
        return Err(filesystem_error(
            FileSystemAction::Read,
            root,
            io::Error::new(
                io::ErrorKind::InvalidData,
                "confined file exceeds its byte limit",
            ),
        ));
    }
    if require_regular(&file).ok() != Some(identity) {
        return Err(filesystem_error(
            FileSystemAction::Read,
            root,
            io::Error::other("confined file identity changed during read"),
        ));
    }
    verify_at(parent.as_raw_fd(), &name, identity)
        .map_err(|source| filesystem_error(FileSystemAction::Read, root, source))?;
    Ok(Some(contents))
}

#[cfg(not(unix))]
fn read_confined_file(
    _root: &AbsolutePath,
    _destination: &RelativeArtifactPath,
    _maximum_bytes: u64,
) -> Result<Option<Vec<u8>>, RuntimeError> {
    Err(RuntimeError::UnsupportedPlatform {
        platform: std::env::consts::OS.to_owned(),
    })
}

#[cfg(unix)]
fn write_confined_file(
    root: &AbsolutePath,
    destination: &RelativeArtifactPath,
    contents: &[u8],
) -> Result<(), RuntimeError> {
    let root_directory = open_absolute_directory_preserve_mode(root, false)
        .map_err(|source| filesystem_error(FileSystemAction::Write, root, source))?;
    let (parent, name) = open_relative_parent(&root_directory, destination, true)
        .map_err(|source| filesystem_error(FileSystemAction::Write, root, source))?;
    let sequence = CONFINED_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary_name = CString::new(format!(
        ".{}.skilltap-confined-{}-{sequence}",
        name.to_string_lossy(),
        std::process::id()
    ))
    .map_err(|_| {
        filesystem_error(
            FileSystemAction::Write,
            root,
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "NUL in confined temporary name",
            ),
        )
    })?;
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            temporary_name.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            0o600,
        )
    };
    if fd == -1 {
        return Err(filesystem_error(
            FileSystemAction::Write,
            root,
            io::Error::last_os_error(),
        ));
    }
    let mut temporary = unsafe { File::from_raw_fd(fd) };
    let identity = require_regular(&temporary)
        .map_err(|source| filesystem_error(FileSystemAction::Write, root, source))?;
    let result = temporary
        .write_all(contents)
        .and_then(|()| temporary.sync_all())
        .and_then(|()| verify_at(parent.as_raw_fd(), &temporary_name, identity))
        .and_then(|()| {
            let result = unsafe {
                libc::renameat(
                    parent.as_raw_fd(),
                    temporary_name.as_ptr(),
                    parent.as_raw_fd(),
                    name.as_ptr(),
                )
            };
            if result == -1 {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            }
        })
        .and_then(|()| verify_at(parent.as_raw_fd(), &name, identity))
        .and_then(|()| parent.sync_all());
    if let Err(source) = result {
        let _ = unlink_at(parent.as_raw_fd(), &temporary_name, false);
        return Err(filesystem_error(FileSystemAction::Write, root, source));
    }
    Ok(())
}

#[cfg(not(unix))]
fn write_confined_file(
    _root: &AbsolutePath,
    _destination: &RelativeArtifactPath,
    _contents: &[u8],
) -> Result<(), RuntimeError> {
    Err(RuntimeError::UnsupportedPlatform {
        platform: std::env::consts::OS.to_owned(),
    })
}

#[cfg(unix)]
fn remove_confined_file(
    root: &AbsolutePath,
    destination: &RelativeArtifactPath,
) -> Result<(), RuntimeError> {
    let root_directory = open_absolute_directory_preserve_mode(root, false)
        .map_err(|source| filesystem_error(FileSystemAction::Remove, root, source))?;
    let (parent, name) = match open_relative_parent(&root_directory, destination, false) {
        Ok(value) => value,
        Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(source) => return Err(filesystem_error(FileSystemAction::Remove, root, source)),
    };
    match stat_identity_at(parent.as_raw_fd(), &name) {
        Ok(_) => {}
        Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(source) => return Err(filesystem_error(FileSystemAction::Remove, root, source)),
    }
    unlink_at(parent.as_raw_fd(), &name, false)
        .and_then(|()| parent.sync_all())
        .map_err(|source| filesystem_error(FileSystemAction::Remove, root, source))
}

#[cfg(not(unix))]
fn remove_confined_file(
    _root: &AbsolutePath,
    _destination: &RelativeArtifactPath,
) -> Result<(), RuntimeError> {
    Err(RuntimeError::UnsupportedPlatform {
        platform: std::env::consts::OS.to_owned(),
    })
}

#[cfg(unix)]
fn publish_tree(
    managed_root: &AbsolutePath,
    destination: &RelativeArtifactPath,
    files: &BTreeMap<RelativeArtifactPath, ArtifactFile>,
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
    _files: &BTreeMap<RelativeArtifactPath, ArtifactFile>,
) -> Result<DirectoryPublishOutcome, RuntimeError> {
    Err(RuntimeError::UnsupportedPlatform {
        platform: std::env::consts::OS.to_owned(),
    })
}

#[cfg(unix)]
fn load_tree(
    managed_root: &AbsolutePath,
    destination: &RelativeArtifactPath,
) -> Result<
    (
        DirectoryIdentity,
        BTreeMap<RelativeArtifactPath, ArtifactFile>,
    ),
    RuntimeError,
> {
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
) -> Result<
    (
        DirectoryIdentity,
        BTreeMap<RelativeArtifactPath, ArtifactFile>,
    ),
    RuntimeError,
> {
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
