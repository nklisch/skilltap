use std::{collections::BTreeSet, ffi::OsString, path::Path};

use crate::domain::{
    AbsolutePath, HarnessId, HarnessSet, NativeId, Scope, TargetSelection, ValidationError,
};

use super::{
    CommandRequest, CommandRunner, FileKind, FileSystem, PathRole, RuntimeError, SystemFileSystem,
};

static SYSTEM_FILE_SYSTEM: SystemFileSystem = SystemFileSystem;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ScopeRequest {
    Global,
    Project {
        path: Option<AbsolutePath>,
    },
    AllScopes {
        recorded_projects: Vec<AbsolutePath>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedScopes(Vec<Scope>);

impl ResolvedScopes {
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &Scope> {
        self.0.iter()
    }

    pub fn into_scopes(self) -> Vec<Scope> {
        self.0
    }
}

pub trait WorkingDirectory {
    fn current_directory(&self) -> Result<AbsolutePath, RuntimeError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SystemWorkingDirectory;

impl WorkingDirectory for SystemWorkingDirectory {
    fn current_directory(&self) -> Result<AbsolutePath, RuntimeError> {
        let path =
            std::env::current_dir().map_err(|source| RuntimeError::WorkingDirectory { source })?;
        let value = path
            .into_os_string()
            .into_string()
            .map_err(|_| RuntimeError::NonUtf8Path {
                role: PathRole::WorkingDirectory,
            })?;
        AbsolutePath::new(value).map_err(|source| RuntimeError::InvalidPath {
            role: PathRole::WorkingDirectory,
            source,
        })
    }
}

pub trait GitRoot {
    fn containing_root(
        &self,
        directory: &AbsolutePath,
    ) -> Result<Option<AbsolutePath>, RuntimeError>;
}

pub struct CommandGitRoot<'a> {
    runner: &'a dyn CommandRunner,
    filesystem: &'a dyn FileSystem,
    executable: NativeId,
}

impl<'a> CommandGitRoot<'a> {
    pub fn new(runner: &'a dyn CommandRunner, executable: NativeId) -> Self {
        Self::with_filesystem(runner, &SYSTEM_FILE_SYSTEM, executable)
    }

    pub const fn with_filesystem(
        runner: &'a dyn CommandRunner,
        filesystem: &'a dyn FileSystem,
        executable: NativeId,
    ) -> Self {
        Self {
            runner,
            filesystem,
            executable,
        }
    }
}

impl GitRoot for CommandGitRoot<'_> {
    fn containing_root(
        &self,
        directory: &AbsolutePath,
    ) -> Result<Option<AbsolutePath>, RuntimeError> {
        let request = CommandRequest::new(
            self.executable.clone(),
            [
                OsString::from("-C"),
                OsString::from(directory.as_str()),
                OsString::from("rev-parse"),
                OsString::from("--show-toplevel"),
            ],
            None,
        );
        let output = self.runner.run(&request)?;
        if !output.status().success() {
            let status = output.status().code();
            if self.contains_git_metadata(directory, status)? {
                return Err(RuntimeError::GitRootProbe {
                    directory: directory.clone(),
                    status,
                });
            }
            return Ok(None);
        }

        let root = std::str::from_utf8(output.stdout()).map_err(|_| RuntimeError::NonUtf8Path {
            role: PathRole::GitRoot,
        })?;
        let root = root.strip_suffix('\n').unwrap_or(root);
        let root = root.strip_suffix('\r').unwrap_or(root);
        AbsolutePath::new(root)
            .map(Some)
            .map_err(|source| RuntimeError::InvalidPath {
                role: PathRole::GitRoot,
                source,
            })
    }
}

impl CommandGitRoot<'_> {
    fn contains_git_metadata(
        &self,
        directory: &AbsolutePath,
        status: Option<i32>,
    ) -> Result<bool, RuntimeError> {
        for ancestor in Path::new(directory.as_str()).ancestors() {
            let marker = ancestor.join(".git");
            let marker = marker
                .to_str()
                .ok_or(RuntimeError::NonUtf8Path {
                    role: PathRole::GitRoot,
                })?
                .to_owned();
            let marker = AbsolutePath::new(marker).map_err(|source| RuntimeError::InvalidPath {
                role: PathRole::GitRoot,
                source,
            })?;
            match self.filesystem.inspect(&marker) {
                Ok(metadata) if metadata.kind() == FileKind::Missing => {}
                Ok(_) => return Ok(true),
                Err(_) => {
                    return Err(RuntimeError::GitRootProbe {
                        directory: directory.clone(),
                        status,
                    });
                }
            }
        }
        Ok(false)
    }
}

pub struct ScopeResolver<'a> {
    filesystem: &'a dyn FileSystem,
    working_directory: &'a dyn WorkingDirectory,
    git_root: &'a dyn GitRoot,
}

impl<'a> ScopeResolver<'a> {
    pub const fn new(
        filesystem: &'a dyn FileSystem,
        working_directory: &'a dyn WorkingDirectory,
        git_root: &'a dyn GitRoot,
    ) -> Self {
        Self {
            filesystem,
            working_directory,
            git_root,
        }
    }

    pub fn resolve(&self, request: &ScopeRequest) -> Result<ResolvedScopes, RuntimeError> {
        match request {
            ScopeRequest::Global => Ok(ResolvedScopes(vec![Scope::Global])),
            ScopeRequest::AllScopes { recorded_projects } => {
                let projects = recorded_projects.iter().cloned().collect::<BTreeSet<_>>();
                let mut scopes = Vec::with_capacity(projects.len() + 1);
                scopes.push(Scope::Global);
                scopes.extend(projects.into_iter().map(Scope::Project));
                Ok(ResolvedScopes(scopes))
            }
            ScopeRequest::Project { path } => {
                let candidate = path
                    .clone()
                    .map_or_else(|| self.working_directory.current_directory(), Ok)?;
                let canonical = self.filesystem.canonicalize(&candidate)?;
                let directory = match self.filesystem.inspect(&canonical)?.kind() {
                    FileKind::Directory => canonical,
                    FileKind::RegularFile => parent(&canonical)?,
                    FileKind::Missing | FileKind::Symlink | FileKind::Other => {
                        return Err(RuntimeError::UnsuitableProjectPath { path: canonical });
                    }
                };
                let root = self
                    .git_root
                    .containing_root(&directory)?
                    .unwrap_or(directory);
                let root = self.filesystem.canonicalize(&root)?;
                if self.filesystem.inspect(&root)?.kind() != FileKind::Directory {
                    return Err(RuntimeError::UnsuitableProjectPath { path: root });
                }
                Ok(ResolvedScopes(vec![Scope::Project(root)]))
            }
        }
    }
}

pub fn resolve_targets(
    selection: Option<&TargetSelection>,
    enabled: impl IntoIterator<Item = HarnessId>,
) -> Result<HarnessSet, ValidationError> {
    let enabled = HarnessSet::new(enabled)?;
    selection.unwrap_or(&TargetSelection::All).resolve(&enabled)
}

fn parent(path: &AbsolutePath) -> Result<AbsolutePath, RuntimeError> {
    let parent = Path::new(path.as_str())
        .parent()
        .ok_or_else(|| RuntimeError::UnsuitableProjectPath { path: path.clone() })?;
    let value = parent
        .to_str()
        .ok_or(RuntimeError::NonUtf8Path {
            role: PathRole::ProjectPath,
        })?
        .to_owned();
    AbsolutePath::new(value).map_err(|source| RuntimeError::InvalidPath {
        role: PathRole::ProjectPath,
        source,
    })
}

#[cfg(test)]
mod tests;
