use std::{collections::BTreeSet, ffi::OsString, path::Path};

use crate::domain::{
    AbsolutePath, HarnessId, HarnessSet, NativeId, Scope, TargetSelection, ValidationError,
};

use super::{CommandRequest, CommandRunner, FileKind, FileSystem, PathRole, RuntimeError};

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
    executable: NativeId,
}

impl<'a> CommandGitRoot<'a> {
    pub const fn new(runner: &'a dyn CommandRunner, executable: NativeId) -> Self {
        Self { runner, executable }
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
mod tests {
    use std::{
        fs,
        path::PathBuf,
        process::Command,
        sync::atomic::{AtomicU64, Ordering},
    };

    use super::*;
    use crate::runtime::{SystemCommandRunner, SystemFileSystem};

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    struct TempDirectory(PathBuf);

    impl TempDirectory {
        fn new() -> Self {
            let suffix = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "skilltap-scope-test-{}-{suffix}",
                std::process::id()
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn absolute(&self, child: &str) -> AbsolutePath {
            AbsolutePath::new(self.0.join(child).to_str().unwrap()).unwrap()
        }

        fn root(&self) -> AbsolutePath {
            AbsolutePath::new(self.0.to_str().unwrap()).unwrap()
        }
    }

    impl Drop for TempDirectory {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    struct FixedWorkingDirectory(AbsolutePath);

    impl WorkingDirectory for FixedWorkingDirectory {
        fn current_directory(&self) -> Result<AbsolutePath, RuntimeError> {
            Ok(self.0.clone())
        }
    }

    struct NoGit;

    impl GitRoot for NoGit {
        fn containing_root(
            &self,
            _directory: &AbsolutePath,
        ) -> Result<Option<AbsolutePath>, RuntimeError> {
            Ok(None)
        }
    }

    struct PanicWorkingDirectory;

    impl WorkingDirectory for PanicWorkingDirectory {
        fn current_directory(&self) -> Result<AbsolutePath, RuntimeError> {
            panic!("all-scopes must not read the working directory")
        }
    }

    struct PanicGit;

    impl GitRoot for PanicGit {
        fn containing_root(
            &self,
            _directory: &AbsolutePath,
        ) -> Result<Option<AbsolutePath>, RuntimeError> {
            panic!("all-scopes must not search for Git repositories")
        }
    }

    fn project(scopes: ResolvedScopes) -> AbsolutePath {
        match scopes.into_scopes().as_slice() {
            [Scope::Project(path)] => path.clone(),
            scopes => panic!("expected one project scope, got {scopes:?}"),
        }
    }

    #[test]
    fn current_nested_directory_resolves_to_containing_git_root() {
        let temporary = TempDirectory::new();
        let nested = temporary.0.join("nested/deep");
        fs::create_dir_all(&nested).unwrap();
        assert!(
            Command::new("git")
                .args(["init", "--quiet"])
                .current_dir(&temporary.0)
                .status()
                .unwrap()
                .success()
        );

        let working = FixedWorkingDirectory(AbsolutePath::new(nested.to_str().unwrap()).unwrap());
        let runner = SystemCommandRunner;
        let git = CommandGitRoot::new(&runner, NativeId::new("git").unwrap());
        let resolver = ScopeResolver::new(&SystemFileSystem, &working, &git);

        assert_eq!(
            project(
                resolver
                    .resolve(&ScopeRequest::Project { path: None })
                    .unwrap()
            ),
            temporary.root()
        );
    }

    #[test]
    fn explicit_file_resolves_through_parent_to_git_root() {
        let temporary = TempDirectory::new();
        assert!(
            Command::new("git")
                .args(["init", "--quiet"])
                .current_dir(&temporary.0)
                .status()
                .unwrap()
                .success()
        );
        let file = temporary.0.join("nested/file.txt");
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, b"content").unwrap();

        let runner = SystemCommandRunner;
        let git = CommandGitRoot::new(&runner, NativeId::new("git").unwrap());
        let working = FixedWorkingDirectory(temporary.root());
        let resolver = ScopeResolver::new(&SystemFileSystem, &working, &git);

        assert_eq!(
            project(
                resolver
                    .resolve(&ScopeRequest::Project {
                        path: Some(AbsolutePath::new(file.to_str().unwrap()).unwrap()),
                    })
                    .unwrap()
            ),
            temporary.root()
        );
    }

    #[test]
    fn non_git_project_falls_back_to_canonical_directory() {
        let temporary = TempDirectory::new();
        let nested = temporary.0.join("nested");
        fs::create_dir(&nested).unwrap();
        let spelling = temporary.absolute("nested");
        let runner = SystemCommandRunner;
        let git = CommandGitRoot::new(&runner, NativeId::new("git").unwrap());
        let working = FixedWorkingDirectory(temporary.root());
        let resolver = ScopeResolver::new(&SystemFileSystem, &working, &git);

        assert_eq!(
            project(
                resolver
                    .resolve(&ScopeRequest::Project {
                        path: Some(spelling.clone()),
                    })
                    .unwrap()
            ),
            spelling
        );
    }

    #[test]
    fn unsuitable_and_missing_project_inputs_fail_explicitly() {
        let temporary = TempDirectory::new();
        let working = FixedWorkingDirectory(temporary.root());
        let resolver = ScopeResolver::new(&SystemFileSystem, &working, &NoGit);
        let missing = temporary.absolute("missing");
        assert!(
            resolver
                .resolve(&ScopeRequest::Project {
                    path: Some(missing)
                })
                .is_err()
        );

        #[cfg(unix)]
        {
            use std::os::unix::{fs::FileTypeExt, net::UnixListener};

            let socket = temporary.0.join("socket");
            let _listener = UnixListener::bind(&socket).unwrap();
            assert!(
                fs::symlink_metadata(&socket)
                    .unwrap()
                    .file_type()
                    .is_socket()
            );
            let error = resolver
                .resolve(&ScopeRequest::Project {
                    path: Some(AbsolutePath::new(socket.to_str().unwrap()).unwrap()),
                })
                .unwrap_err();
            assert!(matches!(error, RuntimeError::UnsuitableProjectPath { .. }));
        }
    }

    #[test]
    fn global_and_all_scopes_are_deterministic_without_scanning() {
        let resolver = ScopeResolver::new(&SystemFileSystem, &PanicWorkingDirectory, &PanicGit);
        assert_eq!(
            resolver
                .resolve(&ScopeRequest::Global)
                .unwrap()
                .into_scopes(),
            [Scope::Global]
        );

        let scopes = resolver
            .resolve(&ScopeRequest::AllScopes {
                recorded_projects: vec![
                    AbsolutePath::new("/z/project").unwrap(),
                    AbsolutePath::new("/a/project").unwrap(),
                    AbsolutePath::new("/z/project").unwrap(),
                ],
            })
            .unwrap()
            .into_scopes();
        assert_eq!(
            scopes,
            [
                Scope::Global,
                Scope::Project(AbsolutePath::new("/a/project").unwrap()),
                Scope::Project(AbsolutePath::new("/z/project").unwrap()),
            ]
        );
    }

    #[test]
    fn targets_resolve_omitted_all_named_disabled_and_empty() {
        let codex = HarnessId::new("codex").unwrap();
        let claude = HarnessId::new("claude").unwrap();
        let enabled = [codex.clone(), claude.clone()];

        for selection in [None, Some(&TargetSelection::All)] {
            assert_eq!(
                resolve_targets(selection, enabled.clone())
                    .unwrap()
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>(),
                [claude.clone(), codex.clone()]
            );
        }
        assert_eq!(
            resolve_targets(Some(&TargetSelection::Only(codex.clone())), enabled)
                .unwrap()
                .iter()
                .cloned()
                .collect::<Vec<_>>(),
            [codex]
        );
        assert!(matches!(
            resolve_targets(
                Some(&TargetSelection::Only(HarnessId::new("pi").unwrap())),
                [HarnessId::new("codex").unwrap()]
            ),
            Err(ValidationError::HarnessNotEnabled { .. })
        ));
        assert_eq!(
            resolve_targets(None, Vec::<HarnessId>::new()).unwrap_err(),
            ValidationError::EmptyHarnessSet
        );
    }

    #[test]
    fn request_shape_is_mutually_exclusive() {
        let requests = [
            ScopeRequest::Global,
            ScopeRequest::Project { path: None },
            ScopeRequest::AllScopes {
                recorded_projects: Vec::new(),
            },
        ];
        assert_eq!(requests.len(), 3);
    }
}
