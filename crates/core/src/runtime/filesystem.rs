use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use crate::domain::{AbsolutePath, ValidationError};

use super::{FileSystemAction, LockAction, PathRole, RuntimeError};

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
        if path
            .components()
            .any(|component| matches!(component, std::path::Component::CurDir))
            || path
                .components()
                .all(|component| !matches!(component, std::path::Component::Normal(_)))
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
        if inspect(source)?.kind == FileKind::Symlink {
            return Err(unsafe_symlink(FileSystemAction::Copy, source));
        }
        if inspect(destination)?.kind != FileKind::Missing {
            return Err(filesystem_error(
                FileSystemAction::Copy,
                destination,
                io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "backup destination already exists",
                ),
            ));
        }
        let contents = self.read(source)?;
        let mut destination_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(destination.as_str())
            .map_err(|source| filesystem_error(FileSystemAction::Copy, destination, source))?;
        if let Err(source) = destination_file
            .write_all(&contents)
            .and_then(|()| destination_file.sync_all())
        {
            drop(destination_file);
            let _ = fs::remove_file(destination.as_str());
            return Err(filesystem_error(
                FileSystemAction::Copy,
                destination,
                source,
            ));
        }
        sync_parent(destination, FileSystemAction::Copy)
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
    path: AbsolutePath,
}

impl ConfigurationLock for SystemConfigurationLock {
    type Guard = SystemConfigurationLockGuard;

    fn try_acquire(&self, path: &AbsolutePath) -> Result<Self::Guard, RuntimeError> {
        if inspect(path)?.kind == FileKind::Symlink {
            return Err(unsafe_symlink(FileSystemAction::Write, path));
        }
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path.as_str())
            .map_err(|source| RuntimeError::Lock {
                action: LockAction::Acquire,
                path: path.clone(),
                source,
            })?;
        match file.try_lock() {
            Ok(()) => Ok(SystemConfigurationLockGuard {
                file: Some(file),
                path: path.clone(),
            }),
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
}

impl ConfigurationLockGuard for SystemConfigurationLockGuard {
    fn path(&self) -> &AbsolutePath {
        &self.path
    }

    fn release(mut self) -> Result<(), RuntimeError> {
        let file = self.file.take().expect("lock guard owns its file");
        file.unlock().map_err(|source| RuntimeError::Lock {
            action: LockAction::Release,
            path: self.path.clone(),
            source,
        })
    }
}

impl Drop for SystemConfigurationLockGuard {
    fn drop(&mut self) {
        if let Some(file) = self.file.take() {
            let _ = file.unlock();
        }
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

fn sync_parent(path: &AbsolutePath, action: FileSystemAction) -> Result<(), RuntimeError> {
    let parent = Path::new(path.as_str()).parent().ok_or_else(|| {
        filesystem_error(
            action,
            path,
            io::Error::new(io::ErrorKind::InvalidInput, "path has no parent directory"),
        )
    })?;
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|source| filesystem_error(FileSystemAction::Sync, path, source))
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
        sync::{Arc, atomic::AtomicBool},
        thread,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

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
        for invalid in [
            "",
            "/tmp/AGENTS.md",
            "./AGENTS.md",
            "dir//AGENTS.md",
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
}
