use std::{
    collections::BTreeMap,
    fs, io,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};

use crate::{FileBarrier, TempRoot};

const ESCAPED_PIPE_HOLDER: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/escaped-pipe-holder"));

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PipeHolder {
    Child,
    Descendant,
    EscapedDescendant,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FakeNativeMode {
    Exit(u8),
    Hang,
    Flood {
        stdout_bytes: u64,
        stderr_bytes: u64,
    },
    RetainPipes(PipeHolder),
}

/// Builds one neutral fake-native executable with isolated byte captures.
#[derive(Debug)]
pub struct FakeNativeBuilder {
    mode: FakeNativeMode,
    environment: Vec<String>,
    start_barrier: bool,
}

impl FakeNativeBuilder {
    pub fn new(mode: FakeNativeMode) -> Self {
        Self {
            mode,
            environment: Vec::new(),
            start_barrier: false,
        }
    }

    pub fn capture_environment(
        mut self,
        names: impl IntoIterator<Item = impl Into<String>>,
    ) -> io::Result<Self> {
        for name in names {
            let name = name.into();
            validate_environment_name(&name)?;
            if !self.environment.contains(&name) {
                self.environment.push(name);
            }
        }
        self.environment.sort();
        Ok(self)
    }

    pub fn wait_for_release(mut self) -> Self {
        self.start_barrier = true;
        self
    }

    pub fn build(self) -> io::Result<FakeNativeProcess> {
        let root = TempRoot::new("skilltap-fake-native")?;
        let captures = root.join("captures");
        fs::create_dir_all(captures.join("argv"))?;
        fs::create_dir_all(captures.join("environment"))?;
        for name in &self.environment {
            fs::create_dir(captures.join("environment").join(name))?;
        }

        let start_barrier = self
            .start_barrier
            .then(|| FileBarrier::new(&root.join("barriers"), "start"))
            .transpose()?;
        let pipe_holder_barrier = matches!(self.mode, FakeNativeMode::RetainPipes(_))
            .then(|| FileBarrier::new(&root.join("barriers"), "pipe-holder"))
            .transpose()?;
        let escaped_helper = matches!(
            self.mode,
            FakeNativeMode::RetainPipes(PipeHolder::EscapedDescendant)
        )
        .then(|| {
            let path = root.join("escaped-pipe-holder");
            materialize_executable(&path, ESCAPED_PIPE_HOLDER)?;
            Ok::<_, io::Error>(path)
        })
        .transpose()?;
        let executable = root.join("fake-native");
        let script = render_script(
            &captures,
            &self.environment,
            self.mode,
            start_barrier.as_ref(),
            pipe_holder_barrier.as_ref(),
            escaped_helper.as_deref(),
        );
        materialize_executable(&executable, script.as_bytes())?;

        Ok(FakeNativeProcess {
            _root: root,
            executable,
            captures,
            environment: self.environment,
            mode: self.mode,
            start_barrier,
            pipe_holder_barrier,
        })
    }
}

/// A materialized fake executable. Debug intentionally omits paths and captured values.
pub struct FakeNativeProcess {
    _root: TempRoot,
    executable: PathBuf,
    captures: PathBuf,
    environment: Vec<String>,
    mode: FakeNativeMode,
    start_barrier: Option<FileBarrier>,
    pipe_holder_barrier: Option<FileBarrier>,
}

impl FakeNativeProcess {
    pub fn new(mode: FakeNativeMode) -> io::Result<Self> {
        FakeNativeBuilder::new(mode).build()
    }

    pub fn command(&self) -> Command {
        Command::new(&self.executable)
    }

    pub fn executable(&self) -> &Path {
        &self.executable
    }

    pub fn start_barrier(&self) -> Option<&FileBarrier> {
        self.start_barrier.as_ref()
    }

    pub fn pipe_holder_barrier(&self) -> Option<&FileBarrier> {
        self.pipe_holder_barrier.as_ref()
    }

    pub fn captured_invocation(&self) -> io::Result<CapturedInvocation> {
        let count = fs::read_to_string(self.captures.join("argument-count"))?
            .parse::<usize>()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid argument count"))?;
        let mut arguments = Vec::with_capacity(count);
        for index in 0..count {
            arguments.push(fs::read(
                self.captures.join("argv").join(format!("{index:020}")),
            )?);
        }
        let mut environment = BTreeMap::new();
        for name in &self.environment {
            let directory = self.captures.join("environment").join(name);
            let present = fs::read(directory.join("present"))? == b"1";
            environment.insert(
                name.clone(),
                present
                    .then(|| fs::read(directory.join("value")))
                    .transpose()?,
            );
        }
        Ok(CapturedInvocation {
            arguments,
            environment,
            working_directory: fs::read(self.captures.join("working-directory"))?,
        })
    }
}

impl std::fmt::Debug for FakeNativeProcess {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FakeNativeProcess")
            .field("mode", &self.mode)
            .finish_non_exhaustive()
    }
}

#[derive(Eq, PartialEq)]
pub struct CapturedInvocation {
    arguments: Vec<Vec<u8>>,
    environment: BTreeMap<String, Option<Vec<u8>>>,
    working_directory: Vec<u8>,
}

impl CapturedInvocation {
    pub fn arguments(&self) -> &[Vec<u8>] {
        &self.arguments
    }

    pub fn environment(&self) -> &BTreeMap<String, Option<Vec<u8>>> {
        &self.environment
    }

    pub fn working_directory(&self) -> &[u8] {
        &self.working_directory
    }
}

impl std::fmt::Debug for CapturedInvocation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CapturedInvocation")
            .field("argument_count", &self.arguments.len())
            .field("environment_count", &self.environment.len())
            .field("working_directory_bytes", &self.working_directory.len())
            .finish()
    }
}

fn render_script(
    captures: &Path,
    environment: &[String],
    mode: FakeNativeMode,
    start_barrier: Option<&FileBarrier>,
    pipe_holder_barrier: Option<&FileBarrier>,
    escaped_helper: Option<&Path>,
) -> String {
    let mut script = String::from("#!/bin/sh\nset -eu\numask 077\n");
    script.push_str(&format!(
        "capture={}\nprintf '%s' \"$PWD\" > \"$capture/working-directory\"\n",
        shell_quote(captures)
    ));
    script.push_str(
        "index=0\nfor argument do\n  name=$(printf '%020u' \"$index\")\n  printf '%s' \"$argument\" > \"$capture/argv/$name\"\n  index=$((index + 1))\ndone\nprintf '%s' \"$index\" > \"$capture/argument-count\"\n",
    );
    for name in environment {
        let directory = captures.join("environment").join(name);
        script.push_str(&format!(
            "if [ \"${{{name}+x}}\" = x ]; then\n  printf 1 > {present}\n  printf '%s' \"${{{name}}}\" > {value}\nelse\n  printf 0 > {present}\nfi\n",
            present = shell_quote(&directory.join("present")),
            value = shell_quote(&directory.join("value")),
        ));
    }
    if let Some(barrier) = start_barrier {
        script.push_str(&barrier_script(barrier, ""));
    }
    match mode {
        FakeNativeMode::Exit(code) => script.push_str(&format!("exit {code}\n")),
        FakeNativeMode::Hang => script.push_str("while :; do /bin/sleep 3600; done\n"),
        FakeNativeMode::Flood {
            stdout_bytes,
            stderr_bytes,
        } => {
            script.push_str(&format!(
                "x_chunk='xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx'\ny_chunk='yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy'\nflood_stdout() {{ remaining={stdout_bytes}; while [ \"$remaining\" -ge 256 ]; do printf '%s' \"$x_chunk\"; remaining=$((remaining - 256)); done; while [ \"$remaining\" -gt 0 ]; do printf x; remaining=$((remaining - 1)); done; }}\nflood_stderr() {{ remaining={stderr_bytes}; while [ \"$remaining\" -ge 256 ]; do printf '%s' \"$y_chunk\" >&2; remaining=$((remaining - 256)); done; while [ \"$remaining\" -gt 0 ]; do printf y >&2; remaining=$((remaining - 1)); done; }}\nflood_stdout &\nflood_stderr &\nwait\n"
            ));
        }
        FakeNativeMode::RetainPipes(holder) => {
            let barrier = pipe_holder_barrier.expect("pipe-holder mode has a barrier");
            match holder {
                PipeHolder::Child => {
                    script.push_str("(\n");
                    script.push_str(&barrier_script(barrier, "  "));
                    script.push_str(") &\nexit 0\n");
                }
                PipeHolder::Descendant => {
                    script.push_str("(\n  (\n");
                    script.push_str(&barrier_script(barrier, "    "));
                    script.push_str("  ) &\n  wait\n) &\nexit 0\n");
                }
                PipeHolder::EscapedDescendant => {
                    script.push_str(&format!(
                        "{} {} {} &\nexit 0\n",
                        shell_quote(escaped_helper.expect("escaped mode has helper")),
                        shell_quote(barrier.ready_path()),
                        shell_quote(barrier.release_path()),
                    ));
                }
            }
        }
    }
    if matches!(mode, FakeNativeMode::Flood { .. }) {
        normalize_flood_chunk(&mut script, "x_chunk", 'x');
        normalize_flood_chunk(&mut script, "y_chunk", 'y');
    }
    script
}

fn normalize_flood_chunk(script: &mut String, variable: &str, byte: char) {
    let marker = format!("{variable}='");
    let start = script.find(&marker).expect("flood chunk is rendered") + marker.len();
    let end = start
        + script[start..]
            .find('\'')
            .expect("flood chunk is single quoted");
    script.replace_range(start..end, &byte.to_string().repeat(256));
}

fn barrier_script(barrier: &FileBarrier, indent: &str) -> String {
    format!(
        "{indent}: > {ready}\n{indent}while [ ! -e {release} ]; do :; done\n",
        ready = shell_quote(barrier.ready_path()),
        release = shell_quote(barrier.release_path()),
    )
}

fn shell_quote(path: &Path) -> String {
    let value = path
        .to_str()
        .expect("fixture roots are constructed from UTF-8 names");
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn validate_environment_name(name: &str) -> io::Result<()> {
    let mut bytes = name.bytes();
    let valid = bytes
        .next()
        .is_some_and(|byte| byte == b'_' || byte.is_ascii_alphabetic())
        && bytes.all(|byte| byte == b'_' || byte.is_ascii_alphanumeric());
    if valid {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "environment capture name is not portable",
        ))
    }
}

fn materialize_executable(path: &Path, bytes: &[u8]) -> io::Result<()> {
    fs::write(path, bytes)?;
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(path, permissions)
}

#[cfg(test)]
mod tests {
    use std::{io::Read, process::Stdio, time::Duration};

    use super::*;

    #[test]
    fn capture_preserves_exact_arguments_environment_and_working_directory() {
        const SECRET: &str = "secret-native-capture-canary";
        let working = TempRoot::new("skilltap-native-cwd").unwrap();
        let native = FakeNativeBuilder::new(FakeNativeMode::Exit(0))
            .capture_environment(["CAPTURED", "ABSENT"])
            .unwrap()
            .build()
            .unwrap();
        let status = native
            .command()
            .args(["", "two words", "line\nbreak"])
            .current_dir(&working)
            .env_clear()
            .env("CAPTURED", SECRET)
            .status()
            .unwrap();
        assert!(status.success());

        let capture = native.captured_invocation().unwrap();
        assert_eq!(
            capture.arguments(),
            [b"".as_slice(), b"two words", b"line\nbreak"]
        );
        assert_eq!(
            capture.environment().get("CAPTURED").unwrap().as_deref(),
            Some(SECRET.as_bytes())
        );
        assert_eq!(capture.environment().get("ABSENT"), Some(&None));
        assert_eq!(
            capture.working_directory(),
            working
                .canonicalize()
                .unwrap()
                .as_os_str()
                .as_encoded_bytes()
        );
        assert!(!format!("{capture:?}").contains(SECRET));
        assert!(!format!("{native:?}").contains(SECRET));
    }

    #[test]
    fn nonzero_and_both_stream_flood_modes_are_exact() {
        let nonzero = FakeNativeProcess::new(FakeNativeMode::Exit(23)).unwrap();
        assert_eq!(nonzero.command().status().unwrap().code(), Some(23));

        let flood = FakeNativeProcess::new(FakeNativeMode::Flood {
            stdout_bytes: 4097,
            stderr_bytes: 3073,
        })
        .unwrap();
        let output = flood.command().output().unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, vec![b'x'; 4097]);
        assert_eq!(output.stderr, vec![b'y'; 3073]);
    }

    #[test]
    fn start_and_pipe_holder_barriers_are_deterministic() {
        let gated = FakeNativeBuilder::new(FakeNativeMode::Exit(0))
            .wait_for_release()
            .build()
            .unwrap();
        let mut child = gated.command().spawn().unwrap();
        let barrier = gated.start_barrier().unwrap();
        barrier.wait_until_ready(Duration::from_secs(1)).unwrap();
        assert!(child.try_wait().unwrap().is_none());
        barrier.release().unwrap();
        assert!(child.wait().unwrap().success());

        for holder in [
            PipeHolder::Child,
            PipeHolder::Descendant,
            PipeHolder::EscapedDescendant,
        ] {
            let native = FakeNativeProcess::new(FakeNativeMode::RetainPipes(holder)).unwrap();
            let mut child = native
                .command()
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            let barrier = native.pipe_holder_barrier().unwrap();
            barrier.wait_until_ready(Duration::from_secs(1)).unwrap();
            assert!(child.wait().unwrap().success());
            barrier.release().unwrap();
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            child
                .stdout
                .take()
                .unwrap()
                .read_to_end(&mut stdout)
                .unwrap();
            child
                .stderr
                .take()
                .unwrap()
                .read_to_end(&mut stderr)
                .unwrap();
            assert!(stdout.is_empty());
            assert!(stderr.is_empty());
        }
    }

    #[test]
    fn invalid_environment_names_fail_before_materialization() {
        assert_eq!(
            FakeNativeBuilder::new(FakeNativeMode::Exit(0))
                .capture_environment(["NOT-PORTABLE"])
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidInput
        );
    }
}
