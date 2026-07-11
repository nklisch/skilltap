use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::CString,
    fs::File,
    io::{self, Read, Write},
    os::fd::{AsRawFd, FromRawFd},
};

use crate::domain::RelativeArtifactPath;

use super::ancestor_paths;
use super::unix_support::{
    create_dir_at_verified, cvt, directory_names, open_dir_at, open_relative_directory,
    open_relative_parent, require_directory, require_regular, stat_at, unlink_at, verify_at,
};

pub(super) fn write_tree(
    root: &File,
    files: &BTreeMap<RelativeArtifactPath, Vec<u8>>,
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
        let identity = require_regular(&file)?;
        verify_at(parent.as_raw_fd(), &name, identity)?;
        file.write_all(contents)?;
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
    files: &mut BTreeMap<RelativeArtifactPath, Vec<u8>>,
) -> io::Result<()> {
    read_tree_with(directory, prefix, files, &mut |_| {})
}

pub(super) fn read_tree_with(
    directory: &File,
    prefix: Option<&str>,
    files: &mut BTreeMap<RelativeArtifactPath, Vec<u8>>,
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

pub(super) fn remove_open_tree(directory: &File) -> io::Result<()> {
    remove_open_tree_with(directory, &mut |_| {})
}

pub(super) fn remove_open_tree_with(
    directory: &File,
    before_open: &mut impl FnMut(&str),
) -> io::Result<()> {
    for name in directory_names(directory)? {
        let name = CString::new(name).map_err(|_| io::Error::other("NUL in directory entry"))?;
        let metadata = stat_at(directory.as_raw_fd(), &name)?;
        match metadata.st_mode & libc::S_IFMT {
            libc::S_IFDIR => {
                before_open(name.to_str().unwrap_or("<non-utf8>"));
                let child = open_dir_at(directory.as_raw_fd(), &name)?;
                let identity = require_directory(&child)?;
                verify_at(directory.as_raw_fd(), &name, identity)?;
                remove_open_tree_with(&child, before_open)?;
                verify_at(directory.as_raw_fd(), &name, identity)?;
                unlink_at(directory.as_raw_fd(), &name, true)?;
            }
            libc::S_IFREG => {
                before_open(name.to_str().unwrap_or("<non-utf8>"));
                let fd = cvt(unsafe {
                    libc::openat(
                        directory.as_raw_fd(),
                        name.as_ptr(),
                        libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
                    )
                })?;
                let file = unsafe { File::from_raw_fd(fd) };
                let identity = require_regular(&file)?;
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
