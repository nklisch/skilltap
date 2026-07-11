use std::{
    ffi::OsString,
    fmt,
    process::{Command, ExitStatus, Stdio},
    time::{Duration, Instant},
};

use crate::domain::{AbsolutePath, NativeId};

use super::{CommandAction, RuntimeError};

#[derive(Clone, Eq, PartialEq)]
pub struct CommandRequest {
    executable: NativeId,
    arguments: Vec<OsString>,
    working_directory: Option<AbsolutePath>,
}

impl CommandRequest {
    pub fn new(
        executable: NativeId,
        arguments: impl IntoIterator<Item = OsString>,
        working_directory: Option<AbsolutePath>,
    ) -> Self {
        Self {
            executable,
            arguments: arguments.into_iter().collect(),
            working_directory,
        }
    }

    pub const fn executable(&self) -> &NativeId {
        &self.executable
    }

    pub fn arguments(&self) -> &[OsString] {
        &self.arguments
    }

    pub const fn working_directory(&self) -> Option<&AbsolutePath> {
        self.working_directory.as_ref()
    }
}

impl fmt::Debug for CommandRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CommandRequest")
            .field("executable", &self.executable)
            .field("argument_count", &self.arguments.len())
            .field("working_directory", &self.working_directory)
            .finish()
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct CommandOutput {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    elapsed: Duration,
}

impl CommandOutput {
    #[cfg(test)]
    pub(crate) const fn for_test(
        status: ExitStatus,
        stdout: Vec<u8>,
        stderr: Vec<u8>,
        elapsed: Duration,
    ) -> Self {
        Self {
            status,
            stdout,
            stderr,
            elapsed,
        }
    }

    pub const fn status(&self) -> ExitStatus {
        self.status
    }

    pub fn stdout(&self) -> &[u8] {
        &self.stdout
    }

    pub fn stderr(&self) -> &[u8] {
        &self.stderr
    }

    pub const fn elapsed(&self) -> Duration {
        self.elapsed
    }
}

impl fmt::Debug for CommandOutput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CommandOutput")
            .field("status", &self.status)
            .field("stdout_bytes", &self.stdout.len())
            .field("stderr_bytes", &self.stderr.len())
            .field("elapsed", &self.elapsed)
            .finish()
    }
}

pub trait CommandRunner {
    fn run(&self, request: &CommandRequest) -> Result<CommandOutput, RuntimeError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SystemCommandRunner;

impl CommandRunner for SystemCommandRunner {
    fn run(&self, request: &CommandRequest) -> Result<CommandOutput, RuntimeError> {
        let mut command = Command::new(request.executable.as_str());
        command
            .args(request.arguments.iter().map(OsString::as_os_str))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(working_directory) = &request.working_directory {
            command.current_dir(working_directory.as_str());
        }

        let started = Instant::now();
        let child = command.spawn().map_err(|source| RuntimeError::Command {
            action: CommandAction::Spawn,
            executable: request.executable.clone(),
            source,
        })?;
        let output = child
            .wait_with_output()
            .map_err(|source| RuntimeError::Command {
                action: CommandAction::Wait,
                executable: request.executable.clone(),
                source,
            })?;

        Ok(CommandOutput {
            status: output.status,
            stdout: output.stdout,
            stderr: output.stderr,
            elapsed: started.elapsed(),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsStr, fs};

    use skilltap_test_support::TempRoot;

    use super::*;

    struct Fixture {
        root: TempRoot,
        executable: NativeId,
    }

    impl Fixture {
        fn compile() -> Self {
            let root = TempRoot::new("skilltap-command-fixture").unwrap();
            let source = root.join("fixture.rs");
            let executable_path = root.join("fixture");
            fs::write(
                &source,
                include_str!("../../tests/fixtures/command_fixture.rs"),
            )
            .unwrap();
            let status = Command::new("rustc")
                .args([
                    source.as_os_str(),
                    OsStr::new("-o"),
                    executable_path.as_os_str(),
                ])
                .status()
                .unwrap();
            assert!(status.success());

            Self {
                root,
                executable: NativeId::new(executable_path.into_os_string().into_string().unwrap())
                    .unwrap(),
            }
        }

        fn working_directory(&self) -> AbsolutePath {
            AbsolutePath::new(self.root.to_str().unwrap()).unwrap()
        }
    }

    #[test]
    fn direct_arguments_working_directory_and_output_are_preserved() {
        let fixture = Fixture::compile();
        let arguments = ["space value", "$HOME", "*.md", "semi;colon"];
        let request = CommandRequest::new(
            fixture.executable.clone(),
            arguments.into_iter().map(OsString::from),
            Some(fixture.working_directory()),
        );

        let output = SystemCommandRunner.run(&request).unwrap();

        assert!(output.status().success());
        let stdout = String::from_utf8(output.stdout().to_vec()).unwrap();
        assert!(stdout.contains(&format!("cwd={}", fixture.root.display())));
        for (index, argument) in arguments.iter().enumerate() {
            assert!(stdout.contains(&format!("arg[{index}]={argument}")));
        }
        assert_eq!(output.stderr(), b"fixture-stderr\n");
        assert!(output.elapsed() <= Duration::from_secs(30));
    }

    #[test]
    fn non_zero_exit_is_captured_as_a_result() {
        let fixture = Fixture::compile();
        let request = CommandRequest::new(
            fixture.executable.clone(),
            [OsString::from("--exit=17")],
            None,
        );

        let output = SystemCommandRunner.run(&request).unwrap();

        assert_eq!(output.status().code(), Some(17));
        assert!(String::from_utf8_lossy(output.stdout()).contains("arg[0]=--exit=17"));
        assert_eq!(output.stderr(), b"fixture-stderr\n");
    }

    #[test]
    fn request_output_and_errors_are_debug_safe() {
        let secret = "secret-token-in-argument";
        let request = CommandRequest::new(
            NativeId::new("/definitely/missing/skilltap-command-fixture").unwrap(),
            [OsString::from(secret)],
            None,
        );

        assert!(!format!("{request:?}").contains(secret));
        let error = SystemCommandRunner.run(&request).unwrap_err();
        assert!(matches!(
            error,
            RuntimeError::Command {
                action: CommandAction::Spawn,
                ..
            }
        ));
        assert!(!error.to_string().contains(secret));
        assert!(!format!("{error:?}").contains(secret));

        let output = CommandOutput {
            status: Command::new("rustc")
                .arg("--version")
                .output()
                .unwrap()
                .status,
            stdout: secret.as_bytes().to_vec(),
            stderr: secret.as_bytes().to_vec(),
            elapsed: Duration::ZERO,
        };
        assert!(!format!("{output:?}").contains(secret));
    }
}
