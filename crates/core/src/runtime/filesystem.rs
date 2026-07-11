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
mod tests;
