use std::{cell::RefCell, fs, io, process::Command, time::Duration};

use skilltap_test_support::TempRoot;

use super::*;
use crate::runtime::{
    CommandOutput, FileMetadata, RelativeSymlinkTarget, SystemCommandRunner, SystemFileSystem,
};

struct TempDirectory(TempRoot);

impl TempDirectory {
    fn new() -> Self {
        Self(TempRoot::new("skilltap-scope-test").unwrap())
    }

    fn absolute(&self, child: &str) -> AbsolutePath {
        AbsolutePath::new(self.0.join(child).to_str().unwrap()).unwrap()
    }

    fn root(&self) -> AbsolutePath {
        AbsolutePath::new(self.0.to_str().unwrap()).unwrap()
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

struct RejectedRunner {
    output: CommandOutput,
}

impl RejectedRunner {
    fn new(stderr: impl Into<Vec<u8>>) -> Self {
        let status = Command::new("git")
            .args(["rev-parse", "--verify", "refs/heads/definitely-missing"])
            .output()
            .unwrap()
            .status;
        assert!(!status.success());
        Self {
            output: CommandOutput::for_test(status, Vec::new(), stderr.into(), Duration::ZERO),
        }
    }
}

impl CommandRunner for RejectedRunner {
    fn run(&self, _request: &CommandRequest) -> Result<CommandOutput, RuntimeError> {
        Ok(self.output.clone())
    }
}

struct InspectOnlyFileSystem {
    inspected: RefCell<Vec<AbsolutePath>>,
    fail: bool,
}

impl InspectOnlyFileSystem {
    fn recording() -> Self {
        Self {
            inspected: RefCell::new(Vec::new()),
            fail: false,
        }
    }

    fn failing() -> Self {
        Self {
            inspected: RefCell::new(Vec::new()),
            fail: true,
        }
    }
}

impl FileSystem for InspectOnlyFileSystem {
    fn inspect(&self, path: &AbsolutePath) -> Result<FileMetadata, RuntimeError> {
        self.inspected.borrow_mut().push(path.clone());
        if self.fail {
            return Err(RuntimeError::FileSystem {
                action: super::super::FileSystemAction::Inspect,
                path: path.clone(),
                source: io::Error::new(io::ErrorKind::PermissionDenied, "secret-source-text"),
            });
        }
        SystemFileSystem.inspect(path)
    }

    fn canonicalize(&self, _path: &AbsolutePath) -> Result<AbsolutePath, RuntimeError> {
        unreachable!("Git metadata probing only inspects marker paths")
    }

    fn create_directory_all(&self, _path: &AbsolutePath) -> Result<(), RuntimeError> {
        unreachable!("Git metadata probing is read-only")
    }

    fn read(&self, _path: &AbsolutePath) -> Result<Vec<u8>, RuntimeError> {
        unreachable!("Git metadata probing does not read contents")
    }

    fn read_regular_no_follow(
        &self,
        _path: &AbsolutePath,
    ) -> Result<Option<Vec<u8>>, RuntimeError> {
        unreachable!("Git metadata probing does not read contents")
    }

    fn atomic_write(&self, _path: &AbsolutePath, _contents: &[u8]) -> Result<(), RuntimeError> {
        unreachable!("Git metadata probing is read-only")
    }

    fn copy_recoverable(
        &self,
        _source: &AbsolutePath,
        _destination: &AbsolutePath,
    ) -> Result<(), RuntimeError> {
        unreachable!("Git metadata probing is read-only")
    }

    fn create_relative_symlink(
        &self,
        _target: &RelativeSymlinkTarget,
        _link: &AbsolutePath,
    ) -> Result<(), RuntimeError> {
        unreachable!("Git metadata probing is read-only")
    }

    fn remove(&self, _path: &AbsolutePath) -> Result<(), RuntimeError> {
        unreachable!("Git metadata probing is read-only")
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
fn corrupt_git_file_and_directory_are_typed_probe_errors() {
    for as_directory in [false, true] {
        let temporary = TempDirectory::new();
        let marker = temporary.0.join(".git");
        if as_directory {
            fs::create_dir(&marker).unwrap();
        } else {
            fs::write(&marker, b"gitdir: missing-target\n").unwrap();
        }
        let runner = SystemCommandRunner;
        let git = CommandGitRoot::new(&runner, NativeId::new("git").unwrap());

        let error = git.containing_root(&temporary.root()).unwrap_err();

        assert!(matches!(
            error,
            RuntimeError::GitRootProbe {
                directory,
                status: Some(_)
            } if directory == temporary.root()
        ));
    }
}

#[test]
fn nested_candidate_detects_only_metadata_on_its_ancestor_chain() {
    let temporary = TempDirectory::new();
    fs::create_dir(temporary.0.join(".git")).unwrap();
    let nested = temporary.0.join("nested/deep");
    fs::create_dir_all(&nested).unwrap();
    let candidate = AbsolutePath::new(nested.to_str().unwrap()).unwrap();
    let runner = RejectedRunner::new(b"ignored-stderr".to_vec());
    let filesystem = InspectOnlyFileSystem::recording();
    let git = CommandGitRoot::with_filesystem(&runner, &filesystem, NativeId::new("git").unwrap());

    assert!(matches!(
        git.containing_root(&candidate),
        Err(RuntimeError::GitRootProbe { .. })
    ));
    let inspected = filesystem.inspected.borrow();
    assert_eq!(
        inspected[0].as_str(),
        format!("{}/.git", candidate.as_str())
    );
    assert_eq!(
        inspected.last().unwrap().as_str(),
        format!("{}/.git", temporary.root().as_str())
    );
    assert_eq!(inspected.len(), 3);
}

#[test]
fn rejected_and_inaccessible_probe_details_are_not_rendered() {
    let secret = "secret-stderr-or-environment-value";
    let temporary = TempDirectory::new();
    fs::write(temporary.0.join(".git"), b"broken").unwrap();
    let runner = RejectedRunner::new(secret.as_bytes().to_vec());
    let git = CommandGitRoot::new(&runner, NativeId::new("git").unwrap());
    let error = git.containing_root(&temporary.root()).unwrap_err();
    assert!(!error.to_string().contains(secret));
    assert!(!format!("{error:?}").contains(secret));

    let filesystem = InspectOnlyFileSystem::failing();
    let git = CommandGitRoot::with_filesystem(&runner, &filesystem, NativeId::new("git").unwrap());
    let error = git.containing_root(&temporary.root()).unwrap_err();
    assert!(matches!(error, RuntimeError::GitRootProbe { .. }));
    assert!(!error.to_string().contains("secret-source-text"));
    assert!(!format!("{error:?}").contains("secret-source-text"));
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
