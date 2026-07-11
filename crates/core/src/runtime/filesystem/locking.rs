use std::{
    fs::{File, TryLockError},
    io,
    path::Path,
};

use crate::{
    domain::AbsolutePath,
    runtime::{LockAction, PathRole, RuntimeError},
};

use super::unix_identity::{
    descriptor_identity_io, open_directory_no_follow, open_lock_no_follow, verify_lock_identity,
};
use super::{
    ConfigurationLock, ConfigurationLockGuard, SystemConfigurationLock,
    SystemConfigurationLockGuard,
};

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

pub(super) fn try_acquire_with(
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
    let directory = ProvisionalLock::acquire(directory, path)?;
    verify_lock_identity(&parent_path, directory_identity, path)?;

    let file = open_lock_no_follow(path)?;
    let identity = descriptor_identity_io(&file).map_err(|source| RuntimeError::Lock {
        action: LockAction::Acquire,
        path: path.clone(),
        source,
    })?;
    let file = ProvisionalLock::acquire(file, path)?;
    after_file_lock();
    verify_lock_identity(path, identity, path)?;

    let file = file.into_file();
    let directory = directory.into_file();

    Ok(SystemConfigurationLockGuard {
        file: Some(file),
        directory: Some(directory),
        path: path.clone(),
    })
}

struct ProvisionalLock(Option<File>);

impl ProvisionalLock {
    fn acquire(file: File, path: &AbsolutePath) -> Result<Self, RuntimeError> {
        match file.try_lock() {
            Ok(()) => Ok(Self(Some(file))),
            Err(TryLockError::WouldBlock) => {
                Err(RuntimeError::LockContended { path: path.clone() })
            }
            Err(TryLockError::Error(source)) => Err(RuntimeError::Lock {
                action: LockAction::Acquire,
                path: path.clone(),
                source,
            }),
        }
    }

    fn into_file(mut self) -> File {
        self.0.take().expect("provisional lock owns its file")
    }
}

impl Drop for ProvisionalLock {
    fn drop(&mut self) {
        if let Some(file) = self.0.take() {
            let _ = file.unlock();
        }
    }
}
