use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
    process::Command,
};

use skilltap_core::{
    domain::{AbsolutePath, HarnessId, RelativeArtifactPath, ResolvedRevision},
    runtime::{
        ConfinedFileSystem, DirectoryIdentity, DirectoryPublishOutcome, DirectoryTreeFileSystem,
        Environment, EnvironmentVariable, ExternalTreeLimits, FileMetadata, FileSystem,
        FileSystemAction, GitRoot, PlatformPaths, RuntimeError, ScopeResolver, SupportedPlatform,
        SystemFileSystem, WorkingDirectory,
    },
    storage::{
        ConfigDocument, ConfigRepository, FileConfigRepository, FileInventoryRepository,
        FileStateRepository, StateRepository,
    },
};
use skilltap_test_support::TempRoot;

use super::*;
use crate::command::{OutputArgs, ScopeArgs, TargetArgs};

struct RecordingFaultFileSystem {
    delegate: SystemFileSystem,
    fail_tree_publish_at: RefCell<BTreeSet<usize>>,
    fail_confined_write_suffix: RefCell<Option<String>>,
    fail_atomic_write_at: Cell<Option<usize>>,
    atomic_write_calls: Cell<usize>,
    tree_publish_attempts: Cell<usize>,
    tree_publish_successes: Cell<usize>,
    bounded_tree_load_calls: Cell<usize>,
    grow_tree_on_bounded_load_at: Cell<Option<usize>>,
}

impl RecordingFaultFileSystem {
    fn new() -> Self {
        Self {
            delegate: SystemFileSystem,
            fail_tree_publish_at: RefCell::new(BTreeSet::new()),
            fail_confined_write_suffix: RefCell::new(None),
            fail_atomic_write_at: Cell::new(None),
            atomic_write_calls: Cell::new(0),
            tree_publish_attempts: Cell::new(0),
            tree_publish_successes: Cell::new(0),
            bounded_tree_load_calls: Cell::new(0),
            grow_tree_on_bounded_load_at: Cell::new(None),
        }
    }

    fn fail_next_tree_publish(&self) {
        self.fail_tree_publish_offsets(&[1]);
    }

    fn fail_tree_publish_offsets(&self, offsets: &[usize]) {
        let current = self.tree_publish_attempts.get();
        self.fail_tree_publish_at
            .borrow_mut()
            .extend(offsets.iter().map(|offset| current + offset));
    }

    fn fail_next_confined_write(&self, suffix: &str) {
        *self.fail_confined_write_suffix.borrow_mut() = Some(suffix.to_owned());
    }

    fn fail_atomic_write_number(&self, number: usize) {
        self.atomic_write_calls.set(0);
        self.fail_atomic_write_at.set(Some(number));
    }

    fn grow_oversized_tree_on_bounded_load(&self, number: usize) {
        self.bounded_tree_load_calls.set(0);
        self.grow_tree_on_bounded_load_at.set(Some(number));
    }

    fn injected(action: FileSystemAction, path: AbsolutePath) -> RuntimeError {
        RuntimeError::FileSystem {
            action,
            path,
            source: io::Error::other("injected managed lifecycle failure"),
        }
    }

    fn confined_path(root: &AbsolutePath, destination: &RelativeArtifactPath) -> AbsolutePath {
        AbsolutePath::new(format!("{}/{}", root.as_str(), destination.as_str())).unwrap()
    }
}

impl FileSystem for RecordingFaultFileSystem {
    fn inspect(&self, path: &AbsolutePath) -> Result<FileMetadata, RuntimeError> {
        self.delegate.inspect(path)
    }

    fn canonicalize(&self, path: &AbsolutePath) -> Result<AbsolutePath, RuntimeError> {
        self.delegate.canonicalize(path)
    }

    fn create_directory_all(&self, path: &AbsolutePath) -> Result<(), RuntimeError> {
        self.delegate.create_directory_all(path)
    }

    fn ensure_private_directory(&self, path: &AbsolutePath) -> Result<(), RuntimeError> {
        self.delegate.ensure_private_directory(path)
    }

    fn ensure_private_file(&self, path: &AbsolutePath) -> Result<(), RuntimeError> {
        self.delegate.ensure_private_file(path)
    }

    fn read(&self, path: &AbsolutePath) -> Result<Vec<u8>, RuntimeError> {
        self.delegate.read(path)
    }

    fn read_regular_no_follow(&self, path: &AbsolutePath) -> Result<Option<Vec<u8>>, RuntimeError> {
        self.delegate.read_regular_no_follow(path)
    }

    fn atomic_write(&self, path: &AbsolutePath, contents: &[u8]) -> Result<(), RuntimeError> {
        let call = self.atomic_write_calls.get() + 1;
        self.atomic_write_calls.set(call);
        if self.fail_atomic_write_at.get() == Some(call) {
            self.fail_atomic_write_at.set(None);
            return Err(Self::injected(FileSystemAction::Write, path.clone()));
        }
        self.delegate.atomic_write(path, contents)
    }

    fn copy_recoverable(
        &self,
        source: &AbsolutePath,
        destination: &AbsolutePath,
    ) -> Result<(), RuntimeError> {
        self.delegate.copy_recoverable(source, destination)
    }

    fn create_relative_symlink(
        &self,
        target: &skilltap_core::runtime::RelativeSymlinkTarget,
        link: &AbsolutePath,
    ) -> Result<(), RuntimeError> {
        self.delegate.create_relative_symlink(target, link)
    }

    fn remove(&self, path: &AbsolutePath) -> Result<(), RuntimeError> {
        self.delegate.remove(path)
    }
}

impl DirectoryTreeFileSystem for RecordingFaultFileSystem {
    fn publish_tree_no_follow(
        &self,
        managed_root: &AbsolutePath,
        destination: &RelativeArtifactPath,
        files: &BTreeMap<RelativeArtifactPath, ArtifactFile>,
    ) -> Result<DirectoryPublishOutcome, RuntimeError> {
        let attempt = self.tree_publish_attempts.get() + 1;
        self.tree_publish_attempts.set(attempt);
        if self.fail_tree_publish_at.borrow_mut().remove(&attempt) {
            return Err(Self::injected(
                FileSystemAction::Write,
                Self::confined_path(managed_root, destination),
            ));
        }
        let result = self
            .delegate
            .publish_tree_no_follow(managed_root, destination, files);
        if result.is_ok() {
            self.tree_publish_successes
                .set(self.tree_publish_successes.get() + 1);
        }
        result
    }

    fn load_tree_no_follow(
        &self,
        managed_root: &AbsolutePath,
        destination: &RelativeArtifactPath,
    ) -> Result<
        (
            DirectoryIdentity,
            BTreeMap<RelativeArtifactPath, ArtifactFile>,
        ),
        RuntimeError,
    > {
        self.delegate.load_tree_no_follow(managed_root, destination)
    }

    fn remove_tree_no_follow(
        &self,
        managed_root: &AbsolutePath,
        destination: &RelativeArtifactPath,
        expected: DirectoryIdentity,
    ) -> Result<DirectoryIdentity, RuntimeError> {
        self.delegate
            .remove_tree_no_follow(managed_root, destination, expected)
    }
}

impl ConfinedFileSystem for RecordingFaultFileSystem {
    fn load_tree_bounded_no_follow(
        &self,
        root: &AbsolutePath,
        destination: &RelativeArtifactPath,
        limits: ExternalTreeLimits,
    ) -> Result<
        (
            DirectoryIdentity,
            BTreeMap<RelativeArtifactPath, ArtifactFile>,
        ),
        RuntimeError,
    > {
        let call = self.bounded_tree_load_calls.get() + 1;
        self.bounded_tree_load_calls.set(call);
        if self.grow_tree_on_bounded_load_at.get() == Some(call) {
            self.grow_tree_on_bounded_load_at.set(None);
            let tree = Self::confined_path(root, destination);
            fs::create_dir_all(tree.as_str()).unwrap();
            let oversized = Path::new(tree.as_str()).join("oversized");
            fs::File::create(oversized)
                .unwrap()
                .set_len(limits.file_bytes() + 1)
                .unwrap();
        }
        self.delegate
            .load_tree_bounded_no_follow(root, destination, limits)
    }

    fn read_regular_bounded_no_follow(
        &self,
        root: &AbsolutePath,
        destination: &RelativeArtifactPath,
        maximum_bytes: u64,
    ) -> Result<Option<Vec<u8>>, RuntimeError> {
        self.delegate
            .read_regular_bounded_no_follow(root, destination, maximum_bytes)
    }

    fn atomic_write_beneath_no_follow(
        &self,
        root: &AbsolutePath,
        destination: &RelativeArtifactPath,
        contents: &[u8],
    ) -> Result<(), RuntimeError> {
        let matches = self
            .fail_confined_write_suffix
            .borrow()
            .as_ref()
            .is_some_and(|suffix| destination.as_str().ends_with(suffix));
        if matches {
            self.fail_confined_write_suffix.borrow_mut().take();
            return Err(Self::injected(
                FileSystemAction::Write,
                Self::confined_path(root, destination),
            ));
        }
        self.delegate
            .atomic_write_beneath_no_follow(root, destination, contents)
    }

    fn remove_file_beneath_no_follow(
        &self,
        root: &AbsolutePath,
        destination: &RelativeArtifactPath,
    ) -> Result<(), RuntimeError> {
        self.delegate
            .remove_file_beneath_no_follow(root, destination)
    }
}

struct FixtureEnvironment {
    home: OsString,
    config: OsString,
    cache: OsString,
}

impl Environment for FixtureEnvironment {
    fn value(&self, variable: EnvironmentVariable) -> Option<OsString> {
        match variable {
            EnvironmentVariable::Home => Some(self.home.clone()),
            EnvironmentVariable::XdgConfigHome => Some(self.config.clone()),
            EnvironmentVariable::XdgCacheHome => Some(self.cache.clone()),
            EnvironmentVariable::CodexHome | EnvironmentVariable::ClaudeConfigDir => None,
            EnvironmentVariable::Path => std::env::var_os("PATH"),
        }
    }
}

fn isolated_platform_paths(root: &TempRoot) -> PlatformPaths {
    let home = root.join("home");
    let config = root.join("config");
    let cache = root.join("cache");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&config).unwrap();
    fs::create_dir_all(&cache).unwrap();
    PlatformPaths::resolve_for(
        SupportedPlatform::current().unwrap(),
        &FixtureEnvironment {
            home: home.into_os_string(),
            config: config.into_os_string(),
            cache: cache.into_os_string(),
        },
    )
    .unwrap()
}

fn enable_codex_only(paths: &PlatformPaths) {
    let filesystem = SystemFileSystem;
    let repository =
        FileConfigRepository::new(&filesystem, paths.skilltap_config().clone()).unwrap();
    let defaults = ConfigDocument::defaults();
    repository
        .replace(
            &defaults
                .with_harness_enabled(&HarnessId::new("codex").unwrap(), true)
                .unwrap()
                .with_harness_enabled(&HarnessId::new("claude").unwrap(), false)
                .unwrap(),
        )
        .unwrap();
}

fn write_managed_marketplace(source: &Path) {
    fs::create_dir_all(source.join(".agents/plugins")).unwrap();
    fs::create_dir_all(source.join("plugins/demo/.codex-plugin")).unwrap();
    fs::create_dir_all(source.join("plugins/demo/skills/demo")).unwrap();
    fs::write(
        source.join(".agents/plugins/marketplace.json"),
        r#"{"name":"team","plugins":[{"name":"demo","source":{"source":"local","path":"./plugins/demo"}}]}"#,
    )
    .unwrap();
    fs::write(
        source.join("plugins/demo/.codex-plugin/plugin.json"),
        r#"{"name":"demo","version":"1.0.0"}"#,
    )
    .unwrap();
    fs::write(
        source.join("plugins/demo/.codex-plugin/mcp.json"),
        r#"{"mcpServers":{"demo-docs":{"command":"demo-mcp","args":["serve"]}}}"#,
    )
    .unwrap();
    fs::write(
        source.join("plugins/demo/skills/demo/SKILL.md"),
        "---\nname: demo\ndescription: fixture\n---\nbody\n",
    )
    .unwrap();
}

fn commit_marketplace(source: &Path, message: &str) -> ResolvedRevision {
    if !source.join(".git").exists() {
        let status = Command::new("git")
            .args(["init", "--initial-branch=main"])
            .arg(source)
            .status()
            .unwrap();
        assert!(status.success());
        for (key, value) in [
            ("user.name", "skilltap test"),
            ("user.email", "skilltap-test@example.invalid"),
        ] {
            let status = Command::new("git")
                .args(["-C"])
                .arg(source)
                .args(["config", key, value])
                .status()
                .unwrap();
            assert!(status.success());
        }
    }
    let add = Command::new("git")
        .args(["-C"])
        .arg(source)
        .args(["add", "."])
        .status()
        .unwrap();
    assert!(add.success());
    let commit = Command::new("git")
        .args(["-C"])
        .arg(source)
        .args(["commit", "--quiet", "-m", message])
        .status()
        .unwrap();
    assert!(commit.success());
    let output = Command::new("git")
        .args(["-C"])
        .arg(source)
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap();
    assert!(output.status.success());
    ResolvedRevision::GitCommit(
        GitCommit::new(String::from_utf8(output.stdout).unwrap().trim()).unwrap(),
    )
}

fn managed_plugin_target(
    paths: &PlatformPaths,
    project: &Path,
    state_filesystem: &dyn FileSystem,
) -> TargetResourceState {
    let state =
        FileStateRepository::new(state_filesystem, paths.skilltap_config().clone()).unwrap();
    let document = match state.load().unwrap() {
        DocumentState::Present(document) => document,
        DocumentState::Missing => panic!("managed lifecycle must persist state"),
    };
    let project = AbsolutePath::new(
        fs::canonicalize(project)
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned(),
    )
    .unwrap();
    let key = ResourceKey::new(
        ResourceId::new("plugin:demo@team").unwrap(),
        Scope::Project(project),
    );
    document
        .resources()
        .get(&key)
        .and_then(|resource| resource.target(&HarnessId::new("codex").unwrap()))
        .cloned()
        .expect("managed Codex plugin target state")
}

fn managed_scope(project: &Path) -> ScopeArgs {
    ScopeArgs {
        project: Some(Some(project.to_path_buf())),
        all_scopes: false,
    }
}

fn codex_target() -> TargetArgs {
    TargetArgs {
        target: Some(skilltap_core::domain::TargetSelection::Only(
            HarnessId::new("codex").unwrap(),
        )),
    }
}

fn execute_managed_lifecycle(
    paths: &PlatformPaths,
    project: &Path,
    state_filesystem: &dyn FileSystem,
    managed_filesystem: &dyn ManagedProjectFileSystem,
    kind: NativeLifecycleKind,
    source: Option<&str>,
    name: Option<&str>,
) -> Outcome {
    let filesystem = SystemFileSystem;
    let config = FileConfigRepository::new(&filesystem, paths.skilltap_config().clone()).unwrap();
    let inventory =
        FileInventoryRepository::new(&filesystem, paths.skilltap_config().clone()).unwrap();
    let state =
        FileStateRepository::new(state_filesystem, paths.skilltap_config().clone()).unwrap();
    let working_directory = FixedWorkingDirectory(absolute(project));
    let git = NoGitRoot;
    let scopes = ScopeResolver::new(&filesystem, &working_directory, &git);
    let registry = skilltap_harnesses::TargetRegistry::canonical();
    StatusApplication {
        config: &config,
        inventory: &inventory,
        state: &state,
        scopes: &scopes,
        working_directory: &working_directory,
        native_observation: NativeObservationMode::Disabled,
        registry: &registry,
        test_platform_paths: Some(paths.clone()),
        test_managed_project_filesystem: Some(managed_filesystem),
    }
    .execute_native_lifecycle(
        "managed lifecycle test",
        kind,
        &managed_scope(project),
        &codex_target(),
        NativeLifecycleValues { source, name },
        false,
    )
}

fn assert_changed(outcome: &Outcome, expected: bool) {
    assert_eq!(
        outcome.summary.get("changed"),
        Some(&OutputValue::Boolean(expected)),
        "outcome: {outcome:?}"
    );
}

#[test]
fn managed_skill_replacement_reports_clean_and_uncertain_restoration() {
    for restore_fails in [false, true] {
        let root = TempRoot::new(if restore_fails {
            "skilltap-managed-skill-restore-failed"
        } else {
            "skilltap-managed-skill-restore-clean"
        })
        .unwrap();
        let filesystem = RecordingFaultFileSystem::new();
        let managed_root = absolute(&root.join("managed"));
        let config_root = absolute(&root.join("config"));
        let destination = RelativeArtifactPath::new("skills/demo").unwrap();
        let destination_path = AbsolutePath::new(format!(
            "{}/{}",
            managed_root.as_str(),
            destination.as_str()
        ))
        .unwrap();
        let previous = ArtifactTree::new([
            ("SKILL.md", b"---\nname: demo\n---\nold".to_vec()),
            ("reference.md", b"prior reference".to_vec()),
        ])
        .unwrap();
        let replacement = ArtifactTree::new([
            ("SKILL.md", b"---\nname: demo\n---\nnew".to_vec()),
            ("reference.md", b"replacement reference".to_vec()),
        ])
        .unwrap();
        let expected_identity = match filesystem
            .publish_tree_no_follow(&managed_root, &destination, previous.files())
            .unwrap()
        {
            DirectoryPublishOutcome::Published(identity) => identity,
            DirectoryPublishOutcome::AlreadyExists => unreachable!(),
        };
        let owner = ResourceKey::new(ResourceId::new("skill:demo").unwrap(), Scope::Global);
        let operation = skilltap_core::lifecycle_operation::faithful_file_operation(
            OperationId::new("managed:skill:replace:demo").unwrap(),
            HarnessId::new("codex").unwrap(),
            owner.clone(),
            OperationAction::SkillInstall,
            destination_path.clone(),
        )
        .unwrap();
        filesystem.fail_tree_publish_offsets(if restore_fails { &[2, 3] } else { &[2] });
        let port = ManagedSkillPort {
            filesystem: &filesystem,
            entries: BTreeMap::from([(
                operation.id().clone(),
                ManagedSkillEntry {
                    root: managed_root.clone(),
                    destination: destination.clone(),
                    tree: replacement,
                    backup_tree: Some(previous.clone()),
                    action: ManagedSkillAction::Replace,
                    expected_identity: Some(expected_identity),
                    owner: Some(owner),
                    config_root: Some(config_root),
                },
            )]),
        };

        let failure = port.apply(&operation).unwrap_err();
        let ExecutionError::Apply { reason } = failure else {
            panic!("expected managed skill apply failure");
        };
        let detail = reason.detail().unwrap().as_str();
        assert!(detail.contains(destination_path.as_str()), "{detail}");

        if restore_fails {
            assert!(detail.contains("could not be proven"), "{detail}");
            assert!(!detail.contains("was restored"), "{detail}");
            assert!(
                filesystem
                    .load_tree_no_follow(&managed_root, &destination)
                    .is_err()
            );
        } else {
            assert!(detail.contains("was restored"), "{detail}");
            let (_, restored_files) = filesystem
                .load_tree_no_follow(&managed_root, &destination)
                .unwrap();
            assert_eq!(restored_files, previous.files().clone());
        }
    }
}

#[test]
fn managed_project_publication_failures_restore_then_retry_once_and_noop() {
    #[derive(Clone, Copy, Debug)]
    enum Boundary {
        Catalog,
        Tree,
        Config,
        State,
    }

    for boundary in [
        Boundary::Catalog,
        Boundary::Tree,
        Boundary::Config,
        Boundary::State,
    ] {
        let root = TempRoot::new("skilltap-managed-publication-failure").unwrap();
        let paths = isolated_platform_paths(&root);
        enable_codex_only(&paths);
        let project = root.join("project");
        let source = root.join("marketplace");
        fs::create_dir_all(&project).unwrap();
        write_managed_marketplace(&source);
        let managed_filesystem = RecordingFaultFileSystem::new();
        let state_filesystem = RecordingFaultFileSystem::new();

        if matches!(boundary, Boundary::Catalog) {
            managed_filesystem.fail_next_confined_write("marketplace.json");
        }
        let add = execute_managed_lifecycle(
            &paths,
            &project,
            &state_filesystem,
            &managed_filesystem,
            NativeLifecycleKind::MarketplaceAdd,
            Some(source.to_str().unwrap()),
            Some("team"),
        );
        if matches!(boundary, Boundary::Catalog) {
            assert_eq!(add.result, ResultClass::AttentionRequired, "{add:?}");
            assert!(!project.join(".agents/plugins/marketplace.json").exists());
            let retry = execute_managed_lifecycle(
                &paths,
                &project,
                &state_filesystem,
                &managed_filesystem,
                NativeLifecycleKind::MarketplaceAdd,
                Some(source.to_str().unwrap()),
                Some("team"),
            );
            assert_eq!(retry.result, ResultClass::Completed, "{retry:?}");
            assert_changed(&retry, true);
            let repeat = execute_managed_lifecycle(
                &paths,
                &project,
                &state_filesystem,
                &managed_filesystem,
                NativeLifecycleKind::MarketplaceAdd,
                Some(source.to_str().unwrap()),
                Some("team"),
            );
            assert_eq!(repeat.result, ResultClass::Completed, "{repeat:?}");
            assert_changed(&repeat, false);
            continue;
        }
        assert_eq!(add.result, ResultClass::Completed, "{add:?}");

        let state_before_failure = matches!(boundary, Boundary::State).then(|| {
            fs::read(Path::new(paths.skilltap_config().as_str()).join("state.json")).unwrap()
        });

        match boundary {
            Boundary::Tree => managed_filesystem.fail_next_tree_publish(),
            Boundary::Config => managed_filesystem.fail_next_confined_write("config.toml"),
            Boundary::State => state_filesystem.fail_atomic_write_number(1),
            Boundary::Catalog => unreachable!(),
        }
        let failed = execute_managed_lifecycle(
            &paths,
            &project,
            &state_filesystem,
            &managed_filesystem,
            NativeLifecycleKind::PluginInstall,
            Some("demo@team"),
            None,
        );
        assert_eq!(failed.result, ResultClass::AttentionRequired, "{failed:?}");
        assert!(
            !project.join(".agents/skills/demo").exists(),
            "boundary={boundary:?} outcome={failed:?}"
        );
        assert!(
            !project.join(".codex/config.toml").exists(),
            "boundary={boundary:?} outcome={failed:?}"
        );
        if let Some(state_before_failure) = state_before_failure {
            assert_eq!(
                fs::read(Path::new(paths.skilltap_config().as_str()).join("state.json")).unwrap(),
                state_before_failure,
                "the failed pending-state publication must preserve the exact prior state document"
            );
        }
        if matches!(boundary, Boundary::Config) {
            assert!(
                format!("{failed:?}").contains("Rollback restored every prior surface"),
                "{failed:?}"
            );
        }

        let retry = execute_managed_lifecycle(
            &paths,
            &project,
            &state_filesystem,
            &managed_filesystem,
            NativeLifecycleKind::PluginInstall,
            Some("demo@team"),
            None,
        );
        assert_eq!(
            retry.result,
            ResultClass::Completed,
            "boundary={boundary:?} outcome={retry:?}"
        );
        assert_changed(&retry, true);
        assert!(project.join(".agents/skills/demo/SKILL.md").is_file());
        assert!(project.join(".codex/config.toml").is_file());

        let published = managed_filesystem.tree_publish_successes.get();
        let repeat = execute_managed_lifecycle(
            &paths,
            &project,
            &state_filesystem,
            &managed_filesystem,
            NativeLifecycleKind::PluginInstall,
            Some("demo@team"),
            None,
        );
        assert_eq!(repeat.result, ResultClass::Completed, "{repeat:?}");
        assert_changed(&repeat, false);
        assert_eq!(managed_filesystem.tree_publish_successes.get(), published);
    }
}

#[test]
fn managed_marketplace_removal_uses_observed_projection_without_source() {
    let root = TempRoot::new("skilltap-managed-marketplace-source-free-remove").unwrap();
    let paths = isolated_platform_paths(&root);
    enable_codex_only(&paths);
    let project = root.join("project");
    let source = root.join("marketplace");
    fs::create_dir_all(&project).unwrap();
    write_managed_marketplace(&source);
    let managed_filesystem = RecordingFaultFileSystem::new();
    let state_filesystem = RecordingFaultFileSystem::new();

    let add = execute_managed_lifecycle(
        &paths,
        &project,
        &state_filesystem,
        &managed_filesystem,
        NativeLifecycleKind::MarketplaceAdd,
        Some(source.to_str().unwrap()),
        Some("team"),
    );
    assert_eq!(add.result, ResultClass::Completed, "{add:?}");
    assert!(project.join(".agents/plugins/marketplace.json").is_file());

    // Removal is grounded in the owned project projection, not the upstream
    // checkout, so an unavailable source must not prevent safe cleanup.
    fs::remove_dir_all(&source).unwrap();
    let remove = execute_managed_lifecycle(
        &paths,
        &project,
        &state_filesystem,
        &managed_filesystem,
        NativeLifecycleKind::MarketplaceRemove,
        None,
        Some("team"),
    );

    assert_eq!(remove.result, ResultClass::Completed, "{remove:?}");
    assert_changed(&remove, true);
    assert!(!project.join(".agents/plugins/marketplace.json").exists());
    assert!(
        remove
            .errors
            .iter()
            .all(|error| error.code != "managed_project_source_missing"),
        "{remove:?}"
    );
}

#[test]
fn managed_project_tree_limits_preserve_planning_and_revalidation_failures() {
    for post_plan_growth in [false, true] {
        let root = TempRoot::new("skilltap-managed-tree-limit-failure").unwrap();
        let paths = isolated_platform_paths(&root);
        enable_codex_only(&paths);
        let project = root.join("project");
        let source = root.join("marketplace");
        fs::create_dir_all(&project).unwrap();
        write_managed_marketplace(&source);
        let managed_filesystem = RecordingFaultFileSystem::new();
        let state_filesystem = RecordingFaultFileSystem::new();
        let add = execute_managed_lifecycle(
            &paths,
            &project,
            &state_filesystem,
            &managed_filesystem,
            NativeLifecycleKind::MarketplaceAdd,
            Some(source.to_str().unwrap()),
            Some("team"),
        );
        assert_eq!(add.result, ResultClass::Completed, "{add:?}");

        if post_plan_growth {
            managed_filesystem.grow_oversized_tree_on_bounded_load(2);
        } else {
            let hostile = project.join(".agents/skills/demo");
            fs::create_dir_all(&hostile).unwrap();
            fs::File::create(hostile.join("oversized"))
                .unwrap()
                .set_len(managed_project_tree_observation_limits().file_bytes() + 1)
                .unwrap();
        }

        let install = execute_managed_lifecycle(
            &paths,
            &project,
            &state_filesystem,
            &managed_filesystem,
            NativeLifecycleKind::PluginInstall,
            Some("demo@team"),
            None,
        );
        assert_eq!(
            install.result,
            ResultClass::AttentionRequired,
            "post_plan_growth={post_plan_growth} outcome={install:?}"
        );
        let expected_code = if post_plan_growth {
            "native_command_failed"
        } else {
            "managed_project_plugin_unreadable"
        };
        assert!(
            install
                .errors
                .iter()
                .any(|error| error.code == expected_code),
            "post_plan_growth={post_plan_growth} outcome={install:?}"
        );
        assert_eq!(managed_filesystem.tree_publish_attempts.get(), 0);
        assert!(project.join(".agents/skills/demo/oversized").is_file());
    }
}

#[test]
fn managed_terminal_journal_failure_recovers_without_duplicate_projection_publication() {
    let root = TempRoot::new("skilltap-managed-terminal-journal").unwrap();
    let paths = isolated_platform_paths(&root);
    enable_codex_only(&paths);
    let project = root.join("project");
    let source = root.join("marketplace");
    fs::create_dir_all(&project).unwrap();
    write_managed_marketplace(&source);
    let first_revision = commit_marketplace(&source, "initial marketplace");
    let locator = format!("file://{}", source.display());
    let managed_filesystem = RecordingFaultFileSystem::new();
    let state_filesystem = RecordingFaultFileSystem::new();

    let add = execute_managed_lifecycle(
        &paths,
        &project,
        &state_filesystem,
        &managed_filesystem,
        NativeLifecycleKind::MarketplaceAdd,
        Some(&locator),
        Some("team"),
    );
    assert_eq!(add.result, ResultClass::Completed, "{add:?}");

    state_filesystem.fail_atomic_write_number(2);
    let failed_install = execute_managed_lifecycle(
        &paths,
        &project,
        &state_filesystem,
        &managed_filesystem,
        NativeLifecycleKind::PluginInstall,
        Some("demo@team"),
        None,
    );
    assert_eq!(
        failed_install.result,
        ResultClass::AttentionRequired,
        "{failed_install:?}"
    );
    assert!(project.join(".agents/skills/demo/SKILL.md").is_file());
    assert!(project.join(".codex/config.toml").is_file());
    let pending_install = managed_plugin_target(&paths, &project, &state_filesystem);
    assert_eq!(pending_install.installed_revision(), None);
    assert!(pending_install.managed_projections().is_empty());
    let attempt = pending_install
        .pending_managed_attempt()
        .expect("terminal failure preserves pending install evidence");
    assert_eq!(attempt.installed_revision(), Some(&first_revision));
    let install_manifest = attempt.managed_projections().to_vec();
    assert_eq!(install_manifest.len(), 2, "{install_manifest:?}");
    assert!(install_manifest.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(matches!(
        &install_manifest[0],
        ManagedProjection::Skill { id, .. } if id.as_str() == "demo"
    ));
    assert!(matches!(
        &install_manifest[1],
        ManagedProjection::Mcp { id, .. } if id.as_str() == "demo-docs"
    ));
    let publications_after_failed_install = managed_filesystem.tree_publish_successes.get();

    let recovered_install = execute_managed_lifecycle(
        &paths,
        &project,
        &state_filesystem,
        &managed_filesystem,
        NativeLifecycleKind::PluginInstall,
        Some("demo@team"),
        None,
    );
    assert_eq!(
        recovered_install.result,
        ResultClass::Completed,
        "{recovered_install:?}"
    );
    assert_changed(&recovered_install, false);
    assert_eq!(
        managed_filesystem.tree_publish_successes.get(),
        publications_after_failed_install
    );
    let installed = managed_plugin_target(&paths, &project, &state_filesystem);
    assert_eq!(installed.installed_revision(), Some(&first_revision));
    assert_eq!(installed.managed_projections(), install_manifest);
    assert!(installed.pending_managed_attempt().is_none());

    let repeat_install = execute_managed_lifecycle(
        &paths,
        &project,
        &state_filesystem,
        &managed_filesystem,
        NativeLifecycleKind::PluginInstall,
        Some("demo@team"),
        None,
    );
    assert_eq!(
        repeat_install.result,
        ResultClass::Completed,
        "{repeat_install:?}"
    );
    assert_changed(&repeat_install, false);
    assert_eq!(
        managed_filesystem.tree_publish_successes.get(),
        publications_after_failed_install
    );

    fs::write(
        source.join("plugins/demo/skills/demo/SKILL.md"),
        "---\nname: demo\ndescription: updated fixture\n---\nupdated body\n",
    )
    .unwrap();
    let second_revision = commit_marketplace(&source, "update marketplace");
    assert_ne!(first_revision, second_revision);
    state_filesystem.fail_atomic_write_number(2);
    let failed_update = execute_managed_lifecycle(
        &paths,
        &project,
        &state_filesystem,
        &managed_filesystem,
        NativeLifecycleKind::PluginUpdate,
        None,
        Some("demo@team"),
    );
    assert_eq!(
        failed_update.result,
        ResultClass::AttentionRequired,
        "{failed_update:?}"
    );
    assert!(
        fs::read_to_string(project.join(".agents/skills/demo/SKILL.md"))
            .unwrap()
            .contains("updated body")
    );
    let pending_update = managed_plugin_target(&paths, &project, &state_filesystem);
    assert_eq!(pending_update.installed_revision(), Some(&first_revision));
    assert!(!pending_update.managed_projections().is_empty());
    let attempt = pending_update
        .pending_managed_attempt()
        .expect("terminal failure preserves pending update evidence");
    assert_eq!(attempt.installed_revision(), Some(&second_revision));
    let update_manifest = attempt.managed_projections().to_vec();
    assert_eq!(update_manifest.len(), 2, "{update_manifest:?}");
    assert!(update_manifest.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(matches!(
        &update_manifest[0],
        ManagedProjection::Skill { id, .. } if id.as_str() == "demo"
    ));
    assert!(matches!(
        &update_manifest[1],
        ManagedProjection::Mcp { id, .. } if id.as_str() == "demo-docs"
    ));
    assert_ne!(update_manifest[0], install_manifest[0]);
    assert_eq!(update_manifest[1], install_manifest[1]);
    let publications_after_failed_update = managed_filesystem.tree_publish_successes.get();
    assert_eq!(
        publications_after_failed_update,
        publications_after_failed_install + 1
    );

    let recovered_update = execute_managed_lifecycle(
        &paths,
        &project,
        &state_filesystem,
        &managed_filesystem,
        NativeLifecycleKind::PluginUpdate,
        None,
        Some("demo@team"),
    );
    assert_eq!(
        recovered_update.result,
        ResultClass::Completed,
        "{recovered_update:?}"
    );
    assert_changed(&recovered_update, false);
    assert_eq!(
        managed_filesystem.tree_publish_successes.get(),
        publications_after_failed_update
    );
    let updated = managed_plugin_target(&paths, &project, &state_filesystem);
    assert_eq!(updated.installed_revision(), Some(&second_revision));
    assert_eq!(updated.managed_projections(), update_manifest);
    assert!(updated.pending_managed_attempt().is_none());

    let repeat_update = execute_managed_lifecycle(
        &paths,
        &project,
        &state_filesystem,
        &managed_filesystem,
        NativeLifecycleKind::PluginUpdate,
        None,
        Some("demo@team"),
    );
    assert_eq!(
        repeat_update.result,
        ResultClass::Completed,
        "{repeat_update:?}"
    );
    assert_changed(&repeat_update, false);
    assert_eq!(
        managed_filesystem.tree_publish_successes.get(),
        publications_after_failed_update
    );
}

fn application_root(root: &TempRoot) -> PathBuf {
    root.join("skilltap")
}

fn absolute(path: &std::path::Path) -> AbsolutePath {
    AbsolutePath::new(path.to_str().unwrap()).unwrap()
}

struct FixedWorkingDirectory(AbsolutePath);

impl WorkingDirectory for FixedWorkingDirectory {
    fn current_directory(&self) -> Result<AbsolutePath, skilltap_core::runtime::RuntimeError> {
        Ok(self.0.clone())
    }
}

struct NoGitRoot;

impl GitRoot for NoGitRoot {
    fn containing_root(
        &self,
        _directory: &AbsolutePath,
    ) -> Result<Option<AbsolutePath>, skilltap_core::runtime::RuntimeError> {
        Ok(None)
    }
}

fn status_args(scope: ScopeArgs) -> StatusArgs {
    StatusArgs {
        target: TargetArgs::default(),
        scope,
        output: OutputArgs::default(),
    }
}

fn execute(root: &std::path::Path, args: &StatusArgs, cwd: AbsolutePath) -> Outcome {
    let filesystem = SystemFileSystem;
    let config = FileConfigRepository::new(&filesystem, absolute(root)).unwrap();
    let inventory = FileInventoryRepository::new(&filesystem, absolute(root)).unwrap();
    let state = FileStateRepository::new(&filesystem, absolute(root)).unwrap();
    let working_directory = FixedWorkingDirectory(cwd);
    let git = NoGitRoot;
    let scopes = ScopeResolver::new(&filesystem, &working_directory, &git);
    let registry = skilltap_harnesses::TargetRegistry::canonical();
    StatusApplication {
        config: &config,
        inventory: &inventory,
        state: &state,
        scopes: &scopes,
        working_directory: &working_directory,
        native_observation: NativeObservationMode::Disabled,
        registry: &registry,
        test_platform_paths: None,
        test_managed_project_filesystem: None,
    }
    .execute(args)
}

#[test]
fn first_use_status_reports_no_enabled_harnesses_and_creates_nothing() {
    let temporary = TempRoot::new("skilltap-cli-application").unwrap();
    let root = application_root(&temporary);
    let cwd = AbsolutePath::new(std::env::current_dir().unwrap().to_str().unwrap()).unwrap();
    assert!(!root.exists());

    let outcome = execute(&root, &status_args(ScopeArgs::default()), cwd);

    assert_eq!(outcome.result, ResultClass::AttentionRequired);
    assert_eq!(outcome.scope, Some(OutputScope::Global));
    assert_eq!(outcome.summary.get("targets"), Some(&0_u64.into()));
    assert!(
        outcome
            .errors
            .iter()
            .any(|error| error.code == "no_enabled_harnesses")
    );
    assert!(
        outcome
            .resources
            .iter()
            .any(|resource| resource.id == "codex" && resource.status == "not_enabled")
    );
    assert!(
        outcome
            .resources
            .iter()
            .any(|resource| resource.id == "claude")
    );
    assert!(!root.exists());
}

fn enable_all_harnesses(root: &std::path::Path) {
    let filesystem = SystemFileSystem;
    let repository = FileConfigRepository::new(&filesystem, absolute(root)).unwrap();
    let defaults = ConfigDocument::defaults();
    let enabled = defaults
        .with_harness_enabled(&HarnessId::new("codex").unwrap(), true)
        .unwrap()
        .with_harness_enabled(&HarnessId::new("claude").unwrap(), true)
        .unwrap();
    repository.replace(&enabled).unwrap();
}

#[test]
fn missing_inventory_makes_all_scopes_global_only() {
    let temporary = TempRoot::new("skilltap-cli-application").unwrap();
    let root = application_root(&temporary);
    enable_all_harnesses(&root);
    let cwd = AbsolutePath::new(std::env::current_dir().unwrap().to_str().unwrap()).unwrap();
    let args = status_args(ScopeArgs {
        project: None,
        all_scopes: true,
    });

    let outcome = execute(&root, &args, cwd);

    assert_eq!(outcome.scope, Some(OutputScope::All));
    assert_eq!(outcome.summary.get("scopes"), Some(&1_u64.into()));
    assert!(
        outcome
            .warnings
            .iter()
            .all(|warning| warning.code != "native_observation_unavailable")
    );
    let value = serde_json::to_value(&outcome).unwrap();
    assert_eq!(value["scope"]["kind"], "all");
}

#[test]
fn relative_project_is_resolved_against_the_working_directory() {
    let temporary = TempRoot::new("skilltap-cli-application").unwrap();
    let root = application_root(&temporary);
    enable_all_harnesses(&root);
    let workspace = TempRoot::new("skilltap-cli-application-workspace").unwrap();
    let current = workspace.join("current");
    let project = workspace.join("project");
    fs::create_dir_all(&current).unwrap();
    fs::create_dir_all(&project).unwrap();
    let args = status_args(ScopeArgs {
        project: Some(Some(PathBuf::from("../project"))),
        all_scopes: false,
    });

    let outcome = execute(
        &root,
        &args,
        AbsolutePath::new(current.to_str().unwrap()).unwrap(),
    );

    assert_eq!(
        outcome.scope,
        Some(OutputScope::Project {
            path: fs::canonicalize(&project)
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned(),
        })
    );
}

#[test]
fn zero_enabled_harnesses_requires_attention_without_panicking() {
    let temporary = TempRoot::new("skilltap-cli-application").unwrap();
    let root = application_root(&temporary);
    let filesystem = SystemFileSystem;
    let repository = FileConfigRepository::new(&filesystem, absolute(&root)).unwrap();
    let defaults = ConfigDocument::defaults();
    let disabled = defaults
        .with_harness_enabled(&HarnessId::new("codex").unwrap(), false)
        .unwrap()
        .with_harness_enabled(&HarnessId::new("claude").unwrap(), false)
        .unwrap();
    repository.replace(&disabled).unwrap();
    let cwd = AbsolutePath::new(std::env::current_dir().unwrap().to_str().unwrap()).unwrap();

    let outcome = execute(&root, &status_args(ScopeArgs::default()), cwd);

    assert_eq!(outcome.result, ResultClass::AttentionRequired);
    assert!(
        outcome
            .errors
            .iter()
            .any(|error| error.code == "no_enabled_harnesses")
    );
}

#[test]
fn malformed_owned_documents_are_classified_independently_without_source_text() {
    for (file, document) in [
        ("config.toml", "config"),
        ("inventory.toml", "inventory"),
        ("state.json", "state"),
    ] {
        let temporary = TempRoot::new("skilltap-cli-application").unwrap();
        let root = application_root(&temporary);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join(file), "SECRET invalid [[[\n").unwrap();
        let cwd = AbsolutePath::new(std::env::current_dir().unwrap().to_str().unwrap()).unwrap();

        let outcome = execute(&root, &status_args(ScopeArgs::default()), cwd);
        let rendered = serde_json::to_string(&outcome).unwrap();

        assert_eq!(outcome.result, ResultClass::Invalid);
        assert!(outcome.errors.iter().any(|error| {
            error.code == "owned_document_malformed"
                && error.context.get("document").map(String::as_str) == Some(document)
        }));
        assert!(!rendered.contains("SECRET"));
        assert!(!rendered.contains("[[["));
    }
}

#[test]
fn explicit_disabled_target_is_invalid_and_actionable() {
    let temporary = TempRoot::new("skilltap-cli-application").unwrap();
    let root = application_root(&temporary);
    let filesystem = SystemFileSystem;
    let repository = FileConfigRepository::new(&filesystem, absolute(&root)).unwrap();
    let defaults = ConfigDocument::defaults();
    let config = defaults
        .with_harness_enabled(&HarnessId::new("codex").unwrap(), true)
        .unwrap()
        .with_harness_enabled(&HarnessId::new("claude").unwrap(), false)
        .unwrap();
    repository.replace(&config).unwrap();
    let mut args = status_args(ScopeArgs::default());
    args.target.target = Some(skilltap_core::domain::TargetSelection::Only(
        HarnessId::new("claude").unwrap(),
    ));
    let cwd = AbsolutePath::new(std::env::current_dir().unwrap().to_str().unwrap()).unwrap();

    let outcome = execute(&root, &args, cwd);

    assert_eq!(outcome.result, ResultClass::Invalid);
    assert!(
        outcome
            .errors
            .iter()
            .any(|error| error.code == "target_not_enabled")
    );
}

#[test]
fn lifecycle_operation_identity_includes_concrete_scope() {
    let target = HarnessId::new("claude").unwrap();
    let resource_id = ResourceId::new("formatter@team").unwrap();
    let global = ResourceKey::new(resource_id.clone(), Scope::Global);
    let project_scope = Scope::Project(AbsolutePath::new("/tmp/skilltap-project").unwrap());
    let project = ResourceKey::new(resource_id, project_scope.clone());

    let global_id = lifecycle_operation_id(
        NativeLifecycleKind::PluginRemove,
        &target,
        &Scope::Global,
        &global,
    );
    let project_id = lifecycle_operation_id(
        NativeLifecycleKind::PluginRemove,
        &target,
        &project_scope,
        &project,
    );

    assert_ne!(global_id, project_id);
    assert_eq!(
        global_id,
        lifecycle_operation_id(
            NativeLifecycleKind::PluginRemove,
            &target,
            &Scope::Global,
            &global,
        )
    );
}

#[test]
fn daemon_noop_normalization_requires_clean_safe_operations() {
    let mut completed = Outcome::new("daemon run", ResultClass::AttentionRequired);
    normalize_daemon_noop_result(&mut completed, 1, 0);
    assert_eq!(completed.result, ResultClass::Completed);

    let mut warning = Outcome::new("daemon run", ResultClass::AttentionRequired)
        .with_warning(Warning::new("update_warning", "review the update"));
    normalize_daemon_noop_result(&mut warning, 1, 0);
    assert_eq!(warning.result, ResultClass::AttentionRequired);

    let mut pending = Outcome::new("daemon run", ResultClass::AttentionRequired);
    normalize_daemon_noop_result(&mut pending, 1, 1);
    assert_eq!(pending.result, ResultClass::AttentionRequired);
}

#[test]
fn detection_diagnostics_are_typed_actionable_and_source_free() {
    let cases = [
        (
            DetectionError::Runtime(
                skilltap_core::runtime::ObservationRuntimeError::ExecutableNotFound,
            ),
            "native_executable_not_found",
            "configure_harness_binary",
        ),
        (
            DetectionError::InvalidVersion,
            "native_version_invalid",
            "inspect_harness_version",
        ),
        (
            DetectionError::NonZeroExit,
            "native_version_command_failed",
            "inspect_harness_version",
        ),
        (
            DetectionError::Runtime(
                skilltap_core::runtime::ObservationRuntimeError::ProcessDeadlineExceeded,
            ),
            "native_detection_bounded",
            "inspect_harness_version",
        ),
    ];

    for (error, warning_code, action_code) in cases {
        let diagnostic = detection_diagnostic(&error, "codex", "/tmp/custom codex");
        assert_eq!(diagnostic.warning.code, warning_code);
        assert_eq!(diagnostic.next_action.code, action_code);
        let rendered = format!("{:?}{:?}", diagnostic.warning, diagnostic.next_action);
        assert!(!rendered.contains("secret-native-output"));
        assert!(!rendered.contains("argv"));
        assert!(!rendered.contains("environment"));
        if matches!(
            error,
            DetectionError::InvalidVersion
                | DetectionError::NonZeroExit
                | DetectionError::Runtime(
                    skilltap_core::runtime::ObservationRuntimeError::ProcessDeadlineExceeded
                )
        ) {
            assert_eq!(
                diagnostic.next_action.command.as_deref(),
                Some("'/tmp/custom codex' --version")
            );
        }
    }
}

struct MemoryStateRepository(RefCell<DocumentState<StateDocument>>);

impl StateRepository for MemoryStateRepository {
    fn load(&self) -> Result<DocumentState<StateDocument>, StorageError> {
        Ok(self.0.borrow().clone())
    }

    fn replace(&self, value: &StateDocument) -> Result<(), StorageError> {
        *self.0.borrow_mut() = DocumentState::Present(value.clone());
        Ok(())
    }
}

fn pending_managed_fixture(
    action: OperationAction,
    existing: Option<TargetResourceState>,
) -> (
    MemoryStateRepository,
    Plan,
    BTreeMap<ResourceKey, ResourceState>,
    ResourceKey,
    Fingerprint,
    Vec<ManagedProjection>,
) {
    let scope = Scope::Project(AbsolutePath::new("/tmp/managed-pending-project").unwrap());
    let key = ResourceKey::new(ResourceId::new("plugin:demo@team").unwrap(), scope);
    let operation_id = OperationId::new("managed:pending:demo").unwrap();
    let operation = skilltap_core::lifecycle_operation::managed_materialization_operation(
        operation_id,
        HarnessId::new("codex").unwrap(),
        key.clone(),
        action,
        [AbsolutePath::new("/tmp/managed-pending-project/.agents/skills/demo").unwrap()],
    )
    .unwrap();
    let plan = Plan::new([operation]).unwrap();
    let fingerprint = fingerprint_contents(b"desired");
    let projections = vec![ManagedProjection::Skill {
        id: skilltap_core::domain::RelativeArtifactPath::new("demo").unwrap(),
        fingerprint: fingerprint_contents(b"skill"),
    }];
    let desired = TargetResourceState::new(
        HarnessId::new("codex").unwrap(),
        Some(NativeId::new("demo@team").unwrap()),
        Provenance::Materialized,
        Ownership::Skilltap,
        None,
        None,
        Some(fingerprint.clone()),
        None,
        None,
        Timestamp::new(10, 0).unwrap(),
        None,
    )
    .unwrap()
    .with_managed_projections(projections.clone());
    let seed = ResourceState::new(key.clone(), [desired]).unwrap();
    let state = StateDocument::new(
        skilltap_core::storage::STATE_SCHEMA_VERSION,
        [],
        existing.map(|target| ResourceState::new(key.clone(), [target]).unwrap()),
        None,
        None,
        None,
    )
    .unwrap();
    (
        MemoryStateRepository(RefCell::new(DocumentState::Present(state))),
        plan,
        BTreeMap::from([(key.clone(), seed)]),
        key,
        fingerprint,
        projections,
    )
}

#[test]
fn managed_pending_writer_and_recovery_use_exact_first_install_and_update_shapes() {
    for existing in [
        None,
        Some(
            TargetResourceState::new(
                HarnessId::new("codex").unwrap(),
                Some(NativeId::new("demo@team").unwrap()),
                Provenance::Materialized,
                Ownership::Skilltap,
                None,
                None,
                Some(fingerprint_contents(b"previous")),
                None,
                None,
                Timestamp::new(9, 0).unwrap(),
                None,
            )
            .unwrap()
            .with_managed_projections([ManagedProjection::Skill {
                id: skilltap_core::domain::RelativeArtifactPath::new("old").unwrap(),
                fingerprint: fingerprint_contents(b"old-skill"),
            }]),
        ),
    ] {
        let action = if existing.is_some() {
            OperationAction::PluginUpdate
        } else {
            OperationAction::PluginInstall
        };
        let (repository, plan, seeds, key, desired_fingerprint, desired_projections) =
            pending_managed_fixture(action, existing.clone());
        let journal = StateExecutionJournal {
            plan: &plan,
            state: &repository,
            seeds,
        };
        let operation_id = plan.iter().next().unwrap().1.id().clone();
        journal
            .record(&OperationResult::new(operation_id.clone(), OperationOutcome::Pending).unwrap())
            .unwrap();
        let document = match repository.load().unwrap() {
            DocumentState::Present(value) => value,
            DocumentState::Missing => unreachable!(),
        };
        let target = document
            .resources()
            .get(&key)
            .unwrap()
            .target(&HarnessId::new("codex").unwrap())
            .unwrap();
        let attempt = target.pending_managed_attempt().expect("pending evidence");
        assert_eq!(attempt.operation_id(), &operation_id);
        assert_eq!(attempt.fingerprint(), &desired_fingerprint);
        assert_eq!(attempt.managed_projections(), desired_projections);
        if let Some(previous) = existing {
            assert_eq!(target.fingerprint(), previous.fingerprint());
            assert_eq!(target.managed_projections(), previous.managed_projections());
        } else {
            assert_eq!(target.fingerprint(), None);
            assert!(target.managed_projections().is_empty());
        }
        assert!(
            validate_managed_project_ownership(
                if action == OperationAction::PluginUpdate {
                    NativeLifecycleKind::PluginUpdate
                } else {
                    NativeLifecycleKind::PluginInstall
                },
                document.resources().get(&key),
                Some(&desired_fingerprint),
                Some(&desired_fingerprint),
                &desired_projections,
                None,
                &operation_id,
            )
            .is_ok()
        );
        journal
            .record(&OperationResult::new(operation_id, OperationOutcome::NoChange).unwrap())
            .unwrap();
        let completed = match repository.load().unwrap() {
            DocumentState::Present(value) => value,
            DocumentState::Missing => unreachable!(),
        };
        let completed_target = completed
            .resources()
            .get(&key)
            .unwrap()
            .target(&HarnessId::new("codex").unwrap())
            .unwrap();
        assert_eq!(completed_target.fingerprint(), Some(&desired_fingerprint));
        assert_eq!(completed_target.managed_projections(), desired_projections);
        assert!(completed_target.pending_managed_attempt().is_none());
    }
}
