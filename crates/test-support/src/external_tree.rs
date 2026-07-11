use std::{
    fs::{self, File},
    io,
    os::unix::{fs::symlink, net::UnixListener},
    path::{Component, Path, PathBuf},
    process::Command,
    time::Duration,
};

use crate::{FileBarrier, TempRoot};

/// An isolated external tree containing only caller-requested adversarial entries.
pub struct ExternalTreeFixture {
    root: TempRoot,
}

impl ExternalTreeFixture {
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            root: TempRoot::new("skilltap-external-tree")?,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn deep_tree(&self, depth: usize) -> io::Result<PathBuf> {
        let mut current = self.root.join("deep");
        fs::create_dir(&current)?;
        for index in 0..depth {
            current = current.join(format!("level-{index:06}"));
            fs::create_dir(&current)?;
        }
        fs::write(current.join("leaf"), b"leaf")?;
        Ok(current)
    }

    pub fn wide_tree(&self, entries: usize) -> io::Result<PathBuf> {
        let directory = self.root.join("wide");
        fs::create_dir(&directory)?;
        for index in 0..entries {
            fs::write(directory.join(format!("entry-{index:06}")), b"entry")?;
        }
        Ok(directory)
    }

    pub fn oversized_file(&self, relative: &Path, logical_bytes: u64) -> io::Result<PathBuf> {
        let path = self.resolve(relative)?;
        create_parent(&path)?;
        File::create(&path)?.set_len(logical_bytes)?;
        Ok(path)
    }

    pub fn live_symlink(&self, relative: &Path, target: &Path) -> io::Result<PathBuf> {
        let link = self.resolve(relative)?;
        create_parent(&link)?;
        symlink(target, &link)?;
        Ok(link)
    }

    pub fn dangling_symlink(&self, relative: &Path) -> io::Result<PathBuf> {
        self.live_symlink(relative, Path::new("missing-target"))
    }

    pub fn fifo(&self, relative: &Path) -> io::Result<PathBuf> {
        let path = self.resolve(relative)?;
        create_parent(&path)?;
        let executable = ["/usr/bin/mkfifo", "/bin/mkfifo"]
            .into_iter()
            .find(|candidate| Path::new(candidate).is_file())
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "mkfifo is unavailable"))?;
        let status = Command::new(executable).arg(&path).status()?;
        if !status.success() {
            return Err(io::Error::other("mkfifo fixture command failed"));
        }
        Ok(path)
    }

    pub fn live_socket(&self, relative: &Path) -> io::Result<UnixListener> {
        let path = self.resolve(relative)?;
        create_parent(&path)?;
        UnixListener::bind(path)
    }

    pub fn permission_denied_fault(&self, name: &str) -> io::Result<InjectedIoFault> {
        InjectedIoFault::new(
            &self.root.join("barriers"),
            name,
            io::ErrorKind::PermissionDenied,
        )
    }

    pub fn prepare_file_replacement(
        &self,
        relative: &Path,
        replacement: &[u8],
    ) -> io::Result<ReplacementRace> {
        let target = self.resolve(relative)?;
        create_parent(&target)?;
        let staged = unique_sibling(&target, "file-replacement");
        fs::write(&staged, replacement)?;
        ReplacementRace::new(&self.root, target, staged, ReplacementKind::File)
    }

    pub fn prepare_tree_replacement(
        &self,
        relative: &Path,
        populate: impl FnOnce(&Path) -> io::Result<()>,
    ) -> io::Result<ReplacementRace> {
        let target = self.resolve(relative)?;
        create_parent(&target)?;
        let staged = unique_sibling(&target, "tree-replacement");
        fs::create_dir(&staged)?;
        populate(&staged)?;
        ReplacementRace::new(&self.root, target, staged, ReplacementKind::Tree)
    }

    pub fn resolve(&self, relative: &Path) -> io::Result<PathBuf> {
        validate_relative(relative)?;
        Ok(self.root.join(relative))
    }
}

impl std::fmt::Debug for ExternalTreeFixture {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ExternalTreeFixture { .. }")
    }
}

#[derive(Clone)]
pub struct InjectedIoFault {
    barrier: FileBarrier,
    kind: io::ErrorKind,
}

impl InjectedIoFault {
    fn new(directory: &Path, name: &str, kind: io::ErrorKind) -> io::Result<Self> {
        Ok(Self {
            barrier: FileBarrier::new(directory, name)?,
            kind,
        })
    }

    pub fn barrier(&self) -> &FileBarrier {
        &self.barrier
    }

    pub fn fail_after_release(&self, timeout: Duration) -> io::Result<()> {
        self.barrier.signal_ready()?;
        self.barrier.wait_until_released(timeout)?;
        Err(io::Error::from(self.kind))
    }
}

impl std::fmt::Debug for InjectedIoFault {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("InjectedIoFault")
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, Debug)]
enum ReplacementKind {
    File,
    Tree,
}

#[derive(Clone)]
pub struct ReplacementRace {
    target: PathBuf,
    staged: PathBuf,
    kind: ReplacementKind,
    barrier: FileBarrier,
}

impl ReplacementRace {
    fn new(
        fixture_root: &Path,
        target: PathBuf,
        staged: PathBuf,
        kind: ReplacementKind,
    ) -> io::Result<Self> {
        static NEXT_RACE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let sequence = NEXT_RACE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(Self {
            target,
            staged,
            kind,
            barrier: FileBarrier::new(
                &fixture_root.join("barriers"),
                &format!("replacement-{sequence}"),
            )?,
        })
    }

    pub fn barrier(&self) -> &FileBarrier {
        &self.barrier
    }

    pub fn replace_after_release(&self, timeout: Duration) -> io::Result<()> {
        self.barrier.signal_ready()?;
        self.barrier.wait_until_released(timeout)?;
        match self.kind {
            ReplacementKind::File => fs::rename(&self.staged, &self.target),
            ReplacementKind::Tree => {
                match fs::remove_dir_all(&self.target) {
                    Ok(()) => {}
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error),
                }
                fs::rename(&self.staged, &self.target)
            }
        }
    }
}

impl std::fmt::Debug for ReplacementRace {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ReplacementRace")
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

fn create_parent(path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn unique_sibling(target: &Path, suffix: &str) -> PathBuf {
    static NEXT_STAGED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT_STAGED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    target.with_extension(format!("{suffix}-{sequence}"))
}

fn validate_relative(path: &Path) -> io::Result<()> {
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "fixture entry path must be normalized and relative",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{os::unix::fs::FileTypeExt, sync::Arc, thread};

    use super::*;

    #[test]
    fn creates_bounded_shape_and_special_entry_fixtures() {
        let tree = ExternalTreeFixture::new().unwrap();
        let deep = tree.deep_tree(4).unwrap();
        assert!(deep.join("leaf").is_file());
        let wide = tree.wide_tree(7).unwrap();
        assert_eq!(fs::read_dir(wide).unwrap().count(), 7);
        let oversized = tree
            .oversized_file(Path::new("large/payload"), 1_048_577)
            .unwrap();
        assert_eq!(fs::metadata(oversized).unwrap().len(), 1_048_577);

        fs::write(tree.root().join("target"), b"target").unwrap();
        let live = tree
            .live_symlink(Path::new("live-link"), Path::new("target"))
            .unwrap();
        let dangling = tree.dangling_symlink(Path::new("dangling-link")).unwrap();
        assert!(fs::symlink_metadata(live).unwrap().file_type().is_symlink());
        assert!(
            fs::symlink_metadata(dangling)
                .unwrap()
                .file_type()
                .is_symlink()
        );

        let fifo = tree.fifo(Path::new("special/fifo")).unwrap();
        assert!(fs::symlink_metadata(fifo).unwrap().file_type().is_fifo());
        let socket_path = tree.resolve(Path::new("special/socket")).unwrap();
        let _socket = tree.live_socket(Path::new("special/socket")).unwrap();
        assert!(
            fs::symlink_metadata(socket_path)
                .unwrap()
                .file_type()
                .is_socket()
        );
    }

    #[test]
    fn permission_fault_is_barrier_driven_and_deterministic() {
        let tree = ExternalTreeFixture::new().unwrap();
        let fault = Arc::new(tree.permission_denied_fault("permission").unwrap());
        let worker = {
            let fault = Arc::clone(&fault);
            thread::spawn(move || fault.fail_after_release(Duration::from_secs(1)))
        };
        fault
            .barrier()
            .wait_until_ready(Duration::from_secs(1))
            .unwrap();
        fault.barrier().release().unwrap();
        assert_eq!(
            worker.join().unwrap().unwrap_err().kind(),
            io::ErrorKind::PermissionDenied
        );
    }

    #[test]
    fn file_and_tree_replacements_wait_for_explicit_release() {
        let tree = ExternalTreeFixture::new().unwrap();
        let file = tree.resolve(Path::new("raced-file")).unwrap();
        fs::write(&file, b"old").unwrap();
        let race = Arc::new(
            tree.prepare_file_replacement(Path::new("raced-file"), b"new-secret-bytes")
                .unwrap(),
        );
        let worker = {
            let race = Arc::clone(&race);
            thread::spawn(move || race.replace_after_release(Duration::from_secs(1)))
        };
        race.barrier()
            .wait_until_ready(Duration::from_secs(1))
            .unwrap();
        assert_eq!(fs::read(&file).unwrap(), b"old");
        race.barrier().release().unwrap();
        worker.join().unwrap().unwrap();
        assert_eq!(fs::read(&file).unwrap(), b"new-secret-bytes");
        assert!(!format!("{race:?}").contains("new-secret-bytes"));

        let directory = tree.resolve(Path::new("raced-tree")).unwrap();
        fs::create_dir(&directory).unwrap();
        fs::write(directory.join("old"), b"old").unwrap();
        let race = Arc::new(
            tree.prepare_tree_replacement(Path::new("raced-tree"), |staged| {
                fs::write(staged.join("new"), b"new")
            })
            .unwrap(),
        );
        let worker = {
            let race = Arc::clone(&race);
            thread::spawn(move || race.replace_after_release(Duration::from_secs(1)))
        };
        race.barrier()
            .wait_until_ready(Duration::from_secs(1))
            .unwrap();
        assert!(directory.join("old").is_file());
        race.barrier().release().unwrap();
        worker.join().unwrap().unwrap();
        assert!(directory.join("new").is_file());
        assert!(!directory.join("old").exists());
    }

    #[test]
    fn entry_paths_and_debug_rendering_are_safe() {
        let tree = ExternalTreeFixture::new().unwrap();
        assert_eq!(
            tree.resolve(Path::new("../escape")).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
        assert!(!format!("{tree:?}").contains(tree.root().to_str().unwrap()));
    }
}
