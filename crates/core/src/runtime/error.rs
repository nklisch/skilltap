use std::{fmt, io, time::SystemTimeError};

use crate::domain::{AbsolutePath, NativeId, ValidationError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeBoundary {
    Environment,
    Path,
    FileSystem,
    Lock,
    Command,
    Clock,
    UnsupportedPlatform,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EnvironmentVariable {
    Home,
    XdgConfigHome,
}

impl EnvironmentVariable {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Home => "HOME",
            Self::XdgConfigHome => "XDG_CONFIG_HOME",
        }
    }
}

impl fmt::Display for EnvironmentVariable {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PathRole {
    Home,
    ConfigHome,
    SkilltapConfig,
    GlobalAgents,
    CodexHome,
    ClaudeHome,
}

impl fmt::Display for PathRole {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Home => "home directory",
            Self::ConfigHome => "configuration home",
            Self::SkilltapConfig => "skilltap configuration directory",
            Self::GlobalAgents => "global AGENTS.md",
            Self::CodexHome => "Codex home",
            Self::ClaudeHome => "Claude home",
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileSystemAction {
    Inspect,
    Canonicalize,
    CreateDirectory,
    Read,
    Write,
    Sync,
    Rename,
    Remove,
    Copy,
    ReadLink,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LockAction {
    Acquire,
    Release,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandAction {
    Spawn,
    Wait,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClockAction {
    Now,
    Elapsed,
}

#[derive(Debug)]
pub enum RuntimeError {
    MissingEnvironment {
        variable: EnvironmentVariable,
    },
    NonUtf8Environment {
        variable: EnvironmentVariable,
    },
    InvalidEnvironmentPath {
        variable: EnvironmentVariable,
        source: ValidationError,
    },
    InvalidPath {
        role: PathRole,
        source: ValidationError,
    },
    NonUtf8Path {
        role: PathRole,
    },
    FileSystem {
        action: FileSystemAction,
        path: AbsolutePath,
        source: io::Error,
    },
    LockContended {
        path: AbsolutePath,
    },
    Lock {
        action: LockAction,
        path: AbsolutePath,
        source: io::Error,
    },
    Command {
        action: CommandAction,
        executable: NativeId,
        source: io::Error,
    },
    Clock {
        action: ClockAction,
        source: SystemTimeError,
    },
    UnsupportedPlatform {
        platform: String,
    },
}

impl RuntimeError {
    pub const fn boundary(&self) -> RuntimeBoundary {
        match self {
            Self::MissingEnvironment { .. }
            | Self::NonUtf8Environment { .. }
            | Self::InvalidEnvironmentPath { .. } => RuntimeBoundary::Environment,
            Self::InvalidPath { .. } | Self::NonUtf8Path { .. } => RuntimeBoundary::Path,
            Self::FileSystem { .. } => RuntimeBoundary::FileSystem,
            Self::LockContended { .. } | Self::Lock { .. } => RuntimeBoundary::Lock,
            Self::Command { .. } => RuntimeBoundary::Command,
            Self::Clock { .. } => RuntimeBoundary::Clock,
            Self::UnsupportedPlatform { .. } => RuntimeBoundary::UnsupportedPlatform,
        }
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingEnvironment { variable } => {
                write!(
                    formatter,
                    "required environment variable `{variable}` is missing"
                )
            }
            Self::NonUtf8Environment { variable } => {
                write!(
                    formatter,
                    "environment variable `{variable}` is not valid UTF-8"
                )
            }
            Self::InvalidEnvironmentPath { variable, source } => {
                write!(
                    formatter,
                    "environment variable `{variable}` is not a valid path: {source}"
                )
            }
            Self::InvalidPath { role, source } => {
                write!(formatter, "could not resolve {role}: {source}")
            }
            Self::NonUtf8Path { role } => write!(formatter, "resolved {role} is not valid UTF-8"),
            Self::FileSystem {
                action,
                path,
                source,
            } => write!(
                formatter,
                "filesystem {action:?} failed for `{path}`: {source}"
            ),
            Self::LockContended { path } => {
                write!(formatter, "configuration lock `{path}` is already held")
            }
            Self::Lock {
                action,
                path,
                source,
            } => write!(formatter, "lock {action:?} failed for `{path}`: {source}"),
            Self::Command {
                action,
                executable,
                source,
            } => write!(
                formatter,
                "command {action:?} failed for executable `{executable}`: {source}"
            ),
            Self::Clock { action, source } => {
                write!(formatter, "clock {action:?} failed: {source}")
            }
            Self::UnsupportedPlatform { platform } => {
                write!(formatter, "unsupported platform `{platform}`")
            }
        }
    }
}

impl std::error::Error for RuntimeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidEnvironmentPath { source, .. } | Self::InvalidPath { source, .. } => {
                Some(source)
            }
            Self::FileSystem { source, .. }
            | Self::Lock { source, .. }
            | Self::Command { source, .. } => Some(source),
            Self::Clock { source, .. } => Some(source),
            Self::MissingEnvironment { .. }
            | Self::NonUtf8Environment { .. }
            | Self::NonUtf8Path { .. }
            | Self::LockContended { .. }
            | Self::UnsupportedPlatform { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boundary_categories_remain_distinct() {
        let path = AbsolutePath::new("/tmp/skilltap").unwrap();
        let executable = NativeId::new("codex").unwrap();
        let clock_error = std::time::UNIX_EPOCH
            .duration_since(std::time::SystemTime::now())
            .unwrap_err();
        let errors = [
            (
                RuntimeError::MissingEnvironment {
                    variable: EnvironmentVariable::Home,
                },
                RuntimeBoundary::Environment,
            ),
            (
                RuntimeError::InvalidPath {
                    role: PathRole::Home,
                    source: ValidationError::PathNotAbsolute,
                },
                RuntimeBoundary::Path,
            ),
            (
                RuntimeError::FileSystem {
                    action: FileSystemAction::Read,
                    path: path.clone(),
                    source: io::Error::new(io::ErrorKind::NotFound, "not found"),
                },
                RuntimeBoundary::FileSystem,
            ),
            (
                RuntimeError::LockContended { path: path.clone() },
                RuntimeBoundary::Lock,
            ),
            (
                RuntimeError::Command {
                    action: CommandAction::Spawn,
                    executable,
                    source: io::Error::new(io::ErrorKind::NotFound, "not found"),
                },
                RuntimeBoundary::Command,
            ),
            (
                RuntimeError::Clock {
                    action: ClockAction::Elapsed,
                    source: clock_error,
                },
                RuntimeBoundary::Clock,
            ),
            (
                RuntimeError::UnsupportedPlatform {
                    platform: "windows".to_owned(),
                },
                RuntimeBoundary::UnsupportedPlatform,
            ),
        ];

        for (error, expected) in errors {
            assert_eq!(error.boundary(), expected);
        }
    }

    #[test]
    fn environment_errors_do_not_render_raw_values() {
        let secret = "do-not-render-this-value";
        let errors = [
            RuntimeError::NonUtf8Environment {
                variable: EnvironmentVariable::Home,
            },
            RuntimeError::InvalidEnvironmentPath {
                variable: EnvironmentVariable::XdgConfigHome,
                source: ValidationError::PathNotAbsolute,
            },
        ];

        for error in errors {
            assert!(!error.to_string().contains(secret));
            assert!(!format!("{error:?}").contains(secret));
        }
    }
}
