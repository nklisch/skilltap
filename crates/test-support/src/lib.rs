//! Shared test fixtures for skilltap workspace crates.

#[cfg(unix)]
mod barrier;
#[cfg(unix)]
mod external_tree;
#[cfg(unix)]
mod native_process;

#[cfg(unix)]
pub use barrier::FileBarrier;
#[cfg(unix)]
pub use external_tree::{ExternalTreeFixture, InjectedIoFault, ReplacementRace};
#[cfg(unix)]
pub use native_process::{
    CapturedInvocation, FakeNativeBuilder, FakeNativeMode, FakeNativeProcess, PipeHolder,
};

use std::{
    ffi::OsStr,
    fs, io,
    ops::Deref,
    path::{Path, PathBuf},
    process::{Command, Output},
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

/// An isolated home, XDG configuration home, and working directory for process tests.
#[derive(Debug)]
pub struct IsolatedMachine {
    _root: TempRoot,
    home: PathBuf,
    configuration_home: PathBuf,
    working_directory: PathBuf,
}

impl IsolatedMachine {
    pub fn new(prefix: &str) -> io::Result<Self> {
        let root = TempRoot::new(prefix)?;
        let home = root.join("home");
        let configuration_home = root.join("xdg");
        let working_directory = root.join("work");
        fs::create_dir_all(&home)?;
        fs::create_dir_all(&configuration_home)?;
        fs::create_dir_all(&working_directory)?;
        Ok(Self {
            _root: root,
            home,
            configuration_home,
            working_directory,
        })
    }

    pub fn home(&self) -> &Path {
        &self.home
    }

    pub fn configuration_home(&self) -> &Path {
        &self.configuration_home
    }

    pub fn working_directory(&self) -> &Path {
        &self.working_directory
    }

    pub fn run(&self, binary: &Path, arguments: &[&str]) -> io::Result<Output> {
        self.run_in(binary, self.working_directory(), arguments)
    }

    pub fn run_in(
        &self,
        binary: &Path,
        working_directory: &Path,
        arguments: &[&str],
    ) -> io::Result<Output> {
        Command::new(binary)
            .args(arguments)
            .current_dir(working_directory)
            .env("HOME", &self.home)
            .env("XDG_CONFIG_HOME", &self.configuration_home)
            .env_remove("SKILLTAP_HOME")
            .output()
    }
}

/// Resolves the compiled test binary, honoring an absolute or working-directory-relative override.
pub fn compiled_binary(default: impl Into<PathBuf>) -> io::Result<PathBuf> {
    let Some(override_path) = std::env::var_os("SKILLTAP_TEST_BIN") else {
        return Ok(default.into());
    };
    let override_path = PathBuf::from(override_path);
    if override_path.is_absolute() {
        Ok(override_path)
    } else {
        Ok(std::env::current_dir()?.join(override_path))
    }
}

pub fn captured_stdout(output: &Output) -> Result<&str, std::str::Utf8Error> {
    std::str::from_utf8(&output.stdout)
}

pub fn captured_stderr(output: &Output) -> Result<&str, std::str::Utf8Error> {
    std::str::from_utf8(&output.stderr)
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

    #[test]
    fn isolated_machine_creates_distinct_environment_roots() {
        let machine = IsolatedMachine::new("skilltap-isolated-machine-test").unwrap();

        assert!(machine.home().is_dir());
        assert!(machine.configuration_home().is_dir());
        assert!(machine.working_directory().is_dir());
        assert_ne!(machine.home(), machine.configuration_home());
        assert_ne!(machine.home(), machine.working_directory());
    }

    #[test]
    fn captured_output_helpers_validate_utf8() {
        let output = Output {
            status: success_status(),
            stdout: b"stdout".to_vec(),
            stderr: b"stderr".to_vec(),
        };

        assert_eq!(captured_stdout(&output).unwrap(), "stdout");
        assert_eq!(captured_stderr(&output).unwrap(), "stderr");
    }

    #[cfg(unix)]
    fn success_status() -> std::process::ExitStatus {
        use std::os::unix::process::ExitStatusExt;
        std::process::ExitStatus::from_raw(0)
    }

    #[cfg(windows)]
    fn success_status() -> std::process::ExitStatus {
        use std::os::windows::process::ExitStatusExt;
        std::process::ExitStatus::from_raw(0)
    }
}
