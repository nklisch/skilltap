use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    ffi::CString,
    io,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

use skilltap_test_support::TempRoot;

use super::*;
use crate::{
    runtime::{FileKind, FileMetadata, FileSystemAction, RelativeSymlinkTarget, SystemFileSystem},
    storage::{CONFIG_SCHEMA_VERSION, INVENTORY_SCHEMA_VERSION, STATE_SCHEMA_VERSION},
};

#[derive(Default)]
struct FakeFileSystem {
    files: RefCell<BTreeMap<AbsolutePath, Vec<u8>>>,
    kinds: RefCell<BTreeMap<AbsolutePath, FileKind>>,
    created: RefCell<Vec<AbsolutePath>>,
    writes: RefCell<Vec<(AbsolutePath, Vec<u8>)>>,
    fail_create: Cell<bool>,
    fail_write: Cell<bool>,
}

impl FakeFileSystem {
    fn put(&self, path: AbsolutePath, contents: impl Into<Vec<u8>>) {
        self.files.borrow_mut().insert(path, contents.into());
    }

    fn bytes(&self, path: &AbsolutePath) -> Option<Vec<u8>> {
        self.files.borrow().get(path).cloned()
    }

    fn error(action: FileSystemAction, path: &AbsolutePath) -> RuntimeError {
        RuntimeError::FileSystem {
            action,
            path: path.clone(),
            source: io::Error::other("secret-runtime-detail"),
        }
    }
}

impl FileSystem for FakeFileSystem {
    fn inspect(&self, _path: &AbsolutePath) -> Result<FileMetadata, RuntimeError> {
        unreachable!("document repositories must not inspect before descriptor-bound reads")
    }

    fn canonicalize(&self, _path: &AbsolutePath) -> Result<AbsolutePath, RuntimeError> {
        unreachable!("document repositories do not canonicalize configured paths")
    }

    fn create_directory_all(&self, path: &AbsolutePath) -> Result<(), RuntimeError> {
        if self.fail_create.get() {
            return Err(Self::error(FileSystemAction::CreateDirectory, path));
        }
        self.created.borrow_mut().push(path.clone());
        Ok(())
    }

    fn read(&self, _path: &AbsolutePath) -> Result<Vec<u8>, RuntimeError> {
        unreachable!("document repositories use descriptor-bound reads")
    }

    fn read_regular_no_follow(&self, path: &AbsolutePath) -> Result<Option<Vec<u8>>, RuntimeError> {
        let kind = self.kinds.borrow().get(path).copied().unwrap_or_else(|| {
            if self.files.borrow().contains_key(path) {
                FileKind::RegularFile
            } else {
                FileKind::Missing
            }
        });
        match kind {
            FileKind::Missing => Ok(None),
            FileKind::RegularFile => self
                .files
                .borrow()
                .get(path)
                .cloned()
                .map(Some)
                .ok_or_else(|| Self::error(FileSystemAction::Read, path)),
            FileKind::Directory | FileKind::Symlink | FileKind::Other => {
                Err(Self::error(FileSystemAction::Read, path))
            }
        }
    }

    fn atomic_write(&self, path: &AbsolutePath, contents: &[u8]) -> Result<(), RuntimeError> {
        if self.fail_write.get() {
            return Err(Self::error(FileSystemAction::Write, path));
        }
        self.writes
            .borrow_mut()
            .push((path.clone(), contents.to_vec()));
        self.files
            .borrow_mut()
            .insert(path.clone(), contents.to_vec());
        Ok(())
    }

    fn copy_recoverable(
        &self,
        _source: &AbsolutePath,
        _destination: &AbsolutePath,
    ) -> Result<(), RuntimeError> {
        unreachable!("document repositories do not copy artifacts")
    }

    fn create_relative_symlink(
        &self,
        _target: &RelativeSymlinkTarget,
        _link: &AbsolutePath,
    ) -> Result<(), RuntimeError> {
        unreachable!("document repositories do not create links")
    }

    fn remove(&self, _path: &AbsolutePath) -> Result<(), RuntimeError> {
        unreachable!("document repositories do not remove owned documents")
    }
}

fn root() -> AbsolutePath {
    AbsolutePath::new("/machine/config/skilltap").unwrap()
}

fn path(name: &str) -> AbsolutePath {
    AbsolutePath::new(format!("{}/{name}", root())).unwrap()
}

fn empty_inventory() -> InventoryDocument {
    InventoryDocument::new(INVENTORY_SCHEMA_VERSION, [], []).unwrap()
}

fn empty_state() -> StateDocument {
    StateDocument::new(STATE_SCHEMA_VERSION, [], [], None, None, None).unwrap()
}

include!("tests/codec.rs");

#[test]
fn missing_loads_are_explicit_and_create_nothing() {
    let filesystem = FakeFileSystem::default();
    let config = FileConfigRepository::new(&filesystem, root()).unwrap();
    let inventory = FileInventoryRepository::new(&filesystem, root()).unwrap();
    let state = FileStateRepository::new(&filesystem, root()).unwrap();

    assert_eq!(config.load().unwrap(), DocumentState::Missing);
    assert_eq!(inventory.load().unwrap(), DocumentState::Missing);
    assert_eq!(state.load().unwrap(), DocumentState::Missing);
    assert!(filesystem.created.borrow().is_empty());
    assert!(filesystem.writes.borrow().is_empty());
    assert!(filesystem.files.borrow().is_empty());
}

#[test]
fn failed_publication_preserves_old_bytes_and_reports_safe_write_context() {
    let filesystem = FakeFileSystem::default();
    let config_path = path("config.toml");
    let old = b"old-complete-document".to_vec();
    filesystem.put(config_path.clone(), old.clone());
    filesystem.fail_write.set(true);
    let repository = FileConfigRepository::new(&filesystem, root()).unwrap();

    let error = repository.replace(&ConfigDocument::defaults()).unwrap_err();

    assert_eq!(error.action(), DocumentAction::Write);
    assert_eq!(error.failure(), StorageFailure::Runtime);
    assert_eq!(filesystem.bytes(&config_path), Some(old));
    assert!(!error.to_string().contains("secret-runtime-detail"));
    assert!(!format!("{error:?}").contains("secret-runtime-detail"));
    assert!(std::error::Error::source(&error).is_none());
}

#[test]
fn create_failure_happens_after_validation_and_before_atomic_write() {
    let filesystem = FakeFileSystem::default();
    filesystem.fail_create.set(true);
    let repository = FileStateRepository::new(&filesystem, root()).unwrap();

    let error = repository.replace(&empty_state()).unwrap_err();

    assert_eq!(error.action(), DocumentAction::Write);
    assert_eq!(error.failure(), StorageFailure::Runtime);
    assert!(filesystem.writes.borrow().is_empty());
    assert!(filesystem.files.borrow().is_empty());
}

#[test]
fn non_regular_owned_documents_are_never_followed_or_decoded() {
    let filesystem = FakeFileSystem::default();
    let config_path = path("config.toml");
    filesystem
        .kinds
        .borrow_mut()
        .insert(config_path.clone(), FileKind::Symlink);
    let repository = FileConfigRepository::new(&filesystem, root()).unwrap();

    let error = repository.load().unwrap_err();

    assert_eq!(error.action(), DocumentAction::Read);
    assert_eq!(error.failure(), StorageFailure::Runtime);
    assert!(std::error::Error::source(&error).is_none());
}

#[test]
fn system_adapter_missing_read_creates_nothing_then_round_trips_first_replace() {
    let temporary = TempRoot::new("skilltap-document-repository-test").unwrap();
    let config_root = temporary.join("skilltap");
    let config_root = AbsolutePath::new(config_root.to_str().unwrap()).unwrap();
    let repository = FileConfigRepository::new(&SystemFileSystem, config_root.clone()).unwrap();
    assert!(!std::path::Path::new(config_root.as_str()).exists());

    assert_eq!(repository.load().unwrap(), DocumentState::Missing);
    assert!(!std::path::Path::new(config_root.as_str()).exists());

    let config = ConfigDocument::defaults();
    repository.replace(&config).unwrap();
    assert_eq!(repository.load().unwrap(), DocumentState::Present(config));
    assert_eq!(
        std::fs::read(temporary.join("skilltap/config.toml")).unwrap(),
        include_bytes!("../fixtures/config.toml")
    );
}

#[cfg(unix)]
#[test]
fn repository_load_rejects_fifo_documents_without_hanging() {
    let temporary = TempRoot::new("skilltap-document-repository-fifo-test").unwrap();
    let config_root = temporary.join("skilltap");
    std::fs::create_dir(&config_root).unwrap();
    let fifo = config_root.join("config.toml");
    let fifo_path = CString::new(fifo.to_str().unwrap()).unwrap();
    // SAFETY: `fifo_path` is a live NUL-terminated path and the mode is valid.
    assert_eq!(unsafe { libc::mkfifo(fifo_path.as_ptr(), 0o600) }, 0);
    let config_root = AbsolutePath::new(config_root.to_str().unwrap()).unwrap();
    let (sender, receiver) = mpsc::channel();

    thread::spawn(move || {
        let result = FileConfigRepository::new(&SystemFileSystem, config_root)
            .unwrap()
            .load();
        sender.send(result).unwrap();
    });

    let error = receiver
        .recv_timeout(Duration::from_secs(2))
        .expect("repository FIFO load must not wait for a writer")
        .unwrap_err();
    assert_eq!(error.action(), DocumentAction::Read);
    assert_eq!(error.failure(), StorageFailure::Runtime);
    assert!(std::error::Error::source(&error).is_none());
}

#[test]
fn system_adapter_readers_observe_only_old_or_new_complete_documents() {
    let temporary = TempRoot::new("skilltap-document-repository-atomic-test").unwrap();
    let config_root = temporary.join("skilltap");
    let config_root = AbsolutePath::new(config_root.to_str().unwrap()).unwrap();
    let repository = FileConfigRepository::new(&SystemFileSystem, config_root.clone()).unwrap();
    let old = ConfigDocument::defaults();
    let mut harnesses = old.harnesses().clone();
    harnesses.codex.binary = super::super::HarnessBinary::new("/usr/local/bin/codex").unwrap();
    let new = ConfigDocument::new(
        CONFIG_SCHEMA_VERSION,
        harnesses,
        old.instructions().clone(),
        old.updates().clone(),
    )
    .unwrap();
    repository.replace(&old).unwrap();

    let document_path = PathBuf::from(config_root.as_str()).join("config.toml");
    let old_bytes = toml::to_string_pretty(&old).unwrap().into_bytes();
    let new_bytes = toml::to_string_pretty(&new).unwrap().into_bytes();
    let reader_path = document_path.clone();
    let reader_old = old_bytes.clone();
    let reader_new = new_bytes.clone();
    let running = Arc::new(AtomicBool::new(true));
    let reader_running = Arc::clone(&running);
    let reader = thread::spawn(move || {
        while reader_running.load(Ordering::Relaxed) {
            let observed = std::fs::read(&reader_path).unwrap();
            assert!(observed == reader_old || observed == reader_new);
        }
    });

    repository.replace(&new).unwrap();
    running.store(false, Ordering::Relaxed);
    reader.join().unwrap();
    assert_eq!(std::fs::read(document_path).unwrap(), new_bytes);
}
