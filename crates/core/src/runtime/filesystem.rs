use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};

use crate::domain::{AbsolutePath, ValidationError};

use super::{
    DirectorySyncState, FileSystemAction, LockAction, PathRole, PublicationResidual,
    PublicationResidualRole, PublicationResiduals, RuntimeError,
};

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
        let value =
            canonical
                .into_os_string()
                .into_string()
                .map_err(|_| RuntimeError::NonUtf8Path {
                    role: PathRole::CanonicalPath,
                })?;
        AbsolutePath::new(value).map_err(|source| RuntimeError::InvalidPath {
            role: PathRole::CanonicalPath,
            source,
        })
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FileIdentity {
    device: u64,
    inode: u64,
}

trait Publication {
    fn publish_no_clobber(&self, temporary: &Path, destination: &Path) -> io::Result<()>;
    fn remove(&self, path: &Path) -> io::Result<()>;
    fn sync_parent(&self, destination: &Path) -> io::Result<()>;
}

struct SystemPublication;

impl Publication for SystemPublication {
    fn publish_no_clobber(&self, temporary: &Path, destination: &Path) -> io::Result<()> {
        fs::hard_link(temporary, destination)
    }

    fn remove(&self, path: &Path) -> io::Result<()> {
        fs::remove_file(path)
    }

    fn sync_parent(&self, destination: &Path) -> io::Result<()> {
        sync_parent_io(destination)
    }
}

fn copy_recoverable_with(
    source: &AbsolutePath,
    destination: &AbsolutePath,
    publication: &impl Publication,
    after_source_open: impl FnOnce(),
) -> Result<(), RuntimeError> {
    let mut source_file = open_read_no_follow(source)?;
    let source_identity = descriptor_identity(&source_file, FileSystemAction::Copy, source)?;
    after_source_open();
    verify_path_identity(source, source_identity, FileSystemAction::Copy)?;
    if !source_file
        .metadata()
        .map_err(|error| filesystem_error(FileSystemAction::Copy, source, error))?
        .is_file()
    {
        return Err(filesystem_error(
            FileSystemAction::Copy,
            source,
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "backup source is not a regular file",
            ),
        ));
    }

    let destination_path = Path::new(destination.as_str());
    let parent = destination_path.parent().ok_or_else(|| {
        filesystem_error(
            FileSystemAction::Copy,
            destination,
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "destination has no parent directory",
            ),
        )
    })?;
    let name = destination_path.file_name().ok_or_else(|| {
        filesystem_error(
            FileSystemAction::Copy,
            destination,
            io::Error::new(io::ErrorKind::InvalidInput, "destination has no file name"),
        )
    })?;
    let (temporary_path, mut temporary_file) = create_temporary(parent, name)
        .map_err(|error| filesystem_error(FileSystemAction::Copy, destination, error))?;
    let temporary_identity = match descriptor_identity_io(&temporary_file) {
        Ok(identity) => identity,
        Err(source) => {
            drop(temporary_file);
            return Err(RuntimeError::PartialPublication {
                path: destination.clone(),
                residuals: PublicationResiduals::new(
                    [publication_residual(
                        PublicationResidualRole::Temporary,
                        &temporary_path,
                    )],
                    DirectorySyncState::NotRequired,
                ),
                source,
                cleanup: io::Error::other(
                    "temporary identity unavailable; cleanup could not be proven safe",
                ),
            });
        }
    };

    let staged =
        io::copy(&mut source_file, &mut temporary_file).and_then(|_| temporary_file.sync_all());
    drop(temporary_file);
    if let Err(error) = staged {
        return Err(clean_prepublication_failure(
            destination,
            &temporary_path,
            temporary_identity,
            error,
            publication,
        ));
    }

    if let Err(error) = require_path_identity(&temporary_path, temporary_identity) {
        return Err(clean_prepublication_failure(
            destination,
            &temporary_path,
            temporary_identity,
            error,
            publication,
        ));
    }

    if let Err(error) = publication.publish_no_clobber(&temporary_path, destination_path) {
        return Err(clean_prepublication_failure(
            destination,
            &temporary_path,
            temporary_identity,
            error,
            publication,
        ));
    }

    if let Err(error) = require_path_identity(destination_path, temporary_identity) {
        return Err(rollback_publication(
            destination,
            temporary_identity,
            Some((&temporary_path, temporary_identity)),
            error,
            publication,
        ));
    }

    if let Err(error) = remove_if_identity(&temporary_path, temporary_identity, publication) {
        return Err(rollback_publication(
            destination,
            temporary_identity,
            Some((&temporary_path, temporary_identity)),
            error,
            publication,
        ));
    }

    if let Err(error) = publication.sync_parent(destination_path) {
        return Err(rollback_publication(
            destination,
            temporary_identity,
            None,
            error,
            publication,
        ));
    }
    Ok(())
}

fn clean_prepublication_failure(
    destination: &AbsolutePath,
    temporary: &Path,
    identity: FileIdentity,
    source: io::Error,
    publication: &impl Publication,
) -> RuntimeError {
    match remove_if_identity(temporary, identity, publication) {
        Ok(()) => filesystem_error(FileSystemAction::Copy, destination, source),
        Err(cleanup) => RuntimeError::PartialPublication {
            path: destination.clone(),
            residuals: PublicationResiduals::new(
                [publication_residual(
                    PublicationResidualRole::Temporary,
                    temporary,
                )],
                DirectorySyncState::NotRequired,
            ),
            source,
            cleanup,
        },
    }
}

fn rollback_publication(
    destination: &AbsolutePath,
    identity: FileIdentity,
    temporary: Option<(&Path, FileIdentity)>,
    source: io::Error,
    publication: &impl Publication,
) -> RuntimeError {
    let destination_path = Path::new(destination.as_str());
    let mut failures = Vec::new();
    let mut residual_paths = BTreeSet::new();
    if let Err(error) = remove_if_identity(destination_path, identity, publication) {
        residual_paths.insert(PublicationResidual::new(
            PublicationResidualRole::Destination,
            destination.clone(),
        ));
        failures.push(format!("destination rollback: {error}"));
    }
    if let Some((temporary_path, temporary_identity)) = temporary
        && let Err(error) = remove_if_identity(temporary_path, temporary_identity, publication)
    {
        residual_paths.insert(publication_residual(
            PublicationResidualRole::Temporary,
            temporary_path,
        ));
        failures.push(format!("temporary cleanup: {error}"));
    }
    let directory_sync = match publication.sync_parent(destination_path) {
        Ok(()) => DirectorySyncState::Synced,
        Err(error) => {
            failures.push(format!("rollback directory sync: {error}"));
            DirectorySyncState::Uncertain
        }
    };
    if residual_paths.is_empty() && directory_sync == DirectorySyncState::Synced {
        filesystem_error(FileSystemAction::Copy, destination, source)
    } else {
        RuntimeError::PartialPublication {
            path: destination.clone(),
            residuals: PublicationResiduals::new(residual_paths, directory_sync),
            source,
            cleanup: io::Error::other(failures.join("; ")),
        }
    }
}

fn publication_residual(role: PublicationResidualRole, path: &Path) -> PublicationResidual {
    let value = path
        .to_str()
        .expect("owned publication paths originate from validated UTF-8 paths");
    PublicationResidual::new(
        role,
        AbsolutePath::new(value)
            .expect("owned publication paths remain lexically normalized and absolute"),
    )
}

fn remove_if_identity(
    path: &Path,
    expected: FileIdentity,
    publication: &impl Publication,
) -> io::Result<()> {
    match path_identity(path) {
        Ok(actual) if actual == expected => publication.remove(path),
        Ok(_) => Err(io::Error::other("path identity changed before cleanup")),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn require_path_identity(path: &Path, expected: FileIdentity) -> io::Result<()> {
    match path_identity(path) {
        Ok(actual) if actual == expected => Ok(()),
        Ok(_) => Err(io::Error::other("path identity changed")),
        Err(error) => Err(error),
    }
}

#[cfg(unix)]
fn open_read_no_follow(path: &AbsolutePath) -> Result<File, RuntimeError> {
    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path.as_str())
        .map_err(|error| {
            if error.raw_os_error() == Some(libc::ELOOP) {
                unsafe_symlink(FileSystemAction::Copy, path)
            } else {
                filesystem_error(FileSystemAction::Copy, path, error)
            }
        })
}

#[cfg(not(unix))]
fn open_read_no_follow(_path: &AbsolutePath) -> Result<File, RuntimeError> {
    Err(RuntimeError::UnsupportedPlatform {
        platform: std::env::consts::OS.to_owned(),
    })
}

#[cfg(unix)]
fn descriptor_identity(
    file: &File,
    action: FileSystemAction,
    path: &AbsolutePath,
) -> Result<FileIdentity, RuntimeError> {
    descriptor_identity_io(file).map_err(|error| filesystem_error(action, path, error))
}

#[cfg(unix)]
fn descriptor_identity_io(file: &File) -> io::Result<FileIdentity> {
    let metadata = file.metadata()?;
    Ok(FileIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    })
}

#[cfg(not(unix))]
fn descriptor_identity(
    _file: &File,
    _action: FileSystemAction,
    _path: &AbsolutePath,
) -> Result<FileIdentity, RuntimeError> {
    Err(RuntimeError::UnsupportedPlatform {
        platform: std::env::consts::OS.to_owned(),
    })
}

#[cfg(not(unix))]
fn descriptor_identity_io(_file: &File) -> io::Result<FileIdentity> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "file identity requires a supported Unix platform",
    ))
}

#[cfg(unix)]
fn path_identity(path: &Path) -> io::Result<FileIdentity> {
    let metadata = fs::symlink_metadata(path)?;
    Ok(FileIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    })
}

#[cfg(not(unix))]
fn path_identity(_path: &Path) -> io::Result<FileIdentity> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "file identity requires a supported Unix platform",
    ))
}

fn verify_path_identity(
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

impl ConfigurationLock for SystemConfigurationLock {
    type Guard = SystemConfigurationLockGuard;

    fn try_acquire(&self, path: &AbsolutePath) -> Result<Self::Guard, RuntimeError> {
        try_acquire_with(path, || {})
    }
}

impl ConfigurationLockGuard for SystemConfigurationLockGuard {
    fn path(&self) -> &AbsolutePath {
        &self.path
    }

    fn release(mut self) -> Result<(), RuntimeError> {
        let file = self.file.take().expect("lock guard owns its file");
        let directory = self
            .directory
            .take()
            .expect("lock guard owns its directory");
        let file_result = file.unlock();
        let directory_result = directory.unlock();
        match (file_result, directory_result) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(source), Ok(())) | (Ok(()), Err(source)) => Err(RuntimeError::Lock {
                action: LockAction::Release,
                path: self.path.clone(),
                source,
            }),
            (Err(file), Err(directory)) => Err(RuntimeError::Lock {
                action: LockAction::Release,
                path: self.path.clone(),
                source: io::Error::other(format!(
                    "file unlock failed ({file}); directory unlock failed ({directory})"
                )),
            }),
        }
    }
}

impl Drop for SystemConfigurationLockGuard {
    fn drop(&mut self) {
        if let Some(file) = self.file.take() {
            let _ = file.unlock();
        }
        if let Some(directory) = self.directory.take() {
            let _ = directory.unlock();
        }
    }
}

fn try_acquire_with(
    path: &AbsolutePath,
    after_file_lock: impl FnOnce(),
) -> Result<SystemConfigurationLockGuard, RuntimeError> {
    let parent = Path::new(path.as_str())
        .parent()
        .ok_or_else(|| RuntimeError::Lock {
            action: LockAction::Acquire,
            path: path.clone(),
            source: io::Error::new(
                io::ErrorKind::InvalidInput,
                "lock path has no parent directory",
            ),
        })?;
    let parent_path = AbsolutePath::new(
        parent
            .to_str()
            .ok_or(RuntimeError::NonUtf8Path {
                role: PathRole::SkilltapConfig,
            })?
            .to_owned(),
    )
    .map_err(|source| RuntimeError::InvalidPath {
        role: PathRole::SkilltapConfig,
        source,
    })?;

    let directory = open_directory_no_follow(&parent_path)?;
    let directory_identity =
        descriptor_identity_io(&directory).map_err(|source| RuntimeError::Lock {
            action: LockAction::Acquire,
            path: path.clone(),
            source,
        })?;
    try_lock_file(&directory, path)?;
    verify_lock_identity(&parent_path, directory_identity, path)?;

    let file = open_lock_no_follow(path)?;
    let identity = descriptor_identity_io(&file).map_err(|source| RuntimeError::Lock {
        action: LockAction::Acquire,
        path: path.clone(),
        source,
    })?;
    try_lock_file(&file, path)?;
    after_file_lock();
    verify_lock_identity(path, identity, path)?;

    Ok(SystemConfigurationLockGuard {
        file: Some(file),
        directory: Some(directory),
        path: path.clone(),
    })
}

fn try_lock_file(file: &File, path: &AbsolutePath) -> Result<(), RuntimeError> {
    match file.try_lock() {
        Ok(()) => Ok(()),
        Err(fs::TryLockError::WouldBlock) => {
            Err(RuntimeError::LockContended { path: path.clone() })
        }
        Err(fs::TryLockError::Error(source)) => Err(RuntimeError::Lock {
            action: LockAction::Acquire,
            path: path.clone(),
            source,
        }),
    }
}

#[cfg(unix)]
fn open_directory_no_follow(path: &AbsolutePath) -> Result<File, RuntimeError> {
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
fn open_directory_no_follow(_path: &AbsolutePath) -> Result<File, RuntimeError> {
    Err(RuntimeError::UnsupportedPlatform {
        platform: std::env::consts::OS.to_owned(),
    })
}

#[cfg(unix)]
fn open_lock_no_follow(path: &AbsolutePath) -> Result<File, RuntimeError> {
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
fn open_lock_no_follow(_path: &AbsolutePath) -> Result<File, RuntimeError> {
    Err(RuntimeError::UnsupportedPlatform {
        platform: std::env::consts::OS.to_owned(),
    })
}

fn verify_lock_identity(
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
mod tests {
    use std::{
        cell::Cell,
        sync::{Arc, atomic::AtomicBool},
        thread,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    struct InjectedPublication {
        fail_publish: bool,
        fail_remove_calls: BTreeSet<usize>,
        fail_sync_calls: BTreeSet<usize>,
        remove_calls: Cell<usize>,
        sync_calls: Cell<usize>,
    }

    impl InjectedPublication {
        fn new() -> Self {
            Self {
                fail_publish: false,
                fail_remove_calls: BTreeSet::new(),
                fail_sync_calls: BTreeSet::new(),
                remove_calls: Cell::new(0),
                sync_calls: Cell::new(0),
            }
        }
    }

    impl Publication for InjectedPublication {
        fn publish_no_clobber(&self, temporary: &Path, destination: &Path) -> io::Result<()> {
            if self.fail_publish {
                Err(io::Error::other("injected publication failure"))
            } else {
                fs::hard_link(temporary, destination)
            }
        }

        fn remove(&self, path: &Path) -> io::Result<()> {
            let call = self.remove_calls.get() + 1;
            self.remove_calls.set(call);
            if self.fail_remove_calls.contains(&call) {
                Err(io::Error::other("injected removal failure"))
            } else {
                fs::remove_file(path)
            }
        }

        fn sync_parent(&self, destination: &Path) -> io::Result<()> {
            let call = self.sync_calls.get() + 1;
            self.sync_calls.set(call);
            if self.fail_sync_calls.contains(&call) {
                Err(io::Error::other("injected directory sync failure"))
            } else {
                sync_parent_io(destination)
            }
        }
    }

    struct TempDirectory(PathBuf);

    impl TempDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "skilltap-filesystem-test-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn path(&self, child: &str) -> AbsolutePath {
            AbsolutePath::new(self.0.join(child).to_str().unwrap()).unwrap()
        }
    }

    impl Drop for TempDirectory {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn inspection_distinguishes_files_directories_and_live_or_dangling_links() {
        let temporary = TempDirectory::new();
        let filesystem = SystemFileSystem;
        let directory = temporary.path("directory");
        let file = temporary.path("file");
        let live_link = temporary.path("live-link");
        let dangling_link = temporary.path("dangling-link");
        filesystem.create_directory_all(&directory).unwrap();
        filesystem.atomic_write(&file, b"content").unwrap();
        filesystem
            .create_relative_symlink(&RelativeSymlinkTarget::new("file").unwrap(), &live_link)
            .unwrap();
        filesystem
            .create_relative_symlink(
                &RelativeSymlinkTarget::new("missing").unwrap(),
                &dangling_link,
            )
            .unwrap();

        assert_eq!(
            filesystem.inspect(&file).unwrap().kind(),
            FileKind::RegularFile
        );
        assert_eq!(
            filesystem.inspect(&directory).unwrap().kind(),
            FileKind::Directory
        );
        assert_eq!(
            filesystem
                .inspect(&temporary.path("missing"))
                .unwrap()
                .kind(),
            FileKind::Missing
        );
        let live = filesystem.inspect(&live_link).unwrap();
        assert_eq!(live.kind(), FileKind::Symlink);
        assert_eq!(live.link_target(), Some(Path::new("file")));
        assert_eq!(live.link_target_exists(), Some(true));
        assert_eq!(
            filesystem
                .inspect(&dangling_link)
                .unwrap()
                .link_target_exists(),
            Some(false)
        );
    }

    #[test]
    fn atomic_write_replaces_whole_contents_and_cleans_failed_temporaries() {
        let temporary = TempDirectory::new();
        let path = temporary.path("state.json");
        let filesystem = SystemFileSystem;
        filesystem.atomic_write(&path, b"old").unwrap();
        filesystem.atomic_write(&path, b"new-complete").unwrap();
        assert_eq!(filesystem.read(&path).unwrap(), b"new-complete");

        let error = atomic_write_with(&path, |file| {
            file.write_all(b"partial")?;
            Err(io::Error::other("injected failure"))
        })
        .unwrap_err();
        assert_eq!(error.boundary(), super::super::RuntimeBoundary::FileSystem);
        assert_eq!(filesystem.read(&path).unwrap(), b"new-complete");
        assert_eq!(fs::read_dir(&temporary.0).unwrap().count(), 1);
    }

    #[test]
    fn concurrent_readers_observe_only_old_or_new_complete_files() {
        let temporary = TempDirectory::new();
        let path = temporary.path("inventory.toml");
        let filesystem = SystemFileSystem;
        let old = vec![b'a'; 256 * 1024];
        let new = vec![b'b'; 256 * 1024];
        filesystem.atomic_write(&path, &old).unwrap();
        let path_for_reader = path.clone();
        let old_for_reader = old.clone();
        let new_for_reader = new.clone();
        let running = Arc::new(AtomicBool::new(true));
        let reader_running = Arc::clone(&running);
        let reader = thread::spawn(move || {
            while reader_running.load(Ordering::Relaxed) {
                let observed = fs::read(path_for_reader.as_str()).unwrap();
                assert!(observed == old_for_reader || observed == new_for_reader);
            }
        });
        filesystem.atomic_write(&path, &new).unwrap();
        running.store(false, Ordering::Relaxed);
        reader.join().unwrap();
    }

    #[test]
    fn recoverable_copy_never_overwrites_and_rejects_symlink_sources() {
        let temporary = TempDirectory::new();
        let filesystem = SystemFileSystem;
        let source = temporary.path("AGENTS.md");
        let backup = temporary.path("AGENTS.md.backup");
        filesystem.atomic_write(&source, b"instructions").unwrap();
        filesystem.copy_recoverable(&source, &backup).unwrap();
        assert_eq!(filesystem.read(&backup).unwrap(), b"instructions");
        assert!(filesystem.copy_recoverable(&source, &backup).is_err());
        assert_eq!(filesystem.read(&backup).unwrap(), b"instructions");

        let link = temporary.path("link");
        filesystem
            .create_relative_symlink(&RelativeSymlinkTarget::new("AGENTS.md").unwrap(), &link)
            .unwrap();
        assert!(matches!(
            filesystem.copy_recoverable(&link, &temporary.path("link.backup")),
            Err(RuntimeError::UnsafeSymlink { .. })
        ));
    }

    #[test]
    fn recoverable_copy_is_atomic_no_clobber_for_concurrent_readers() {
        let temporary = TempDirectory::new();
        let filesystem = SystemFileSystem;
        let source = temporary.path("source");
        let destination = temporary.path("backup");
        let complete = vec![b'x'; 512 * 1024];
        filesystem.atomic_write(&source, &complete).unwrap();

        let destination_for_reader = destination.clone();
        let complete_for_reader = complete.clone();
        let running = Arc::new(AtomicBool::new(true));
        let reader_running = Arc::clone(&running);
        let reader = thread::spawn(move || {
            while reader_running.load(Ordering::Relaxed) {
                match fs::read(destination_for_reader.as_str()) {
                    Ok(observed) => assert_eq!(observed, complete_for_reader),
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                    Err(error) => panic!("unexpected backup read error: {error}"),
                }
            }
        });
        filesystem.copy_recoverable(&source, &destination).unwrap();
        running.store(false, Ordering::Relaxed);
        reader.join().unwrap();
        assert_eq!(filesystem.read(&destination).unwrap(), complete);
    }

    #[test]
    fn backup_failures_report_exact_residual_paths_and_independent_sync_state() {
        let temporary = TempDirectory::new();
        let filesystem = SystemFileSystem;
        let source = temporary.path("source");
        filesystem.atomic_write(&source, b"complete").unwrap();

        let cleaned_destination = temporary.path("cleaned-backup");
        let mut cleaned = InjectedPublication::new();
        cleaned.fail_sync_calls.insert(1);
        let cleaned_error =
            copy_recoverable_with(&source, &cleaned_destination, &cleaned, || {}).unwrap_err();
        assert!(matches!(cleaned_error, RuntimeError::FileSystem { .. }));
        assert_eq!(
            filesystem.inspect(&cleaned_destination).unwrap().kind(),
            FileKind::Missing
        );

        let prepublication_destination = temporary.path("prepublication-backup");
        let mut prepublication = InjectedPublication::new();
        prepublication.fail_publish = true;
        prepublication.fail_remove_calls.insert(1);
        let prepublication_error =
            copy_recoverable_with(&source, &prepublication_destination, &prepublication, || {})
                .unwrap_err();
        let prepublication_residuals = partial_residuals(&prepublication_error);
        assert_eq!(
            residual_roles(prepublication_residuals),
            BTreeSet::from([PublicationResidualRole::Temporary])
        );
        assert_eq!(
            prepublication_residuals.directory_sync(),
            DirectorySyncState::NotRequired
        );
        assert_residual_paths_exist(prepublication_residuals);

        let destination_only_path = temporary.path("destination-only-backup");
        let mut destination_only = InjectedPublication::new();
        destination_only.fail_sync_calls.insert(1);
        destination_only.fail_remove_calls.insert(2);
        let destination_only_error =
            copy_recoverable_with(&source, &destination_only_path, &destination_only, || {})
                .unwrap_err();
        let destination_only_residuals = partial_residuals(&destination_only_error);
        assert_eq!(
            destination_only_residuals.paths(),
            &BTreeSet::from([PublicationResidual::new(
                PublicationResidualRole::Destination,
                destination_only_path.clone(),
            )])
        );
        assert_eq!(
            destination_only_residuals.directory_sync(),
            DirectorySyncState::Synced
        );
        assert_residual_paths_exist(destination_only_residuals);

        let temp_only_path = temporary.path("temp-only-backup");
        let mut temp_only = InjectedPublication::new();
        temp_only.fail_remove_calls.extend([1, 3]);
        let temp_only_error =
            copy_recoverable_with(&source, &temp_only_path, &temp_only, || {}).unwrap_err();
        let temp_only_residuals = partial_residuals(&temp_only_error);
        assert_eq!(
            residual_roles(temp_only_residuals),
            BTreeSet::from([PublicationResidualRole::Temporary])
        );
        assert_eq!(
            temp_only_residuals.directory_sync(),
            DirectorySyncState::Synced
        );
        assert_residual_paths_exist(temp_only_residuals);

        let both_path = temporary.path("both-backup");
        let mut both = InjectedPublication::new();
        both.fail_remove_calls.extend([1, 2, 3]);
        let both_error = copy_recoverable_with(&source, &both_path, &both, || {}).unwrap_err();
        let both_residuals = partial_residuals(&both_error);
        assert_eq!(
            residual_roles(both_residuals),
            BTreeSet::from([
                PublicationResidualRole::Temporary,
                PublicationResidualRole::Destination,
            ])
        );
        assert_eq!(both_residuals.directory_sync(), DirectorySyncState::Synced);
        assert_residual_paths_exist(both_residuals);

        let uncertain_path = temporary.path("uncertain-sync-backup");
        let mut uncertain = InjectedPublication::new();
        uncertain.fail_sync_calls.extend([1, 2]);
        let uncertain_error =
            copy_recoverable_with(&source, &uncertain_path, &uncertain, || {}).unwrap_err();
        let uncertain_residuals = partial_residuals(&uncertain_error);
        assert!(uncertain_residuals.paths().is_empty());
        assert_eq!(
            uncertain_residuals.directory_sync(),
            DirectorySyncState::Uncertain
        );

        let rendered = both_error.to_string();
        assert!(rendered.contains("temporary `"));
        assert!(rendered.contains("destination `"));
        assert!(rendered.contains("directory sync: synced"));
        assert!(!rendered.contains("complete"));
    }

    fn partial_residuals(error: &RuntimeError) -> &PublicationResiduals {
        match error {
            RuntimeError::PartialPublication { residuals, .. } => residuals,
            error => panic!("expected partial publication error, got {error}"),
        }
    }

    fn residual_roles(residuals: &PublicationResiduals) -> BTreeSet<PublicationResidualRole> {
        residuals
            .paths()
            .iter()
            .map(PublicationResidual::role)
            .collect()
    }

    fn assert_residual_paths_exist(residuals: &PublicationResiduals) {
        for residual in residuals.paths() {
            let metadata = fs::symlink_metadata(residual.path().as_str()).unwrap();
            assert!(
                !metadata.file_type().is_symlink(),
                "owned residual must identify the generated inode without following a link"
            );
        }
    }

    #[test]
    fn backup_source_swap_is_rejected_after_no_follow_open() {
        let temporary = TempDirectory::new();
        let filesystem = SystemFileSystem;
        let source = temporary.path("source");
        let opened = temporary.path("opened-source");
        let attacker = temporary.path("attacker");
        let destination = temporary.path("backup");
        filesystem.atomic_write(&source, b"original").unwrap();
        filesystem.atomic_write(&attacker, b"attacker").unwrap();

        let error = copy_recoverable_with(&source, &destination, &SystemPublication, || {
            fs::rename(source.as_str(), opened.as_str()).unwrap();
            std::os::unix::fs::symlink("attacker", source.as_str()).unwrap();
        })
        .unwrap_err();
        assert!(matches!(error, RuntimeError::FileIdentityChanged { .. }));
        assert_eq!(
            filesystem.inspect(&destination).unwrap().kind(),
            FileKind::Missing
        );
    }

    #[test]
    fn ownership_sensitive_operations_do_not_follow_symlinks() {
        let temporary = TempDirectory::new();
        let filesystem = SystemFileSystem;
        let target = temporary.path("user-content");
        let link = temporary.path("managed");
        filesystem.atomic_write(&target, b"preserve").unwrap();
        filesystem
            .create_relative_symlink(&RelativeSymlinkTarget::new("user-content").unwrap(), &link)
            .unwrap();

        assert!(matches!(
            filesystem.atomic_write(&link, b"replace"),
            Err(RuntimeError::UnsafeSymlink { .. })
        ));
        filesystem.remove(&link).unwrap();
        assert_eq!(filesystem.read(&target).unwrap(), b"preserve");
        assert_eq!(filesystem.inspect(&link).unwrap().kind(), FileKind::Missing);
    }

    #[test]
    fn relative_targets_allow_parent_bridges_but_reject_absolute_or_unnormalized_paths() {
        assert_eq!(
            RelativeSymlinkTarget::new("../AGENTS.md")
                .unwrap()
                .as_path(),
            Path::new("../AGENTS.md")
        );
        assert_eq!(
            RelativeSymlinkTarget::new("../../shared/AGENTS.md")
                .unwrap()
                .as_path(),
            Path::new("../../shared/AGENTS.md")
        );
        for invalid in [
            "",
            "/tmp/AGENTS.md",
            "./AGENTS.md",
            "dir//AGENTS.md",
            "dir/../AGENTS.md",
            "../dir/../AGENTS.md",
            "..",
            "AGENTS.md\n",
        ] {
            assert!(RelativeSymlinkTarget::new(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn configuration_lock_fails_fast_and_releases_explicitly_or_on_drop() {
        let temporary = TempDirectory::new();
        let path = temporary.path("skilltap.lock");
        let lock = SystemConfigurationLock;
        let first = lock.try_acquire(&path).unwrap();
        assert_eq!(first.path(), &path);
        assert!(matches!(
            lock.try_acquire(&path),
            Err(RuntimeError::LockContended { .. })
        ));
        first.release().unwrap();
        let second = lock.try_acquire(&path).unwrap();
        drop(second);
        lock.try_acquire(&path).unwrap().release().unwrap();
    }

    #[test]
    fn lock_path_swaps_cannot_create_two_successful_guards() {
        let temporary = TempDirectory::new();
        let path = temporary.path("skilltap.lock");
        let displaced = temporary.path("displaced.lock");
        let lock = SystemConfigurationLock;

        let acquisition_error = try_acquire_with(&path, || {
            fs::rename(path.as_str(), displaced.as_str()).unwrap();
            fs::write(path.as_str(), b"replacement").unwrap();
        })
        .unwrap_err();
        assert!(matches!(
            acquisition_error,
            RuntimeError::LockIdentityChanged { .. }
        ));

        let first = lock.try_acquire(&path).unwrap();
        fs::remove_file(path.as_str()).unwrap();
        fs::write(path.as_str(), b"another inode").unwrap();
        assert!(matches!(
            lock.try_acquire(&path),
            Err(RuntimeError::LockContended { .. })
        ));
        drop(first);
        lock.try_acquire(&path).unwrap().release().unwrap();
    }

    #[test]
    fn lock_open_refuses_symlink_paths_without_touching_the_target() {
        let temporary = TempDirectory::new();
        let target = temporary.path("user-file");
        let link = temporary.path("skilltap.lock");
        fs::write(target.as_str(), b"preserve").unwrap();
        std::os::unix::fs::symlink("user-file", link.as_str()).unwrap();

        assert!(SystemConfigurationLock.try_acquire(&link).is_err());
        assert_eq!(fs::read(target.as_str()).unwrap(), b"preserve");
    }
}
