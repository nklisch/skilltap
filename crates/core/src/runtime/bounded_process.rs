//! Bounded direct native process execution.

use std::{
    io,
    os::{fd::AsRawFd, unix::process::CommandExt},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use super::{
    ExecutableResolver, NativeProcessOutput, NativeProcessRequest, NativeProcessRunner,
    NativeProcessStatus, ObservationRuntimeError, ObservationRuntimeError::*, OutputStream,
    SystemExecutableResolver,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct SystemNativeProcessRunner;

impl NativeProcessRunner for SystemNativeProcessRunner {
    fn run(
        &self,
        request: &NativeProcessRequest,
    ) -> Result<NativeProcessOutput, ObservationRuntimeError> {
        SystemExecutableResolver
            .revalidate(request.executable())
            .map_err(|_| ExecutableChanged)?;

        let mut command = Command::new(request.executable().path().as_str());
        command
            .args(request.arguments())
            .env_clear()
            .envs(request.environment())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(directory) = request.working_directory() {
            command.current_dir(directory.as_str());
        }
        unsafe {
            command.pre_exec(|| {
                if libc::setpgid(0, 0) == -1 {
                    Err(io::Error::last_os_error())
                } else {
                    Ok(())
                }
            });
        }

        let started = Instant::now();
        let mut child = command.spawn().map_err(|_| ProcessSpawnFailed)?;
        let stdout = match child.stdout.take() {
            Some(stdout) => stdout,
            None => {
                let _ = terminate_group(&mut child);
                let _ = child.wait();
                return Err(ProcessIoFailed);
            }
        };
        let stderr = match child.stderr.take() {
            Some(stderr) => stderr,
            None => {
                let _ = terminate_group(&mut child);
                let _ = child.wait();
                return Err(ProcessIoFailed);
            }
        };
        if set_nonblocking(stdout.as_raw_fd()).is_err() {
            let _ = terminate_group(&mut child);
            let _ = child.wait();
            return Err(ProcessIoFailed);
        }
        if set_nonblocking(stderr.as_raw_fd()).is_err() {
            let _ = terminate_group(&mut child);
            let _ = child.wait();
            return Err(ProcessIoFailed);
        }

        let mut output = DrainState::new(request.limits());
        let deadline = started + request.limits().deadline();
        let mut status = None;
        let mut failure = None;
        let mut post_kill_deadline = None;

        loop {
            if let Err(error) = output.drain(stdout.as_raw_fd(), OutputStream::StandardOutput) {
                failure = Some(error);
            }
            if failure.is_none()
                && let Err(error) = output.drain(stderr.as_raw_fd(), OutputStream::StandardError)
            {
                failure = Some(error);
            }

            if status.is_none() {
                status = child.try_wait().map_err(|_| ProcessWaitFailed)?;
            }

            if failure.is_none() && status.is_some() && output.closed() {
                break;
            }

            let now = Instant::now();
            if failure.is_none() && status.is_none() && now >= deadline {
                failure = Some(ProcessDeadlineExceeded);
            }
            if failure.is_some() || (status.is_some() && !output.closed()) {
                if post_kill_deadline.is_none() {
                    if let Err(error) = terminate_group(&mut child) {
                        failure = Some(error);
                    }
                    post_kill_deadline = Some(Instant::now() + Duration::from_millis(100));
                }
                if let Some(end) = post_kill_deadline
                    && Instant::now() >= end
                {
                    if failure.is_none() && !output.closed() {
                        failure = Some(ProcessDrainFailed);
                    }
                    break;
                }
            }
            thread::sleep(Duration::from_millis(1));
        }

        drop(stdout);
        drop(stderr);
        let waited = child.wait().map_err(|_| ProcessWaitFailed)?;
        let status = status.unwrap_or(waited);
        if let Some(error) = failure {
            return Err(error);
        }
        let status = process_status(status);
        NativeProcessOutput::new(
            status,
            output.stdout,
            output.stderr,
            started.elapsed(),
            request.limits(),
        )
    }
}

struct DrainState {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    limits: super::ProcessLimits,
    stdout_closed: bool,
    stderr_closed: bool,
}

impl DrainState {
    fn new(limits: super::ProcessLimits) -> Self {
        Self {
            stdout: Vec::new(),
            stderr: Vec::new(),
            limits,
            stdout_closed: false,
            stderr_closed: false,
        }
    }

    fn closed(&self) -> bool {
        self.stdout_closed && self.stderr_closed
    }

    fn drain(&mut self, fd: i32, stream: OutputStream) -> Result<(), ObservationRuntimeError> {
        let is_closed = match stream {
            OutputStream::StandardOutput => self.stdout_closed,
            OutputStream::StandardError => self.stderr_closed,
            OutputStream::Combined => unreachable!(),
        };
        if is_closed {
            return Ok(());
        }
        let mut chunk = [0_u8; 8192];
        loop {
            let read = unsafe { libc::read(fd, chunk.as_mut_ptr().cast(), chunk.len()) };
            if read == 0 {
                match stream {
                    OutputStream::StandardOutput => self.stdout_closed = true,
                    OutputStream::StandardError => self.stderr_closed = true,
                    OutputStream::Combined => unreachable!(),
                }
                return Ok(());
            }
            if read < 0 {
                let error = io::Error::last_os_error();
                if matches!(
                    error.raw_os_error(),
                    Some(code) if code == libc::EAGAIN || code == libc::EWOULDBLOCK
                ) {
                    return Ok(());
                }
                return Err(ProcessIoFailed);
            }
            let read = usize::try_from(read).map_err(|_| ProcessIoFailed)?;
            let current_len = match stream {
                OutputStream::StandardOutput => self.stdout.len(),
                OutputStream::StandardError => self.stderr.len(),
                OutputStream::Combined => unreachable!(),
            };
            let next = current_len
                .checked_add(read)
                .ok_or(ProcessOutputLimitExceeded { stream })?;
            if u64::try_from(next).map_err(|_| ProcessOutputLimitExceeded { stream })?
                > match stream {
                    OutputStream::StandardOutput => self.limits.stdout_bytes(),
                    OutputStream::StandardError => self.limits.stderr_bytes(),
                    OutputStream::Combined => unreachable!(),
                }
            {
                return Err(ProcessOutputLimitExceeded { stream });
            }
            match stream {
                OutputStream::StandardOutput => self.stdout.extend_from_slice(&chunk[..read]),
                OutputStream::StandardError => self.stderr.extend_from_slice(&chunk[..read]),
                OutputStream::Combined => unreachable!(),
            }
            let combined = self.stdout.len().checked_add(self.stderr.len()).ok_or(
                ProcessOutputLimitExceeded {
                    stream: OutputStream::Combined,
                },
            )?;
            if u64::try_from(combined).map_err(|_| ProcessOutputLimitExceeded {
                stream: OutputStream::Combined,
            })? > self.limits.combined_output_bytes()
            {
                return Err(ProcessOutputLimitExceeded {
                    stream: OutputStream::Combined,
                });
            }
        }
    }
}

fn set_nonblocking(fd: i32) -> io::Result<()> {
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags == -1 {
        return Err(io::Error::last_os_error());
    }
    if unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

fn terminate_group(child: &mut Child) -> Result<(), ObservationRuntimeError> {
    let pid = i32::try_from(child.id()).map_err(|_| ProcessTerminationFailed)?;
    let result = unsafe { libc::kill(-pid, libc::SIGKILL) };
    if result == -1 {
        let error = io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::ESRCH) {
            child.kill().map_err(|_| ProcessTerminationFailed)?;
        }
    }
    Ok(())
}

fn process_status(status: std::process::ExitStatus) -> NativeProcessStatus {
    use std::os::unix::process::ExitStatusExt;
    if let Some(code) = status.code() {
        NativeProcessStatus::Exited {
            code: u8::try_from(code).unwrap_or(u8::MAX),
        }
    } else {
        NativeProcessStatus::Signaled {
            signal: std::num::NonZeroU32::new(
                u32::try_from(status.signal().unwrap_or(libc::SIGKILL)).unwrap_or(1),
            )
            .unwrap_or(std::num::NonZeroU32::new(1).unwrap()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, ffi::OsString, time::Duration};

    use skilltap_test_support::{
        FakeNativeBuilder, FakeNativeMode, FakeNativeProcess, PipeHolder, TempRoot,
    };

    use super::*;
    use crate::runtime::{ExecutableResolver, ProcessLimits};

    fn limits() -> ProcessLimits {
        ProcessLimits::new(500, 4096, 4096, 8192).unwrap()
    }

    fn request(
        fixture: &FakeNativeProcess,
        args: &[&str],
        limits: ProcessLimits,
    ) -> NativeProcessRequest {
        let executable = SystemExecutableResolver
            .resolve(&super::super::ExecutableResolutionRequest::new(
                crate::domain::ConfiguredBinary::absolute(
                    crate::domain::AbsolutePath::new(fixture.executable().to_str().unwrap())
                        .unwrap(),
                ),
                None,
            ))
            .unwrap();
        NativeProcessRequest::new(
            executable,
            args.iter().map(OsString::from),
            BTreeMap::new(),
            None,
            limits,
        )
    }

    #[test]
    fn direct_args_environment_and_nonzero_are_bounded_results() {
        let fixture = FakeNativeProcess::new(FakeNativeMode::Exit(17)).unwrap();
        let output = SystemNativeProcessRunner
            .run(&request(&fixture, &["two words"], limits()))
            .unwrap();
        assert_eq!(output.status(), NativeProcessStatus::Exited { code: 17 });
    }

    #[test]
    fn explicit_environment_and_working_directory_are_forwarded_without_inheritance() {
        let fixture = FakeNativeBuilder::new(FakeNativeMode::Exit(0))
            .capture_environment(["CAPTURED", "ABSENT"])
            .unwrap()
            .build()
            .unwrap();
        let root = TempRoot::new("skilltap-bounded-cwd").unwrap();
        let executable = SystemExecutableResolver
            .resolve(&super::super::ExecutableResolutionRequest::new(
                crate::domain::ConfiguredBinary::absolute(
                    crate::domain::AbsolutePath::new(fixture.executable().to_str().unwrap())
                        .unwrap(),
                ),
                None,
            ))
            .unwrap();
        let request = NativeProcessRequest::new(
            executable,
            [OsString::from("literal argument")],
            BTreeMap::from([(OsString::from("CAPTURED"), OsString::from("value"))]),
            Some(crate::domain::AbsolutePath::new(root.to_str().unwrap()).unwrap()),
            limits(),
        );
        SystemNativeProcessRunner.run(&request).unwrap();
        let capture = fixture.captured_invocation().unwrap();
        assert_eq!(capture.arguments(), [b"literal argument".as_slice()]);
        assert_eq!(
            capture.environment().get("CAPTURED").unwrap().as_deref(),
            Some(b"value".as_slice())
        );
        assert_eq!(capture.environment().get("ABSENT"), Some(&None));
        assert_eq!(
            capture.working_directory(),
            root.to_str().unwrap().as_bytes()
        );
    }

    #[test]
    fn flood_and_hang_fail_without_waiting_forever() {
        let flood = FakeNativeProcess::new(FakeNativeMode::Flood {
            stdout_bytes: 100_000,
            stderr_bytes: 100_000,
        })
        .unwrap();
        assert!(matches!(
            SystemNativeProcessRunner.run(&request(&flood, &[], limits())),
            Err(ProcessOutputLimitExceeded { .. })
        ));
        let hang = FakeNativeProcess::new(FakeNativeMode::Hang).unwrap();
        let started = Instant::now();
        assert_eq!(
            SystemNativeProcessRunner.run(&request(&hang, &[], limits())),
            Err(ProcessDeadlineExceeded)
        );
        assert!(started.elapsed() < Duration::from_secs(3));
    }

    #[test]
    fn escaped_descendants_do_not_hold_parent_completion_forever() {
        let fixture =
            FakeNativeProcess::new(FakeNativeMode::RetainPipes(PipeHolder::EscapedDescendant))
                .unwrap();
        let started = Instant::now();
        let result = SystemNativeProcessRunner.run(&request(&fixture, &[], limits()));
        assert!(matches!(result, Ok(_) | Err(ProcessDrainFailed)));
        assert!(started.elapsed() < Duration::from_secs(2));
    }
}
