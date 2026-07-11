//! Shared test fixtures for skilltap workspace crates.

use std::{
    ffi::OsStr,
    fs, io,
    ops::Deref,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_TEMP_ROOT: AtomicU64 = AtomicU64::new(0);

/// A uniquely named temporary directory removed on a best-effort basis.
#[derive(Debug)]
pub struct TempRoot(PathBuf);

impl TempRoot {
    pub fn new(prefix: &str) -> io::Result<Self> {
        loop {
            let sequence = NEXT_TEMP_ROOT.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("{prefix}-{}-{sequence}", std::process::id()));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error),
            }
        }
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl AsRef<OsStr> for TempRoot {
    fn as_ref(&self) -> &OsStr {
        self.0.as_os_str()
    }
}

impl AsRef<Path> for TempRoot {
    fn as_ref(&self) -> &Path {
        self.path()
    }
}

impl Deref for TempRoot {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        self.path()
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roots_are_unique_created_and_removed_on_drop() {
        let first = TempRoot::new("skilltap-temp-root-test").unwrap();
        let second = TempRoot::new("skilltap-temp-root-test").unwrap();
        assert_ne!(first.path(), second.path());
        assert!(first.path().is_dir());
        assert!(second.path().is_dir());

        let first_path = first.path().to_owned();
        drop(first);
        assert!(!first_path.exists());
    }
}
