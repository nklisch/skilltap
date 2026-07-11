use std::collections::BTreeMap;

#[cfg(unix)]
use std::{
    collections::BTreeSet,
    ffi::CString,
    fs::File,
    io::{self, Read, Write},
    os::fd::{AsRawFd, FromRawFd},
    path::Path,
};

use crate::{
    domain::{AbsolutePath, RelativeArtifactPath},
    runtime::{DirectoryIdentity, FileSystemAction, RuntimeError},
};

use super::{SystemFileSystem, filesystem_error};

#[cfg(unix)]
mod unix_support;

#[cfg(unix)]
use unix_support::{
    cvt, directory_identity, directory_names, mkdir_at, open_absolute_directory, open_dir_at,
    open_relative_directory, open_relative_parent, stat_at, unlink_at, verify_at,
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
    let (parent, name) = open_relative_parent(&root, destination, true).map_err(|source| {
        filesystem_error(FileSystemAction::CreateDirectory, managed_root, source)
    })?;
    match mkdir_at(parent.as_raw_fd(), &name) {
        Ok(()) => {}
        Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {
            return Ok(DirectoryPublishOutcome::AlreadyExists);
        }
        Err(source) => {
            return Err(filesystem_error(
                FileSystemAction::CreateDirectory,
                managed_root,
                source,
            ));
        }
    }
    let directory = open_dir_at(parent.as_raw_fd(), &name).map_err(|source| {
        filesystem_error(FileSystemAction::CreateDirectory, managed_root, source)
    })?;
    let identity = directory_identity(&directory).map_err(|source| {
        filesystem_error(FileSystemAction::CreateDirectory, managed_root, source)
    })?;

    let result = write_tree(&directory, files).and_then(|()| {
        directory.sync_all()?;
        parent.sync_all()
    });
    match result {
        Ok(()) => Ok(DirectoryPublishOutcome::Published(identity)),
        Err(source) => {
            let cleanup = remove_open_tree(&directory).and_then(|()| {
                verify_at(parent.as_raw_fd(), &name, identity)?;
                unlink_at(parent.as_raw_fd(), &name, true)?;
                parent.sync_all()
            });
            match cleanup {
                Ok(()) => Err(filesystem_error(
                    FileSystemAction::Write,
                    managed_root,
                    source,
                )),
                Err(cleanup) => Err(RuntimeError::PartialDirectoryPublication {
                    path: join_absolute(managed_root, destination),
                    identity,
                    source,
                    cleanup,
                }),
            }
        }
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
    let identity = directory_identity(&directory)
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
    let root = open_absolute_directory(managed_root, false)
        .map_err(|source| filesystem_error(FileSystemAction::Remove, managed_root, source))?;
    let (parent, name) = open_relative_parent(&root, destination, false)
        .map_err(|source| filesystem_error(FileSystemAction::Remove, managed_root, source))?;
    let directory = open_dir_at(parent.as_raw_fd(), &name)
        .map_err(|source| filesystem_error(FileSystemAction::Remove, managed_root, source))?;
    let identity = directory_identity(&directory)
        .map_err(|source| filesystem_error(FileSystemAction::Remove, managed_root, source))?;
    if identity != expected {
        return Err(RuntimeError::FileIdentityChanged {
            action: FileSystemAction::Remove,
            path: join_absolute(managed_root, destination),
        });
    }
    remove_open_tree(&directory)
        .and_then(|()| verify_at(parent.as_raw_fd(), &name, identity))
        .and_then(|()| unlink_at(parent.as_raw_fd(), &name, true))
        .and_then(|()| parent.sync_all())
        .map_err(|source| filesystem_error(FileSystemAction::Remove, managed_root, source))?;
    Ok(identity)
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
fn write_tree(root: &File, files: &BTreeMap<RelativeArtifactPath, Vec<u8>>) -> io::Result<()> {
    let directories = files
        .keys()
        .flat_map(|path| ancestor_paths(path.as_str()))
        .collect::<BTreeSet<_>>();
    for directory in &directories {
        let path = RelativeArtifactPath::new(directory.clone())
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
        let (parent, name) = open_relative_parent(root, &path, false)?;
        mkdir_at(parent.as_raw_fd(), &name)?;
        parent.sync_all()?;
    }
    for (path, contents) in files {
        let (parent, name) = open_relative_parent(root, path, false)?;
        let fd = cvt(unsafe {
            libc::openat(
                parent.as_raw_fd(),
                name.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                0o600,
            )
        })?;
        let mut file = unsafe { File::from_raw_fd(fd) };
        file.write_all(contents)?;
        file.sync_all()?;
        let identity = directory_identity(&file)?;
        verify_at(parent.as_raw_fd(), &name, identity)?;
    }
    for directory in directories.iter().rev() {
        let path = RelativeArtifactPath::new(directory.clone())
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
        open_relative_directory(root, &path)?.sync_all()?;
    }
    Ok(())
}

#[cfg(unix)]
fn read_tree(
    directory: &File,
    prefix: Option<&str>,
    files: &mut BTreeMap<RelativeArtifactPath, Vec<u8>>,
) -> io::Result<()> {
    let names = directory_names(directory)?;
    if prefix.is_some() && names.is_empty() {
        return Err(io::Error::other(
            "artifact tree contains an empty directory",
        ));
    }
    for name in names {
        let relative = match prefix {
            Some(prefix) => format!("{prefix}/{name}"),
            None => name.clone(),
        };
        let name = CString::new(name).map_err(|_| io::Error::other("NUL in directory entry"))?;
        let metadata = stat_at(directory.as_raw_fd(), &name)?;
        match metadata.st_mode & libc::S_IFMT {
            libc::S_IFDIR => {
                let child = open_dir_at(directory.as_raw_fd(), &name)?;
                let identity = directory_identity(&child)?;
                read_tree(&child, Some(&relative), files)?;
                verify_at(directory.as_raw_fd(), &name, identity)?;
            }
            libc::S_IFREG => {
                let fd = cvt(unsafe {
                    libc::openat(
                        directory.as_raw_fd(),
                        name.as_ptr(),
                        libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
                    )
                })?;
                let mut file = unsafe { File::from_raw_fd(fd) };
                let identity = directory_identity(&file)?;
                let mut contents = Vec::new();
                file.read_to_end(&mut contents)?;
                verify_at(directory.as_raw_fd(), &name, identity)?;
                let path = RelativeArtifactPath::new(relative)
                    .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
                files.insert(path, contents);
            }
            _ => {
                return Err(io::Error::other(
                    "artifact tree contains a non-regular entry",
                ));
            }
        }
    }
    Ok(())
}

#[cfg(unix)]
fn remove_open_tree(directory: &File) -> io::Result<()> {
    for name in directory_names(directory)? {
        let name = CString::new(name).map_err(|_| io::Error::other("NUL in directory entry"))?;
        let metadata = stat_at(directory.as_raw_fd(), &name)?;
        match metadata.st_mode & libc::S_IFMT {
            libc::S_IFDIR => {
                let child = open_dir_at(directory.as_raw_fd(), &name)?;
                let identity = directory_identity(&child)?;
                remove_open_tree(&child)?;
                verify_at(directory.as_raw_fd(), &name, identity)?;
                unlink_at(directory.as_raw_fd(), &name, true)?;
            }
            libc::S_IFREG => {
                let fd = cvt(unsafe {
                    libc::openat(
                        directory.as_raw_fd(),
                        name.as_ptr(),
                        libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
                    )
                })?;
                let file = unsafe { File::from_raw_fd(fd) };
                let identity = directory_identity(&file)?;
                verify_at(directory.as_raw_fd(), &name, identity)?;
                unlink_at(directory.as_raw_fd(), &name, false)?;
            }
            _ => {
                return Err(io::Error::other(
                    "refusing to remove non-regular artifact entry",
                ));
            }
        }
    }
    directory.sync_all()
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
