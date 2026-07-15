use std::{collections::BTreeSet, fmt, io, time::SystemTimeError};

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
    XdgCacheHome,
    CodexHome,
    ClaudeConfigDir,
    KimiShareDir,
    VibeHome,
    KiroHome,
    PiPackageDir,
    Path,
}

impl EnvironmentVariable {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Home => "HOME",
            Self::XdgConfigHome => "XDG_CONFIG_HOME",
            Self::XdgCacheHome => "XDG_CACHE_HOME",
            Self::CodexHome => "CODEX_HOME",
            Self::ClaudeConfigDir => "CLAUDE_CONFIG_DIR",
            Self::KimiShareDir => "KIMI_SHARE_DIR",
            Self::VibeHome => "VIBE_HOME",
            Self::KiroHome => "KIRO_HOME",
            Self::PiPackageDir => "PI_PACKAGE_DIR",
            Self::Path => "PATH",
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
    CanonicalPath,
    WorkingDirectory,
    ProjectPath,
    GitRoot,
    Home,
    ConfigHome,
    CacheHome,
    SkilltapConfig,
    GlobalAgents,
    CodexHome,
    ClaudeHome,
    KimiShareDir,
    VibeHome,
    KiroHome,
    PiHome,
    PiPackageDir,
}

impl fmt::Display for PathRole {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::CanonicalPath => "canonical path",
            Self::WorkingDirectory => "working directory",
            Self::ProjectPath => "project path",
            Self::GitRoot => "Git root",
            Self::Home => "home directory",
            Self::ConfigHome => "configuration home",
            Self::CacheHome => "cache home",
            Self::SkilltapConfig => "skilltap configuration directory",
            Self::GlobalAgents => "global AGENTS.md",
            Self::CodexHome => "Codex home",
            Self::ClaudeHome => "Claude home",
            Self::KimiShareDir => "Kimi share directory",
            Self::VibeHome => "Vibe home",
            Self::KiroHome => "Kiro home",
            Self::PiHome => "Pi home",
            Self::PiPackageDir => "Pi package directory",
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

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PublicationResidualRole {
    Temporary,
    Destination,
}

impl fmt::Display for PublicationResidualRole {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Temporary => "temporary",
            Self::Destination => "destination",
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PublicationResidual {
    role: PublicationResidualRole,
    path: AbsolutePath,
}

impl PublicationResidual {
    pub const fn new(role: PublicationResidualRole, path: AbsolutePath) -> Self {
        Self { role, path }
    }

    pub const fn role(&self) -> PublicationResidualRole {
        self.role
    }

    pub const fn path(&self) -> &AbsolutePath {
        &self.path
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DirectorySyncState {
    NotRequired,
    Synced,
    Uncertain,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DirectoryPathState {
    Present,
    Removed,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DirectoryContentState {
    Intact,
    Partial,
    Empty,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DirectoryIdentity {
    device: u64,
    inode: u64,
}

impl DirectoryIdentity {
    pub const fn new(device: u64, inode: u64) -> Self {
        Self { device, inode }
    }

    pub const fn device(self) -> u64 {
        self.device
    }

    pub const fn inode(self) -> u64 {
        self.inode
    }
}

impl fmt::Display for DirectorySyncState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NotRequired => "not required",
            Self::Synced => "synced",
            Self::Uncertain => "uncertain",
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicationResiduals {
    paths: BTreeSet<PublicationResidual>,
    directory_sync: DirectorySyncState,
}

impl PublicationResiduals {
    pub(super) fn new(
        paths: impl IntoIterator<Item = PublicationResidual>,
        directory_sync: DirectorySyncState,
    ) -> Self {
        Self {
            paths: paths.into_iter().collect(),
            directory_sync,
        }
    }

    pub const fn paths(&self) -> &BTreeSet<PublicationResidual> {
        &self.paths
    }

    pub const fn directory_sync(&self) -> DirectorySyncState {
        self.directory_sync
    }
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
    WorkingDirectory {
        source: io::Error,
    },
    UnsuitableProjectPath {
        path: AbsolutePath,
    },
    GitRootProbe {
        directory: AbsolutePath,
        status: Option<i32>,
    },
    FileSystem {
        action: FileSystemAction,
        path: AbsolutePath,
        source: io::Error,
    },
    UnsafeSymlink {
        action: FileSystemAction,
        path: AbsolutePath,
    },
    FileIdentityChanged {
        action: FileSystemAction,
        path: AbsolutePath,
    },
    PartialPublication {
        path: AbsolutePath,
        residuals: PublicationResiduals,
        source: io::Error,
        cleanup: io::Error,
    },
    PartialDirectoryPublication {
        path: AbsolutePath,
        identity: Option<DirectoryIdentity>,
        presence: DirectoryPathState,
        parent_sync: DirectorySyncState,
        source: io::Error,
        cleanup: io::Error,
    },
    PartialDirectoryRemoval {
        path: AbsolutePath,
        expected: DirectoryIdentity,
        observed: Option<DirectoryIdentity>,
        presence: DirectoryPathState,
        content: DirectoryContentState,
        parent_sync: DirectorySyncState,
        source: io::Error,
    },
    LockContended {
        path: AbsolutePath,
    },
    LockIdentityChanged {
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
            Self::InvalidPath { .. }
            | Self::NonUtf8Path { .. }
            | Self::WorkingDirectory { .. }
            | Self::UnsuitableProjectPath { .. }
            | Self::GitRootProbe { .. } => RuntimeBoundary::Path,
            Self::FileSystem { .. }
            | Self::UnsafeSymlink { .. }
            | Self::FileIdentityChanged { .. }
            | Self::PartialPublication { .. }
            | Self::PartialDirectoryPublication { .. }
            | Self::PartialDirectoryRemoval { .. } => RuntimeBoundary::FileSystem,
            Self::LockContended { .. } | Self::LockIdentityChanged { .. } | Self::Lock { .. } => {
                RuntimeBoundary::Lock
            }
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
            Self::WorkingDirectory { source } => {
                write!(
                    formatter,
                    "could not read the current working directory: {source}"
                )
            }
            Self::UnsuitableProjectPath { path } => {
                write!(
                    formatter,
                    "project path `{path}` is not a file or directory"
                )
            }
            Self::GitRootProbe { directory, status } => match status {
                Some(status) => write!(
                    formatter,
                    "Git root probe failed for `{directory}` with status {status} despite containing Git metadata"
                ),
                None => write!(
                    formatter,
                    "Git root probe failed for `{directory}` without an exit code despite containing Git metadata"
                ),
            },
            Self::FileSystem {
                action,
                path,
                source,
            } => write!(
                formatter,
                "filesystem {action:?} failed for `{path}`: {source}"
            ),
            Self::UnsafeSymlink { action, path } => write!(
                formatter,
                "filesystem {action:?} refused symlink path `{path}`"
            ),
            Self::FileIdentityChanged { action, path } => write!(
                formatter,
                "filesystem {action:?} refused changed path identity `{path}`"
            ),
            Self::PartialPublication {
                path,
                residuals,
                source,
                cleanup,
            } => {
                let paths = if residuals.paths().is_empty() {
                    "none".to_owned()
                } else {
                    residuals
                        .paths()
                        .iter()
                        .map(|residual| format!("{} `{}`", residual.role(), residual.path()))
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                write!(
                    formatter,
                    "backup publication for `{path}` failed ({source}); residual paths: {paths}; directory sync: {}; cleanup failed ({cleanup})",
                    residuals.directory_sync()
                )
            }
            Self::PartialDirectoryPublication {
                path,
                identity,
                presence,
                parent_sync,
                source,
                cleanup,
            } => {
                let identity = identity.map_or_else(
                    || "unknown".to_owned(),
                    |value| format!("{}:{}", value.device(), value.inode()),
                );
                write!(
                    formatter,
                    "directory publication for `{path}` failed ({source}); destination: {presence:?}; identity: {identity}; parent sync: {parent_sync}; cleanup failed ({cleanup})"
                )
            }
            Self::PartialDirectoryRemoval {
                path,
                expected,
                observed,
                presence,
                content,
                parent_sync,
                source,
            } => {
                let observed = observed.map_or_else(
                    || "unknown".to_owned(),
                    |value| format!("{}:{}", value.device(), value.inode()),
                );
                write!(
                    formatter,
                    "directory removal for `{path}` failed ({source}); expected identity: {}:{}; observed identity: {observed}; destination: {presence:?}; content: {content:?}; parent sync: {parent_sync}",
                    expected.device(),
                    expected.inode(),
                )
            }
            Self::LockContended { path } => {
                write!(formatter, "configuration lock `{path}` is already held")
            }
            Self::LockIdentityChanged { path } => {
                write!(
                    formatter,
                    "configuration lock path `{path}` changed during acquisition"
                )
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
            | Self::PartialPublication { source, .. }
            | Self::PartialDirectoryPublication { source, .. }
            | Self::PartialDirectoryRemoval { source, .. }
            | Self::Lock { source, .. }
            | Self::Command { source, .. }
            | Self::WorkingDirectory { source } => Some(source),
            Self::Clock { source, .. } => Some(source),
            Self::MissingEnvironment { .. }
            | Self::NonUtf8Environment { .. }
            | Self::NonUtf8Path { .. }
            | Self::UnsuitableProjectPath { .. }
            | Self::GitRootProbe { .. }
            | Self::UnsafeSymlink { .. }
            | Self::FileIdentityChanged { .. }
            | Self::LockContended { .. }
            | Self::LockIdentityChanged { .. }
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
