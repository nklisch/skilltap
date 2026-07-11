use std::{
    ffi::{CStr, CString, OsString},
    fs::{File, TryLockError},
    io,
    os::{
        fd::{AsRawFd, FromRawFd, RawFd},
        unix::{ffi::OsStringExt, fs::MetadataExt},
    },
    path::{Component, Path},
};

use crate::{
    domain::{AbsolutePath, RelativeArtifactPath},
    runtime::DirectoryIdentity,
};

pub(super) fn open_absolute_directory(path: &AbsolutePath, create: bool) -> io::Result<File> {
    let mut directory = File::open("/")?;
    for component in Path::new(path.as_str()).components() {
        let Component::Normal(component) = component else {
            continue;
        };
        let name = CString::new(component.as_encoded_bytes())
            .map_err(|_| io::Error::other("NUL in path component"))?;
        match open_dir_at(directory.as_raw_fd(), &name) {
            Ok(next) => directory = next,
            Err(error) if create && error.kind() == io::ErrorKind::NotFound => {
                let next = create_dir_at_verified(&directory, &name)?;
                directory.sync_all()?;
                directory = next;
            }
            Err(error) => return Err(error),
        }
    }
    Ok(directory)
}

pub(super) fn open_relative_directory(
    root: &File,
    path: &RelativeArtifactPath,
) -> io::Result<File> {
    let mut directory = root.try_clone()?;
    for component in Path::new(path.as_str()).components() {
        let Component::Normal(component) = component else {
            return Err(io::Error::other("invalid relative artifact component"));
        };
        let name = CString::new(component.as_encoded_bytes())
            .map_err(|_| io::Error::other("NUL in path component"))?;
        directory = open_dir_at(directory.as_raw_fd(), &name)?;
    }
    Ok(directory)
}

pub(super) fn open_relative_parent(
    root: &File,
    path: &RelativeArtifactPath,
    create: bool,
) -> io::Result<(File, CString)> {
    let mut components = Path::new(path.as_str()).components().peekable();
    let mut directory = root.try_clone()?;
    while let Some(component) = components.next() {
        let Component::Normal(component) = component else {
            return Err(io::Error::other("invalid relative artifact component"));
        };
        let name = CString::new(component.as_encoded_bytes())
            .map_err(|_| io::Error::other("NUL in path component"))?;
        if components.peek().is_none() {
            return Ok((directory, name));
        }
        match open_dir_at(directory.as_raw_fd(), &name) {
            Ok(next) => directory = next,
            Err(error) if create && error.kind() == io::ErrorKind::NotFound => {
                let next = create_dir_at_verified(&directory, &name)?;
                directory.sync_all()?;
                directory = next;
            }
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::other("artifact destination is empty"))
}

pub(super) fn directory_names(directory: &File) -> io::Result<Vec<String>> {
    let duplicate = cvt(unsafe { libc::dup(directory.as_raw_fd()) })?;
    let stream = unsafe { libc::fdopendir(duplicate) };
    if stream.is_null() {
        unsafe { libc::close(duplicate) };
        return Err(io::Error::last_os_error());
    }
    let mut names = Vec::new();
    loop {
        clear_errno();
        let entry = unsafe { libc::readdir(stream) };
        if entry.is_null() {
            let error = io::Error::last_os_error();
            unsafe { libc::closedir(stream) };
            if error.raw_os_error() == Some(0) {
                break;
            }
            return Err(error);
        }
        let bytes = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
        if bytes == b"." || bytes == b".." {
            continue;
        }
        let name = match OsString::from_vec(bytes.to_vec()).into_string() {
            Ok(name) => name,
            Err(_) => {
                unsafe { libc::closedir(stream) };
                return Err(io::Error::other("artifact entry is not UTF-8"));
            }
        };
        names.push(name);
    }
    names.sort();
    Ok(names)
}

pub(super) fn open_dir_at(parent: RawFd, name: &CStr) -> io::Result<File> {
    let fd = cvt(unsafe {
        libc::openat(
            parent,
            name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    })?;
    Ok(unsafe { File::from_raw_fd(fd) })
}

pub(super) fn mkdir_at(parent: RawFd, name: &CStr) -> io::Result<()> {
    cvt(unsafe { libc::mkdirat(parent, name.as_ptr(), 0o700) }).map(|_| ())
}

pub(super) fn unlink_at(parent: RawFd, name: &CStr, directory: bool) -> io::Result<()> {
    let flags = if directory { libc::AT_REMOVEDIR } else { 0 };
    cvt(unsafe { libc::unlinkat(parent, name.as_ptr(), flags) }).map(|_| ())
}

pub(super) fn stat_at(parent: RawFd, name: &CStr) -> io::Result<libc::stat> {
    let mut metadata = std::mem::MaybeUninit::uninit();
    cvt(unsafe {
        libc::fstatat(
            parent,
            name.as_ptr(),
            metadata.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    })?;
    Ok(unsafe { metadata.assume_init() })
}

pub(super) fn require_directory(file: &File) -> io::Result<DirectoryIdentity> {
    let metadata = file.metadata()?;
    if !metadata.is_dir() {
        return Err(io::Error::other("expected an opened directory"));
    }
    Ok(DirectoryIdentity::new(metadata.dev(), metadata.ino()))
}

pub(super) fn require_regular(file: &File) -> io::Result<DirectoryIdentity> {
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(io::Error::other("expected an opened regular file"));
    }
    Ok(DirectoryIdentity::new(metadata.dev(), metadata.ino()))
}

pub(super) fn stat_identity_at(parent: RawFd, name: &CStr) -> io::Result<DirectoryIdentity> {
    let metadata = stat_at(parent, name)?;
    normalize_stat_identity(metadata.st_dev, metadata.st_ino)
}

pub(super) fn create_dir_at_verified(parent: &File, name: &CStr) -> io::Result<File> {
    mkdir_at(parent.as_raw_fd(), name)?;
    let created = stat_identity_at(parent.as_raw_fd(), name)?;
    let directory = open_dir_at(parent.as_raw_fd(), name)?;
    let opened = require_directory(&directory)?;
    if created != opened {
        return Err(io::Error::other(
            "created directory identity changed before open",
        ));
    }
    verify_at(parent.as_raw_fd(), name, opened)?;
    Ok(directory)
}

pub(super) struct ExclusiveLock<'a>(&'a File);

impl Drop for ExclusiveLock<'_> {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}

pub(super) fn lock_exclusive(directory: &File) -> io::Result<ExclusiveLock<'_>> {
    match directory.try_lock() {
        Ok(()) => Ok(ExclusiveLock(directory)),
        Err(TryLockError::WouldBlock) => Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            "managed directory is locked by another writer",
        )),
        Err(TryLockError::Error(source)) => Err(source),
    }
}

pub(super) fn verify_at(parent: RawFd, name: &CStr, expected: DirectoryIdentity) -> io::Result<()> {
    let actual = stat_identity_at(parent, name)?;
    if actual == expected {
        Ok(())
    } else {
        Err(io::Error::other("artifact path identity changed"))
    }
}

fn normalize_stat_identity<D, I>(device: D, inode: I) -> io::Result<DirectoryIdentity>
where
    D: TryInto<u64>,
    I: TryInto<u64>,
{
    let device = device.try_into().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "filesystem device identity is outside the normalized u64 range",
        )
    })?;
    let inode = inode.try_into().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "filesystem inode identity is outside the normalized u64 range",
        )
    })?;
    Ok(DirectoryIdentity::new(device, inode))
}

pub(super) fn cvt(result: libc::c_int) -> io::Result<libc::c_int> {
    if result == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(result)
    }
}

#[cfg(target_vendor = "apple")]
fn clear_errno() {
    unsafe { *libc::__error() = 0 };
}

#[cfg(not(target_vendor = "apple"))]
fn clear_errno() {
    unsafe { *libc::__errno_location() = 0 };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apple_errno_access_uses_the_platform_symbol() {
        let source = include_str!("unix_support.rs");
        assert!(source.contains("target_vendor = \"apple\""));
        assert!(source.contains("libc::__error()"));
        assert!(source.contains("libc::__errno_location()"));
    }

    #[test]
    fn stat_identity_normalizes_linux_unsigned_and_apple_signed_shapes() {
        let linux = normalize_stat_identity(7_u64, 11_u64).unwrap();
        let apple = normalize_stat_identity(7_i32, 11_u64).unwrap();
        assert_eq!(linux, DirectoryIdentity::new(7, 11));
        assert_eq!(apple, linux);
        assert!(normalize_stat_identity(-1_i32, 11_u64).is_err());
        assert!(normalize_stat_identity(7_i32, -1_i64).is_err());
    }
}
