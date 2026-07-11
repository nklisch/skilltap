use std::{
    fs, io,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

/// A cross-process, filesystem-backed ready/release barrier.
#[derive(Clone)]
pub struct FileBarrier {
    ready: PathBuf,
    release: PathBuf,
}

impl FileBarrier {
    pub fn new(directory: &Path, name: &str) -> io::Result<Self> {
        if name.is_empty()
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "barrier name must be a non-empty portable identifier",
            ));
        }
        fs::create_dir_all(directory)?;
        Ok(Self {
            ready: directory.join(format!("{name}.ready")),
            release: directory.join(format!("{name}.release")),
        })
    }

    pub fn signal_ready(&self) -> io::Result<()> {
        write_marker(&self.ready)
    }

    pub fn release(&self) -> io::Result<()> {
        write_marker(&self.release)
    }

    pub fn wait_until_ready(&self, timeout: Duration) -> io::Result<()> {
        wait_for_marker(&self.ready, timeout)
    }

    pub fn wait_until_released(&self, timeout: Duration) -> io::Result<()> {
        wait_for_marker(&self.release, timeout)
    }

    pub fn reset(&self) -> io::Result<()> {
        remove_if_present(&self.ready)?;
        remove_if_present(&self.release)
    }

    pub(crate) fn ready_path(&self) -> &Path {
        &self.ready
    }

    pub(crate) fn release_path(&self) -> &Path {
        &self.release
    }
}

impl std::fmt::Debug for FileBarrier {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("FileBarrier { .. }")
    }
}

fn write_marker(path: &Path) -> io::Result<()> {
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(error),
    }
}

fn wait_for_marker(path: &Path, timeout: Duration) -> io::Result<()> {
    let deadline = Instant::now() + timeout;
    loop {
        match fs::symlink_metadata(path) {
            Ok(_) => return Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        if Instant::now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "fixture barrier timed out",
            ));
        }
        thread::sleep(Duration::from_millis(2));
    }
}

fn remove_if_present(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, thread};

    use super::*;
    use crate::TempRoot;

    #[test]
    fn barriers_release_waiters_deterministically_and_reset() {
        let root = TempRoot::new("skilltap-file-barrier").unwrap();
        let barrier = Arc::new(FileBarrier::new(&root, "race").unwrap());
        let waiter = {
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.signal_ready().unwrap();
                barrier.wait_until_released(Duration::from_secs(1)).unwrap();
            })
        };

        barrier.wait_until_ready(Duration::from_secs(1)).unwrap();
        barrier.release().unwrap();
        waiter.join().unwrap();
        barrier.reset().unwrap();
        assert_eq!(
            barrier
                .wait_until_ready(Duration::from_millis(5))
                .unwrap_err()
                .kind(),
            io::ErrorKind::TimedOut
        );
    }

    #[test]
    fn barrier_debug_omits_paths() {
        let root = TempRoot::new("secret-barrier-canary").unwrap();
        let barrier = FileBarrier::new(&root, "secret_name").unwrap();
        let rendered = format!("{barrier:?}");
        assert!(!rendered.contains("secret-barrier-canary"));
        assert!(!rendered.contains("secret_name"));
    }
}
