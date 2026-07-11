use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

#[cfg(test)]
use std::collections::BTreeSet;

use crate::domain::{AbsolutePath, ValidationError};

use super::{FileSystemAction, PathRole, RuntimeError, path_value::absolute_path};

#[cfg(test)]
use super::{
    DirectorySyncState, PublicationResidual, PublicationResidualRole, PublicationResiduals,
};

mod locking;
mod publication;
mod unix_identity;

#[cfg(test)]
use locking::try_acquire_with;
#[cfg(test)]
use publication::Publication;
use publication::{SystemPublication, copy_recoverable_with};
use unix_identity::{descriptor_identity, open_read_no_follow_for, verify_path_identity};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileKind {
    Missing,
    RegularFile,
    Directory,
    Symlink,
    Other,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileMetadata {
    kind: FileKind,
    length: u64,
    link_target: Option<PathBuf>,
    link_target_exists: Option<bool>,
}

impl FileMetadata {
    pub const fn kind(&self) -> FileKind {
        self.kind
    }

    pub const fn length(&self) -> u64 {
        self.length
    }

    pub fn link_target(&self) -> Option<&Path> {
        self.link_target.as_deref()
    }

    pub const fn link_target_exists(&self) -> Option<bool> {
        self.link_target_exists
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelativeSymlinkTarget(String);

impl RelativeSymlinkTarget {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        if value.is_empty() {
            return Err(ValidationError::Empty {
                kind: "relative symlink target",
            });
        }
        if let Some((index, _)) = value
            .char_indices()
            .find(|(_, character)| character.is_control())
        {
            return Err(ValidationError::ControlCharacter {
                kind: "relative symlink target",
                index,
            });
        }
        if value.len() > 4096 {
            return Err(ValidationError::TooLong {
                kind: "relative symlink target",
                max: 4096,
                actual: value.len(),
            });
        }
        let path = Path::new(&value);
        if path.is_absolute() {
            return Err(ValidationError::PathNotRelative);
        }
        let mut saw_normal = false;
        let valid_components = path.components().all(|component| match component {
            std::path::Component::ParentDir if !saw_normal => true,
            std::path::Component::Normal(_) => {
                saw_normal = true;
                true
            }
            _ => false,
        });
        if !valid_components
            || !saw_normal
            || path.components().collect::<PathBuf>().to_str() != Some(value.as_str())
        {
            return Err(ValidationError::InvalidRelativePathComponent);
        }
        Ok(Self(value))
    }

    pub fn as_path(&self) -> &Path {
        Path::new(&self.0)
    }
}

pub trait FileSystem {
    fn inspect(&self, path: &AbsolutePath) -> Result<FileMetadata, RuntimeError>;
    fn canonicalize(&self, path: &AbsolutePath) -> Result<AbsolutePath, RuntimeError>;
    fn create_directory_all(&self, path: &AbsolutePath) -> Result<(), RuntimeError>;
    fn read(&self, path: &AbsolutePath) -> Result<Vec<u8>, RuntimeError>;
    fn read_regular_no_follow(&self, path: &AbsolutePath) -> Result<Option<Vec<u8>>, RuntimeError>;
    fn atomic_write(&self, path: &AbsolutePath, contents: &[u8]) -> Result<(), RuntimeError>;
    fn copy_recoverable(
        &self,
        source: &AbsolutePath,
        destination: &AbsolutePath,
    ) -> Result<(), RuntimeError>;
    fn create_relative_symlink(
        &self,
        target: &RelativeSymlinkTarget,
        link: &AbsolutePath,
    ) -> Result<(), RuntimeError>;
    fn remove(&self, path: &AbsolutePath) -> Result<(), RuntimeError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SystemFileSystem;

impl FileSystem for SystemFileSystem {
    fn inspect(&self, path: &AbsolutePath) -> Result<FileMetadata, RuntimeError> {
        inspect(path)
    }

    fn canonicalize(&self, path: &AbsolutePath) -> Result<AbsolutePath, RuntimeError> {
        let canonical = fs::canonicalize(path.as_str())
            .map_err(|source| filesystem_error(FileSystemAction::Canonicalize, path, source))?;
        absolute_path(&canonical, PathRole::CanonicalPath)
    }

    fn create_directory_all(&self, path: &AbsolutePath) -> Result<(), RuntimeError> {
        if inspect(path)?.kind == FileKind::Symlink {
            return Err(unsafe_symlink(FileSystemAction::CreateDirectory, path));
        }
        fs::create_dir_all(path.as_str())
            .map_err(|source| filesystem_error(FileSystemAction::CreateDirectory, path, source))
    }

    fn read(&self, path: &AbsolutePath) -> Result<Vec<u8>, RuntimeError> {
        fs::read(path.as_str())
            .map_err(|source| filesystem_error(FileSystemAction::Read, path, source))
    }

    fn read_regular_no_follow(&self, path: &AbsolutePath) -> Result<Option<Vec<u8>>, RuntimeError> {
        read_regular_no_follow_with(path, || {})
    }

    fn atomic_write(&self, path: &AbsolutePath, contents: &[u8]) -> Result<(), RuntimeError> {
        if inspect(path)?.kind == FileKind::Symlink {
            return Err(unsafe_symlink(FileSystemAction::Write, path));
        }
        atomic_write_with(path, |file| file.write_all(contents))
    }

    fn copy_recoverable(
        &self,
        source: &AbsolutePath,
        destination: &AbsolutePath,
    ) -> Result<(), RuntimeError> {
        copy_recoverable_with(source, destination, &SystemPublication, || {})
    }

    fn create_relative_symlink(
        &self,
        target: &RelativeSymlinkTarget,
        link: &AbsolutePath,
    ) -> Result<(), RuntimeError> {
        if inspect(link)?.kind != FileKind::Missing {
            return Err(filesystem_error(
                FileSystemAction::Write,
                link,
                io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "link destination already exists",
                ),
            ));
        }
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(target.as_path(), link.as_str())
                .map_err(|source| filesystem_error(FileSystemAction::Write, link, source))?;
            sync_parent(link, FileSystemAction::Write)
        }
        #[cfg(not(unix))]
        {
            let _ = target;
            Err(RuntimeError::UnsupportedPlatform {
                platform: std::env::consts::OS.to_owned(),
            })
        }
    }

    fn remove(&self, path: &AbsolutePath) -> Result<(), RuntimeError> {
        let removed = match inspect(path)?.kind {
            FileKind::Missing => return Ok(()),
            FileKind::Directory => fs::remove_dir_all(path.as_str())
                .map_err(|source| filesystem_error(FileSystemAction::Remove, path, source)),
            FileKind::RegularFile | FileKind::Symlink | FileKind::Other => {
                fs::remove_file(path.as_str())
                    .map_err(|source| filesystem_error(FileSystemAction::Remove, path, source))
            }
        };
        removed?;
        sync_parent(path, FileSystemAction::Remove)
    }
}

fn read_regular_no_follow_with(
    path: &AbsolutePath,
    after_open: impl FnOnce(),
) -> Result<Option<Vec<u8>>, RuntimeError> {
    let Some(mut file) = open_read_no_follow_for(path, FileSystemAction::Read)? else {
        return Ok(None);
    };
    let identity = descriptor_identity(&file, FileSystemAction::Read, path)?;
    let metadata = file
        .metadata()
        .map_err(|source| filesystem_error(FileSystemAction::Read, path, source))?;
    if !metadata.is_file() {
        return Err(filesystem_error(
            FileSystemAction::Read,
            path,
            io::Error::new(io::ErrorKind::InvalidInput, "expected a regular file"),
        ));
    }
    after_open();
    verify_path_identity(path, identity, FileSystemAction::Read)?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)
        .map_err(|source| filesystem_error(FileSystemAction::Read, path, source))?;
    Ok(Some(contents))
}

pub trait ConfigurationLock {
    type Guard: ConfigurationLockGuard;

    fn try_acquire(&self, path: &AbsolutePath) -> Result<Self::Guard, RuntimeError>;
}

pub trait ConfigurationLockGuard {
    fn path(&self) -> &AbsolutePath;
    fn release(self) -> Result<(), RuntimeError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SystemConfigurationLock;

#[derive(Debug)]
pub struct SystemConfigurationLockGuard {
    file: Option<File>,
    directory: Option<File>,
    path: AbsolutePath,
}

fn inspect(path: &AbsolutePath) -> Result<FileMetadata, RuntimeError> {
    let metadata = match fs::symlink_metadata(path.as_str()) {
        Ok(metadata) => metadata,
        Err(source) if source.kind() == io::ErrorKind::NotFound => {
            return Ok(FileMetadata {
                kind: FileKind::Missing,
                length: 0,
                link_target: None,
                link_target_exists: None,
            });
        }
        Err(source) => {
            return Err(filesystem_error(FileSystemAction::Inspect, path, source));
        }
    };
    let file_type = metadata.file_type();
    if file_type.is_symlink() {
        let target = fs::read_link(path.as_str())
            .map_err(|source| filesystem_error(FileSystemAction::ReadLink, path, source))?;
        let target_exists = match fs::metadata(path.as_str()) {
            Ok(_) => true,
            Err(source) if source.kind() == io::ErrorKind::NotFound => false,
            Err(source) => {
                return Err(filesystem_error(FileSystemAction::Inspect, path, source));
            }
        };
        return Ok(FileMetadata {
            kind: FileKind::Symlink,
            length: metadata.len(),
            link_target: Some(target),
            link_target_exists: Some(target_exists),
        });
    }
    let kind = if file_type.is_file() {
        FileKind::RegularFile
    } else if file_type.is_dir() {
        FileKind::Directory
    } else {
        FileKind::Other
    };
    Ok(FileMetadata {
        kind,
        length: metadata.len(),
        link_target: None,
        link_target_exists: None,
    })
}

fn atomic_write_with(
    path: &AbsolutePath,
    write: impl FnOnce(&mut File) -> io::Result<()>,
) -> Result<(), RuntimeError> {
    let destination = Path::new(path.as_str());
    let parent = destination.parent().ok_or_else(|| {
        filesystem_error(
            FileSystemAction::Write,
            path,
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "destination has no parent directory",
            ),
        )
    })?;
    let name = destination.file_name().ok_or_else(|| {
        filesystem_error(
            FileSystemAction::Write,
            path,
            io::Error::new(io::ErrorKind::InvalidInput, "destination has no file name"),
        )
    })?;
    let (temporary_path, mut temporary_file) = create_temporary(parent, name)
        .map_err(|source| filesystem_error(FileSystemAction::Write, path, source))?;

    let result = write(&mut temporary_file)
        .and_then(|()| temporary_file.sync_all())
        .and_then(|()| {
            drop(temporary_file);
            fs::rename(&temporary_path, destination)
        });
    if let Err(source) = result {
        let _ = fs::remove_file(&temporary_path);
        return Err(filesystem_error(FileSystemAction::Write, path, source));
    }
    sync_parent(path, FileSystemAction::Write)
}

fn create_temporary(parent: &Path, name: &std::ffi::OsStr) -> io::Result<(PathBuf, File)> {
    for _ in 0..32 {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = parent.join(format!(
            ".{}.skilltap-tmp-{}-{sequence}",
            name.to_string_lossy(),
            std::process::id()
        ));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {}
            Err(source) => return Err(source),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate atomic-write temporary file",
    ))
}

fn sync_parent(path: &AbsolutePath, _action: FileSystemAction) -> Result<(), RuntimeError> {
    sync_parent_io(Path::new(path.as_str()))
        .map_err(|source| filesystem_error(FileSystemAction::Sync, path, source))
}

fn sync_parent_io(path: &Path) -> io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "path has no parent directory")
    })?;
    File::open(parent).and_then(|directory| directory.sync_all())
}

fn filesystem_error(
    action: FileSystemAction,
    path: &AbsolutePath,
    source: io::Error,
) -> RuntimeError {
    RuntimeError::FileSystem {
        action,
        path: path.clone(),
        source,
    }
}

fn unsafe_symlink(action: FileSystemAction, path: &AbsolutePath) -> RuntimeError {
    RuntimeError::UnsafeSymlink {
        action,
        path: path.clone(),
    }
}

#[cfg(test)]
mod tests;
