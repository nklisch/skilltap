use std::{
    fs::{self, File, OpenOptions},
    io,
    path::Path,
};

#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};

use crate::domain::AbsolutePath;

use super::{filesystem_error, unsafe_symlink};
use crate::runtime::{FileSystemAction, LockAction, RuntimeError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct FileIdentity {
    device: u64,
    inode: u64,
}

#[cfg(unix)]
pub(super) fn open_read_no_follow(path: &AbsolutePath) -> Result<File, RuntimeError> {
    open_read_no_follow_for(path, FileSystemAction::Copy)?.ok_or_else(|| {
        filesystem_error(
            FileSystemAction::Copy,
            path,
            io::Error::new(io::ErrorKind::NotFound, "source does not exist"),
        )
    })
}

#[cfg(unix)]
pub(super) fn open_read_no_follow_for(
    path: &AbsolutePath,
    action: FileSystemAction,
) -> Result<Option<File>, RuntimeError> {
    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
        .open(path.as_str())
        .map(Some)
        .map_err(|error| {
            if error.raw_os_error() == Some(libc::ELOOP) {
                unsafe_symlink(action, path)
            } else {
                filesystem_error(action, path, error)
            }
        })
        .or_else(|error| match &error {
            RuntimeError::FileSystem { source, .. } if source.kind() == io::ErrorKind::NotFound => {
                Ok(None)
            }
            _ => Err(error),
        })
}

#[cfg(not(unix))]
pub(super) fn open_read_no_follow(_path: &AbsolutePath) -> Result<File, RuntimeError> {
    Err(RuntimeError::UnsupportedPlatform {
        platform: std::env::consts::OS.to_owned(),
    })
}

#[cfg(not(unix))]
pub(super) fn open_read_no_follow_for(
    _path: &AbsolutePath,
    _action: FileSystemAction,
) -> Result<Option<File>, RuntimeError> {
    Err(RuntimeError::UnsupportedPlatform {
        platform: std::env::consts::OS.to_owned(),
    })
}

#[cfg(unix)]
pub(super) fn descriptor_identity(
    file: &File,
    action: FileSystemAction,
    path: &AbsolutePath,
) -> Result<FileIdentity, RuntimeError> {
    descriptor_identity_io(file).map_err(|error| filesystem_error(action, path, error))
}

#[cfg(not(unix))]
pub(super) fn descriptor_identity(
    _file: &File,
    _action: FileSystemAction,
    _path: &AbsolutePath,
) -> Result<FileIdentity, RuntimeError> {
    Err(RuntimeError::UnsupportedPlatform {
        platform: std::env::consts::OS.to_owned(),
    })
}

#[cfg(unix)]
pub(super) fn descriptor_identity_io(file: &File) -> io::Result<FileIdentity> {
    let metadata = file.metadata()?;
    Ok(FileIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    })
}

#[cfg(not(unix))]
pub(super) fn descriptor_identity_io(_file: &File) -> io::Result<FileIdentity> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "file identity requires a supported Unix platform",
    ))
}

#[cfg(unix)]
pub(super) fn path_identity(path: &Path) -> io::Result<FileIdentity> {
    let metadata = fs::symlink_metadata(path)?;
    Ok(FileIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    })
}

#[cfg(not(unix))]
pub(super) fn path_identity(_path: &Path) -> io::Result<FileIdentity> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "file identity requires a supported Unix platform",
    ))
}

pub(super) fn verify_path_identity(
    path: &AbsolutePath,
    expected: FileIdentity,
    action: FileSystemAction,
) -> Result<(), RuntimeError> {
    match path_identity(Path::new(path.as_str())) {
        Ok(actual) if actual == expected => Ok(()),
        Ok(_) => Err(RuntimeError::FileIdentityChanged {
            action,
            path: path.clone(),
        }),
        Err(error) => Err(filesystem_error(action, path, error)),
    }
}

#[cfg(unix)]
pub(super) fn open_directory_no_follow(path: &AbsolutePath) -> Result<File, RuntimeError> {
    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_DIRECTORY)
        .open(path.as_str())
        .map_err(|source| RuntimeError::Lock {
            action: LockAction::Acquire,
            path: path.clone(),
            source,
        })
}

#[cfg(not(unix))]
pub(super) fn open_directory_no_follow(_path: &AbsolutePath) -> Result<File, RuntimeError> {
    Err(RuntimeError::UnsupportedPlatform {
        platform: std::env::consts::OS.to_owned(),
    })
}

#[cfg(unix)]
pub(super) fn open_lock_no_follow(path: &AbsolutePath) -> Result<File, RuntimeError> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path.as_str())
        .map_err(|source| RuntimeError::Lock {
            action: LockAction::Acquire,
            path: path.clone(),
            source,
        })
}

#[cfg(not(unix))]
pub(super) fn open_lock_no_follow(_path: &AbsolutePath) -> Result<File, RuntimeError> {
    Err(RuntimeError::UnsupportedPlatform {
        platform: std::env::consts::OS.to_owned(),
    })
}

pub(super) fn verify_lock_identity(
    checked_path: &AbsolutePath,
    expected: FileIdentity,
    lock_path: &AbsolutePath,
) -> Result<(), RuntimeError> {
    match path_identity(Path::new(checked_path.as_str())) {
        Ok(actual) if actual == expected => Ok(()),
        Ok(_) | Err(_) => Err(RuntimeError::LockIdentityChanged {
            path: lock_path.clone(),
        }),
    }
}
