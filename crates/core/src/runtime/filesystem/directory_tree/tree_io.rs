use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::CString,
    fs::File,
    io::{self, Read, Write},
    os::fd::{AsRawFd, FromRawFd},
};

use crate::domain::{ArtifactFile, RelativeArtifactPath};

use super::ancestor_paths;
use super::unix_support::{
    create_dir_at_verified, cvt, directory_names, open_dir_at, open_relative_directory,
    open_relative_parent, require_directory, require_regular, stat_at, unlink_at, verify_at,
};

pub(super) fn write_tree(
    root: &File,
    files: &BTreeMap<RelativeArtifactPath, ArtifactFile>,
) -> io::Result<()> {
    let directories = files
        .keys()
        .flat_map(|path| ancestor_paths(path.as_str()))
        .collect::<BTreeSet<_>>();
    for directory in &directories {
        let path = RelativeArtifactPath::new(directory.clone())
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
        let (parent, name) = open_relative_parent(root, &path, false)?;
        let child = create_dir_at_verified(&parent, &name)?;
        parent.sync_all()?;
        child.sync_all()?;
    }
    for (path, artifact) in files {
        let (parent, name) = open_relative_parent(root, path, false)?;
        let fd = cvt(unsafe {
            libc::openat(
                parent.as_raw_fd(),
                name.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                if artifact.is_executable() {
                    0o700
                } else {
                    0o600
                },
            )
        })?;
        let mut file = unsafe { File::from_raw_fd(fd) };
        let identity = require_regular(&file)?;
        verify_at(parent.as_raw_fd(), &name, identity)?;
        file.write_all(artifact.contents())?;
        cvt(unsafe {
            libc::fchmod(
                file.as_raw_fd(),
                if artifact.is_executable() {
                    0o700
                } else {
                    0o600
                },
            )
        })?;
        file.sync_all()?;
        if require_regular(&file)? != identity {
            return Err(io::Error::other(
                "artifact file descriptor identity changed",
            ));
        }
        verify_at(parent.as_raw_fd(), &name, identity)?;
    }
    for directory in directories.iter().rev() {
        let path = RelativeArtifactPath::new(directory.clone())
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
        open_relative_directory(root, &path)?.sync_all()?;
    }
    Ok(())
}

pub(super) fn read_tree(
    directory: &File,
    prefix: Option<&str>,
    files: &mut BTreeMap<RelativeArtifactPath, ArtifactFile>,
) -> io::Result<()> {
    read_tree_with(directory, prefix, files, &mut |_| {})
}

pub(super) fn read_tree_with(
    directory: &File,
    prefix: Option<&str>,
    files: &mut BTreeMap<RelativeArtifactPath, ArtifactFile>,
    before_open: &mut impl FnMut(&str),
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
                before_open(&relative);
                let child = open_dir_at(directory.as_raw_fd(), &name)?;
                let identity = require_directory(&child)?;
                verify_at(directory.as_raw_fd(), &name, identity)?;
                read_tree_with(&child, Some(&relative), files, before_open)?;
                verify_at(directory.as_raw_fd(), &name, identity)?;
            }
            libc::S_IFREG => {
                before_open(&relative);
                let fd = cvt(unsafe {
                    libc::openat(
                        directory.as_raw_fd(),
                        name.as_ptr(),
                        libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
                    )
                })?;
                let mut file = unsafe { File::from_raw_fd(fd) };
                let identity = require_regular(&file)?;
                let executable = metadata.st_mode & 0o111 != 0;
                verify_at(directory.as_raw_fd(), &name, identity)?;
                let mut contents = Vec::new();
                file.read_to_end(&mut contents)?;
                if require_regular(&file)? != identity {
                    return Err(io::Error::other(
                        "artifact file identity changed during read",
                    ));
                }
                verify_at(directory.as_raw_fd(), &name, identity)?;
                let path = RelativeArtifactPath::new(relative)
                    .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
                files.insert(path, ArtifactFile::new(contents, executable));
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct RemovalProgress {
    pub(super) removed_any: bool,
}

#[derive(Debug)]
pub(super) struct TreeRemovalError {
    pub(super) source: io::Error,
    pub(super) removed_any: bool,
    pub(super) emptied: bool,
}

impl std::fmt::Display for TreeRemovalError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.source.fmt(formatter)
    }
}

impl TreeRemovalError {
    fn before(source: io::Error, removed_any: bool) -> Self {
        Self {
            source,
            removed_any,
            emptied: false,
        }
    }

    fn empty(source: io::Error, removed_any: bool) -> Self {
        Self {
            source,
            removed_any,
            emptied: true,
        }
    }
}

pub(super) fn remove_open_tree(directory: &File) -> io::Result<()> {
    remove_open_tree_tracked(directory)
        .map(|_| ())
        .map_err(|failure| failure.source)
}

pub(super) fn remove_open_tree_tracked(
    directory: &File,
) -> Result<RemovalProgress, TreeRemovalError> {
    remove_open_tree_with(directory, &mut |_| {})
}

pub(super) fn remove_open_tree_with(
    directory: &File,
    before_open: &mut impl FnMut(&str),
) -> Result<RemovalProgress, TreeRemovalError> {
    let mut removed_any = false;
    let names = directory_names(directory)
        .map_err(|source| TreeRemovalError::before(source, removed_any))?;
    for name in names {
        let name = CString::new(name).map_err(|_| {
            TreeRemovalError::before(io::Error::other("NUL in directory entry"), removed_any)
        })?;
        let metadata = stat_at(directory.as_raw_fd(), &name)
            .map_err(|source| TreeRemovalError::before(source, removed_any))?;
        match metadata.st_mode & libc::S_IFMT {
            libc::S_IFDIR => {
                before_open(name.to_str().unwrap_or("<non-utf8>"));
                let child = open_dir_at(directory.as_raw_fd(), &name)
                    .map_err(|source| TreeRemovalError::before(source, removed_any))?;
                let identity = require_directory(&child)
                    .map_err(|source| TreeRemovalError::before(source, removed_any))?;
                verify_at(directory.as_raw_fd(), &name, identity)
                    .map_err(|source| TreeRemovalError::before(source, removed_any))?;
                match remove_open_tree_with(&child, before_open) {
                    Ok(progress) => removed_any |= progress.removed_any,
                    Err(mut failure) => {
                        failure.removed_any |= removed_any;
                        failure.emptied = false;
                        return Err(failure);
                    }
                }
                verify_at(directory.as_raw_fd(), &name, identity)
                    .map_err(|source| TreeRemovalError::before(source, removed_any))?;
                unlink_at(directory.as_raw_fd(), &name, true)
                    .map_err(|source| TreeRemovalError::before(source, removed_any))?;
                removed_any = true;
            }
            libc::S_IFREG => {
                before_open(name.to_str().unwrap_or("<non-utf8>"));
                let fd = cvt(unsafe {
                    libc::openat(
                        directory.as_raw_fd(),
                        name.as_ptr(),
                        libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
                    )
                })
                .map_err(|source| TreeRemovalError::before(source, removed_any))?;
                let file = unsafe { File::from_raw_fd(fd) };
                let identity = require_regular(&file)
                    .map_err(|source| TreeRemovalError::before(source, removed_any))?;
                verify_at(directory.as_raw_fd(), &name, identity)
                    .map_err(|source| TreeRemovalError::before(source, removed_any))?;
                unlink_at(directory.as_raw_fd(), &name, false)
                    .map_err(|source| TreeRemovalError::before(source, removed_any))?;
                removed_any = true;
            }
            _ => {
                return Err(TreeRemovalError::before(
                    io::Error::other("refusing to remove non-regular artifact entry"),
                    removed_any,
                ));
            }
        }
    }
    directory
        .sync_all()
        .map_err(|source| TreeRemovalError::empty(source, removed_any))?;
    Ok(RemovalProgress { removed_any })
}
