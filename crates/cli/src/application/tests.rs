use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

use skilltap_core::{
    domain::{
        AbsolutePath, CapabilityId, CapabilitySupport, ConditionalProfileObservation, HarnessId,
        NativeVersion, RelativeArtifactPath, ResolvedRevision, Scope,
    },
    runtime::{
        ConfinedFileSystem, DirectoryIdentity, DirectoryPublishOutcome, DirectoryTreeFileSystem,
        Environment, EnvironmentVariable, ExternalTreeLimits, FileMetadata, FileSystem,
        FileSystemAction, GitRoot, PlatformPaths, RuntimeError, ScopeResolver, SupportedPlatform,
        SystemFileSystem, WorkingDirectory,
    },
    storage::{
        ConfigDocument, ConfigRepository, FileConfigRepository, FileInventoryRepository,
        FileStateRepository, HarnessBinary, StateRepository,
    },
};
use skilltap_test_support::{
    ConditionalFixtureCase, ConditionalTargetFixture, FakeHarnessProfile, FakeNativeMode,
    FakeNativeProcess, IsolatedMachine, ManagedAcceptanceCheck, ManagedAcceptanceEvidence,
    ManagedAcceptanceScenario, ManagedProjectionProfile, TempRoot, managed_acceptance_matrix,
    snapshot_tree,
};

use super::*;
use crate::command::{OutputArgs, ScopeArgs, TargetArgs};
use skilltap_harnesses::TargetRegistry;

struct RecordingFaultFileSystem {
    delegate: SystemFileSystem,
    fail_tree_publish_at: RefCell<BTreeSet<usize>>,
    fail_confined_write_suffix: RefCell<Option<String>>,
    fail_atomic_write_at: Cell<Option<usize>>,
    atomic_write_calls: Cell<usize>,
    confined_write_calls: Cell<usize>,
    post_write_read_calls: Cell<usize>,
    fail_next_post_write_read: Cell<bool>,
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
            confined_write_calls: Cell::new(0),
            post_write_read_calls: Cell::new(0),
            fail_next_post_write_read: Cell::new(false),
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

    fn fail_next_post_write_read(&self) {
        self.fail_next_post_write_read.set(true);
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
        if self.confined_write_calls.get() > 0 {
            self.post_write_read_calls
                .set(self.post_write_read_calls.get() + 1);
            if self.fail_next_post_write_read.replace(false) {
                return Err(Self::injected(
                    FileSystemAction::Read,
                    Self::confined_path(root, destination),
                ));
            }
        }
        self.delegate
            .read_regular_bounded_no_follow(root, destination, maximum_bytes)
    }

    fn atomic_write_beneath_no_follow(
        &self,
        root: &AbsolutePath,
        destination: &RelativeArtifactPath,
        contents: &[u8],
    ) -> Result<(), RuntimeError> {
        self.confined_write_calls
            .set(self.confined_write_calls.get() + 1);
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
            EnvironmentVariable::CodexHome
            | EnvironmentVariable::ClaudeConfigDir
            | EnvironmentVariable::KimiShareDir
            | EnvironmentVariable::VibeHome
            | EnvironmentVariable::KiroHome
            | EnvironmentVariable::PiPackageDir => None,
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
    fs::create_dir_all(source.join("plugins/demo/skills/demo/scripts")).unwrap();
    fs::create_dir_all(source.join("plugins/demo/skills/demo/references")).unwrap();
    fs::write(
        source.join("plugins/demo/skills/demo/scripts/run.sh"),
        "#!/bin/sh\nexit 0\n",
    )
    .unwrap();
    fs::write(
        source.join("plugins/demo/skills/demo/references/usage.md"),
        "# Usage\n",
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

fn managed_state_document(
    paths: &PlatformPaths,
    state_filesystem: &dyn FileSystem,
) -> StateDocument {
    let state =
        FileStateRepository::new(state_filesystem, paths.skilltap_config().clone()).unwrap();
    match state.load().unwrap() {
        DocumentState::Present(document) => document,
        DocumentState::Missing => panic!("managed lifecycle must persist state"),
    }
}

fn managed_target_state(
    paths: &PlatformPaths,
    project: &Path,
    state_filesystem: &dyn FileSystem,
    target: &HarnessId,
    resource: &str,
) -> TargetResourceState {
    let document = managed_state_document(paths, state_filesystem);
    let key = ResourceKey::new(
        ResourceId::new(resource).unwrap(),
        Scope::Project(absolute(&fs::canonicalize(project).unwrap())),
    );
    document
        .resources()
        .get(&key)
        .and_then(|resource| resource.target(target))
        .cloned()
        .unwrap_or_else(|| panic!("managed target state for {target}:{resource}"))
}

fn managed_scope(project: &Path) -> ScopeArgs {
    ScopeArgs {
        project: Some(Some(project.to_path_buf())),
        all_scopes: false,
    }
}

fn global_scope() -> ScopeArgs {
    ScopeArgs {
        project: None,
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

struct FakeManagedAdapter;
struct FakeManagedProjection;

const FAKE_MANAGED_PROFILE: ManagedProjectionProfile = ManagedProjectionProfile::new(
    "fake-managed",
    &[".fake/managed-marketplace"],
    Some(".fake/config.toml"),
    ".fake/skills",
);
static FAKE_MANAGED_ADAPTER: FakeManagedAdapter = FakeManagedAdapter;
static FAKE_MANAGED_PROJECTION: FakeManagedProjection = FakeManagedProjection;
static FAKE_APPLY_PLANS: AtomicUsize = AtomicUsize::new(0);
static FAKE_REMOVE_PLANS: AtomicUsize = AtomicUsize::new(0);

fn fake_managed_capabilities() -> skilltap_core::domain::CapabilitySet {
    skilltap_core::domain::CapabilitySet::new([
        (
            skilltap_core::domain::CapabilityId::new("managed.projection").unwrap(),
            skilltap_core::domain::CapabilitySupport::Supported,
        ),
        (
            skilltap_core::domain::CapabilityId::new("component.skill").unwrap(),
            skilltap_core::domain::CapabilitySupport::Supported,
        ),
        (
            skilltap_core::domain::CapabilityId::new("component.mcp").unwrap(),
            skilltap_core::domain::CapabilitySupport::Supported,
        ),
    ])
}

impl skilltap_harnesses::HarnessAdapter for FakeManagedAdapter {
    fn identity(&self) -> skilltap_harnesses::TargetIdentity {
        skilltap_harnesses::TargetIdentity {
            id: fake_target_id(),
            display_name: "Fake Managed",
            default_binary: "fake-managed",
            distribution_surface: skilltap_harnesses::DistributionSurface::Managed,
        }
    }

    fn version_arguments(&self) -> Vec<OsString> {
        vec![OsString::from("--version")]
    }

    fn decode_version(
        &self,
        stdout: &[u8],
    ) -> Result<skilltap_core::domain::NativeVersion, DetectionError> {
        let text = std::str::from_utf8(stdout)
            .map_err(|_| DetectionError::InvalidVersion)?
            .trim();
        let version = text
            .strip_prefix("codex-cli ")
            .ok_or(DetectionError::InvalidVersion)?;
        skilltap_core::domain::NativeVersion::new(version)
            .map_err(|_| DetectionError::InvalidVersion)
    }

    fn select_profile(
        &self,
        version: &skilltap_core::domain::NativeVersion,
    ) -> skilltap_core::domain::CapabilityProfileSelection {
        let capabilities = skilltap_core::domain::ScopedCapabilitySets::new(
            fake_managed_capabilities(),
            fake_managed_capabilities(),
        );
        if version.as_str() == "0.144.1" {
            skilltap_core::domain::CapabilityProfileSelection::verified(
                skilltap_core::domain::CapabilityProfileId::new("fake-managed")
                    .expect("test profile id is valid"),
                capabilities,
            )
        } else {
            skilltap_core::domain::CapabilityProfileSelection::unknown_version(capabilities)
        }
    }

    fn observe(
        &self,
        _paths: &PlatformPaths,
        _scope: &Scope,
        _limits: ExternalTreeLimits,
    ) -> Result<skilltap_harnesses::AdapterObservationPaths, skilltap_harnesses::ObservationPathError>
    {
        Ok(skilltap_harnesses::AdapterObservationPaths {
            canonical: Vec::new(),
            project_entry_count: None,
            surface_labels: Vec::new(),
        })
    }

    fn managed_projection(&self) -> Option<&dyn skilltap_harnesses::ManagedProjectionPort> {
        Some(&FAKE_MANAGED_PROJECTION)
    }
}

impl skilltap_harnesses::ManagedProjectionPort for FakeManagedProjection {
    fn plan(
        &self,
        context: &ManagedProjectionContext<'_>,
    ) -> Result<
        skilltap_core::managed_projection::ManagedProjectionPlan,
        skilltap_core::managed_projection::ManagedProjectionError,
    > {
        match &context.input {
            ManagedProjectionInput::Apply { checkout } => {
                FAKE_APPLY_PLANS.fetch_add(1, Ordering::SeqCst);
                assert_eq!(
                    checkout.root().as_str(),
                    checkout.source().locator().as_str()
                );
            }
            ManagedProjectionInput::Remove => {
                FAKE_REMOVE_PLANS.fetch_add(1, Ordering::SeqCst);
            }
        }
        match context.resource_kind {
            ResourceKind::Marketplace => fake_marketplace_plan(context),
            ResourceKind::Plugin => fake_plugin_plan(context),
            _ => Err(
                skilltap_core::managed_projection::ManagedProjectionError::UnsupportedResourceKind,
            ),
        }
    }
}

fn context_root<'scope>(context: &ManagedProjectionContext<'scope>) -> &'scope AbsolutePath {
    match context.scope {
        Scope::Project(project) => project,
        Scope::Global => context.paths.home(),
    }
}

fn fake_marker(
    context: &ManagedProjectionContext<'_>,
    root: &AbsolutePath,
    name: &str,
) -> Result<bool, skilltap_core::managed_projection::ManagedProjectionError> {
    let destination = RelativeArtifactPath::new(name).unwrap();
    context
        .filesystem
        .read_regular_bounded_no_follow(root, &destination, 64)
        .map(|value| value.is_some())
        .map_err(|_| fake_projection_error())
}

fn fake_marketplace_plan(
    context: &ManagedProjectionContext<'_>,
) -> Result<
    skilltap_core::managed_projection::ManagedProjectionPlan,
    skilltap_core::managed_projection::ManagedProjectionError,
> {
    let destination =
        RelativeArtifactPath::new(FAKE_MANAGED_PROFILE.catalog_destinations()[0]).unwrap();
    let current = context
        .filesystem
        .read_regular_bounded_no_follow(context_root(context), &destination, 4096)
        .map_err(|_| fake_projection_error())?;
    let (desired, manifest) = match &context.input {
        ManagedProjectionInput::Apply { checkout } => {
            let desired = checkout.source().locator().as_str().as_bytes().to_vec();
            (Some(desired), Vec::new())
        }
        ManagedProjectionInput::Remove => (None, Vec::new()),
    };
    Ok(skilltap_core::managed_projection::ManagedProjectionPlan {
        files: vec![ManagedFileWrite {
            root: context_root(context).clone(),
            destination,
            expected: current.clone(),
            desired: desired.clone(),
        }],
        manifest,
        current_fingerprint: current.as_deref().map(fingerprint_contents),
        desired_fingerprint: desired.as_deref().map(fingerprint_contents),
        ..skilltap_core::managed_projection::ManagedProjectionPlan::default()
    })
}

fn fake_plugin_plan(
    context: &ManagedProjectionContext<'_>,
) -> Result<
    skilltap_core::managed_projection::ManagedProjectionPlan,
    skilltap_core::managed_projection::ManagedProjectionError,
> {
    let skill_root = AbsolutePath::new(format!(
        "{}/{}",
        context_root(context).as_str(),
        FAKE_MANAGED_PROFILE.skill_destination()
    ))
    .unwrap();
    let skill_destination = RelativeArtifactPath::new("demo").unwrap();
    let current_tree = match context.filesystem.load_tree_bounded_no_follow(
        &skill_root,
        &skill_destination,
        managed_tree_observation_limits(),
    ) {
        Ok((identity, files)) => Some((
            identity,
            ArtifactTree::new(
                files
                    .into_iter()
                    .map(|(path, file)| (path.as_str().to_owned(), file)),
            )
            .map_err(|_| fake_projection_error())?,
        )),
        Err(RuntimeError::FileSystem { source, .. })
            if source.kind() == io::ErrorKind::NotFound =>
        {
            None
        }
        Err(_) => return Err(fake_projection_error()),
    };
    let mcp_destination =
        RelativeArtifactPath::new(FAKE_MANAGED_PROFILE.mcp_destination().unwrap()).unwrap();
    let current_mcp = context
        .filesystem
        .read_regular_bounded_no_follow(context_root(context), &mcp_destination, 4096)
        .map_err(|_| fake_projection_error())?;

    let (desired_tree, desired_mcp, mut manifest) = match &context.input {
        ManagedProjectionInput::Apply { checkout } => {
            if fake_marker(context, checkout.root(), "required-unsupported")? {
                return Err(
                    skilltap_core::managed_projection::ManagedProjectionError::RequiredUnsupported,
                );
            }
            let source_destination = RelativeArtifactPath::new("projection/skill").unwrap();
            let (_, files) = context
                .filesystem
                .load_tree_bounded_no_follow(
                    checkout.root(),
                    &source_destination,
                    managed_tree_observation_limits(),
                )
                .map_err(|_| {
                    skilltap_core::managed_projection::ManagedProjectionError::RequiredUnsupported
                })?;
            let tree = ArtifactTree::new(
                files
                    .into_iter()
                    .map(|(path, file)| (path.as_str().to_owned(), file)),
            )
            .map_err(|_| fake_projection_error())?;
            if !tree
                .files()
                .contains_key(&RelativeArtifactPath::new("SKILL.md").unwrap())
            {
                return Err(
                    skilltap_core::managed_projection::ManagedProjectionError::RequiredUnsupported,
                );
            }
            let mcp = context
                .filesystem
                .read_regular_bounded_no_follow(
                    checkout.root(),
                    &RelativeArtifactPath::new("projection/mcp.conf").unwrap(),
                    4096,
                )
                .map_err(|_| fake_projection_error())?;
            let mcp = mcp.ok_or_else(fake_projection_error)?;
            let mut manifest = vec![
                ManagedProjection::Skill {
                    id: skill_destination.clone(),
                    fingerprint: fake_tree_fingerprint(&skill_destination, &tree),
                },
                ManagedProjection::Mcp {
                    id: NativeId::new("demo-docs").unwrap(),
                    fingerprint: fingerprint_contents(&mcp),
                },
            ];
            if fake_marker(context, checkout.root(), "optional-omission")? {
                manifest.push(fake_omission());
            }
            (Some(tree), Some(mcp), manifest)
        }
        ManagedProjectionInput::Remove => (None, None, Vec::new()),
    };
    manifest.sort();
    let current_fingerprint = fake_aggregate_fingerprint(
        current_tree.as_ref().map(|(_, tree)| tree),
        current_mcp.as_deref(),
    );
    let desired_fingerprint =
        fake_aggregate_fingerprint(desired_tree.as_ref(), desired_mcp.as_deref());

    Ok(skilltap_core::managed_projection::ManagedProjectionPlan {
        trees: vec![ManagedPluginWrite {
            root: skill_root,
            destination: skill_destination,
            desired_tree,
            expected_tree: current_tree.as_ref().map(|(_, tree)| tree.clone()),
            expected_identity: current_tree.map(|(identity, _)| identity),
        }],
        files: vec![ManagedFileWrite {
            root: context_root(context).clone(),
            destination: mcp_destination,
            expected: current_mcp,
            desired: desired_mcp,
        }],
        manifest,
        current_fingerprint,
        desired_fingerprint,
    })
}

fn fake_projection_error() -> skilltap_core::managed_projection::ManagedProjectionError {
    skilltap_core::managed_projection::ManagedProjectionError::Other {
        code: "fake_projection_unreadable",
        summary: "The fake managed projection could not be read.",
    }
}

fn fake_omission() -> ManagedProjection {
    ManagedProjection::Omitted {
        id: skilltap_core::domain::ComponentId::new("optional:fake-hook").unwrap(),
        consequence: skilltap_core::domain::EvidenceCode::new("fake_optional_component_omitted")
            .unwrap(),
    }
}

fn fake_tree_fingerprint(destination: &RelativeArtifactPath, tree: &ArtifactTree) -> Fingerprint {
    let mut bytes = destination.as_str().as_bytes().to_vec();
    for (path, file) in tree.files() {
        bytes.extend_from_slice(path.as_str().as_bytes());
        bytes.push(u8::from(file.is_executable()));
        bytes.extend_from_slice(file.contents());
    }
    fingerprint_contents(&bytes)
}

fn fake_aggregate_fingerprint(
    tree: Option<&ArtifactTree>,
    mcp: Option<&[u8]>,
) -> Option<Fingerprint> {
    let mut bytes = Vec::new();
    if let Some(tree) = tree {
        bytes.extend_from_slice(
            fake_tree_fingerprint(&RelativeArtifactPath::new("demo").unwrap(), tree)
                .digest()
                .as_bytes(),
        );
    }
    if let Some(mcp) = mcp {
        bytes.extend_from_slice(mcp);
    }
    (!bytes.is_empty()).then(|| fingerprint_contents(&bytes))
}

fn fake_target_id() -> HarnessId {
    HarnessId::new("fake-managed").unwrap()
}

fn fake_target() -> TargetArgs {
    TargetArgs {
        target: Some(skilltap_core::domain::TargetSelection::Only(
            fake_target_id(),
        )),
    }
}

fn enable_fake_managed_only(paths: &PlatformPaths, binary: &Path) {
    let filesystem = SystemFileSystem;
    let repository =
        FileConfigRepository::new(&filesystem, paths.skilltap_config().clone()).unwrap();
    let binary = HarnessBinary::new(binary.to_string_lossy().into_owned()).unwrap();
    repository
        .replace(
            &ConfigDocument::defaults()
                .with_harness_policy(&fake_target_id(), true, Some(&binary))
                .unwrap(),
        )
        .unwrap();
}

fn execute_managed_lifecycle(
    paths: &PlatformPaths,
    project: &Path,
    state_filesystem: &dyn FileSystem,
    managed_filesystem: &dyn ManagedLifecycleFileSystem,
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
        test_managed_filesystem: Some(managed_filesystem),
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

#[derive(Clone, Copy)]
struct ManagedLifecycleTestRequest<'a> {
    kind: NativeLifecycleKind,
    source: Option<&'a str>,
    name: Option<&'a str>,
    acknowledged: bool,
}

impl<'a> ManagedLifecycleTestRequest<'a> {
    const fn marketplace_add(source: &'a str) -> Self {
        Self {
            kind: NativeLifecycleKind::MarketplaceAdd,
            source: Some(source),
            name: Some("team"),
            acknowledged: false,
        }
    }

    const fn plugin_install() -> Self {
        Self {
            kind: NativeLifecycleKind::PluginInstall,
            source: Some("demo@team"),
            name: None,
            acknowledged: false,
        }
    }
}

fn execute_fake_managed_lifecycle(
    paths: &PlatformPaths,
    project: &Path,
    kind: NativeLifecycleKind,
    source: Option<&str>,
    name: Option<&str>,
    acknowledged: bool,
) -> Outcome {
    let filesystem = SystemFileSystem;
    execute_fake_managed_lifecycle_with_scope_and_filesystems(
        paths,
        project,
        &managed_scope(project),
        &filesystem,
        &filesystem,
        ManagedLifecycleTestRequest {
            kind,
            source,
            name,
            acknowledged,
        },
    )
}

fn execute_fake_managed_lifecycle_global(
    paths: &PlatformPaths,
    project: &Path,
    kind: NativeLifecycleKind,
    source: Option<&str>,
    name: Option<&str>,
) -> Outcome {
    let filesystem = SystemFileSystem;
    execute_fake_managed_lifecycle_with_scope_and_filesystems(
        paths,
        project,
        &global_scope(),
        &filesystem,
        &filesystem,
        ManagedLifecycleTestRequest {
            kind,
            source,
            name,
            acknowledged: false,
        },
    )
}

fn execute_fake_managed_lifecycle_with_filesystems(
    paths: &PlatformPaths,
    project: &Path,
    state_filesystem: &dyn FileSystem,
    managed_filesystem: &dyn ManagedLifecycleFileSystem,
    request: ManagedLifecycleTestRequest<'_>,
) -> Outcome {
    execute_fake_managed_lifecycle_with_scope_and_filesystems(
        paths,
        project,
        &managed_scope(project),
        state_filesystem,
        managed_filesystem,
        request,
    )
}

fn execute_fake_managed_lifecycle_with_scope_and_filesystems(
    paths: &PlatformPaths,
    project: &Path,
    requested_scope: &ScopeArgs,
    state_filesystem: &dyn FileSystem,
    managed_filesystem: &dyn ManagedLifecycleFileSystem,
    request: ManagedLifecycleTestRequest<'_>,
) -> Outcome {
    let ManagedLifecycleTestRequest {
        kind,
        source,
        name,
        acknowledged,
    } = request;
    let filesystem = SystemFileSystem;
    let config = FileConfigRepository::new(&filesystem, paths.skilltap_config().clone()).unwrap();
    let inventory =
        FileInventoryRepository::new(&filesystem, paths.skilltap_config().clone()).unwrap();
    let state =
        FileStateRepository::new(state_filesystem, paths.skilltap_config().clone()).unwrap();
    let working_directory = FixedWorkingDirectory(absolute(project));
    let git = NoGitRoot;
    let scopes = ScopeResolver::new(&filesystem, &working_directory, &git);
    let registry = skilltap_harnesses::TargetRegistry::new([
        skilltap_harnesses::CodexAdapter::static_ref(),
        skilltap_harnesses::ClaudeAdapter::static_ref(),
        &FAKE_MANAGED_ADAPTER as &'static dyn skilltap_harnesses::HarnessAdapter,
    ]);
    StatusApplication {
        config: &config,
        inventory: &inventory,
        state: &state,
        scopes: &scopes,
        working_directory: &working_directory,
        native_observation: NativeObservationMode::Disabled,
        registry: &registry,
        test_platform_paths: Some(paths.clone()),
        test_managed_filesystem: Some(managed_filesystem),
    }
    .execute_native_lifecycle(
        "fake managed lifecycle test",
        kind,
        requested_scope,
        &fake_target(),
        NativeLifecycleValues { source, name },
        acknowledged,
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
fn fake_managed_projection_uses_the_exact_global_and_project_scopes() {
    let fixture = FakeManagedFixture::new("skilltap-fake-managed-scopes", &[]);

    let global_add = execute_fake_managed_lifecycle_global(
        &fixture.paths,
        &fixture.project,
        NativeLifecycleKind::MarketplaceAdd,
        Some(fixture.source.to_str().unwrap()),
        Some("team"),
    );
    assert_eq!(global_add.result, ResultClass::Completed, "{global_add:?}");
    let global_install = execute_fake_managed_lifecycle_global(
        &fixture.paths,
        &fixture.project,
        NativeLifecycleKind::PluginInstall,
        Some("demo@team"),
        None,
    );
    assert_eq!(
        global_install.result,
        ResultClass::Completed,
        "{global_install:?}"
    );
    let global_root = Path::new(fixture.paths.home().as_str());
    assert!(global_root.join(".fake/skills/demo/SKILL.md").is_file());
    assert!(global_root.join(".fake/config.toml").is_file());

    let project_add = fixture.add_marketplace();
    assert_eq!(
        project_add.result,
        ResultClass::Completed,
        "{project_add:?}"
    );
    let project_install = fixture.install_plugin(false);
    assert_eq!(
        project_install.result,
        ResultClass::Completed,
        "{project_install:?}"
    );
    assert!(fixture.project.join(".fake/skills/demo/SKILL.md").is_file());
    assert!(fixture.project.join(".fake/config.toml").is_file());

    let repeated = execute_fake_managed_lifecycle_global(
        &fixture.paths,
        &fixture.project,
        NativeLifecycleKind::PluginInstall,
        Some("demo@team"),
        None,
    );
    assert_eq!(repeated.result, ResultClass::Completed, "{repeated:?}");
    assert_changed(&repeated, false);
}

#[test]
fn conditional_profile_application_guard_blocks_every_fixture_case_without_widening() {
    for case in ConditionalFixtureCase::ALL {
        let machine = IsolatedMachine::new("pi-application-matrix").unwrap();
        let fixture = ConditionalTargetFixture::pi(case);
        let roots = fixture.install(&machine).unwrap();
        let paths = PlatformPaths::resolve_for(
            SupportedPlatform::Linux,
            &FixtureEnvironment {
                home: machine.home().as_os_str().to_owned(),
                config: machine.configuration_home().as_os_str().to_owned(),
                cache: machine.cache_home().as_os_str().to_owned(),
            },
        )
        .unwrap();
        let project = AbsolutePath::new(roots.project().to_string_lossy()).unwrap();
        let scope = if case == ConditionalFixtureCase::ProjectTrust {
            Scope::Project(project)
        } else {
            Scope::Global
        };
        let profile = skilltap_harnesses::PiConditionalProfile::static_ref();
        let report = profile
            .inspect_components(&skilltap_harnesses::ConditionalProfileContext {
                scope: &scope,
                paths: &paths,
                filesystem: &SystemFileSystem,
                json_limits: skilltap_core::runtime::JsonLimits::new(64 * 1024, 32).unwrap(),
                maximum_manifest_bytes: 64 * 1024,
            })
            .unwrap();
        let compiled = profile
            .select_compiled_profile(&NativeVersion::new("0.80.6").unwrap(), report.components());
        let observation = ConditionalProfileObservation::compose(compiled, report).unwrap();
        assert_eq!(
            observation.mutation_support(&scope, &CapabilityId::new("skill.install").unwrap()),
            CapabilitySupport::Unsupported,
            "case {case:?} must remain observe-only"
        );
        assert_eq!(
            observation
                .profile()
                .observation_capabilities()
                .for_scope(&scope)
                .support(&CapabilityId::new("component.mcp").unwrap()),
            Some(CapabilitySupport::Unverified),
            "case {case:?} must retain MCP narrowing"
        );
        assert_eq!(
            observation
                .profile()
                .observation_capabilities()
                .for_scope(&scope)
                .support(&CapabilityId::new("component.hook").unwrap()),
            Some(CapabilitySupport::Unsupported),
            "case {case:?} must retain hook narrowing"
        );
    }
}

#[test]
fn managed_projection_profiles_pass_the_shared_acceptance_matrix_repeatedly() {
    let codex = FakeHarnessProfile::codex();
    let codex = codex
        .managed_projection()
        .expect("Codex opts into managed fallback acceptance");
    assert!(FakeHarnessProfile::claude().managed_projection().is_none());

    let gemini = ManagedProjectionProfile::gemini();
    let opencode = ManagedProjectionProfile::opencode();
    let kiro = ManagedProjectionProfile::kiro();
    let copilot = ManagedProjectionProfile::copilot();
    let kimi = ManagedProjectionProfile::kimi();
    let vibe = ManagedProjectionProfile::vibe();
    let kilo = ManagedProjectionProfile::kilo();
    for run in 0..2 {
        for profile in [
            codex,
            &FAKE_MANAGED_PROFILE,
            &gemini,
            &opencode,
            &kiro,
            &copilot,
            &kimi,
            &vibe,
            &kilo,
        ] {
            let report = managed_acceptance_matrix(profile, exercise_managed_acceptance)
                .unwrap_or_else(|error| panic!("matrix run {run} failed: {error}"));
            assert_eq!(report.profile_id(), profile.id());
            assert!(report.passed());
            assert_eq!(report.scenarios().count(), 8);
        }
    }
}

fn exercise_managed_acceptance(
    profile: &ManagedProjectionProfile,
    scenario: ManagedAcceptanceScenario,
) -> ManagedAcceptanceEvidence {
    match profile.id() {
        "codex" => exercise_codex_managed_acceptance(scenario),
        "fake-managed" | "gemini" | "opencode" | "kiro" | "copilot" => {
            exercise_fake_managed_acceptance(scenario)
        }
        "kimi" | "vibe" | "kilo" => exercise_declaration_managed_acceptance(profile, scenario),
        other => panic!("no managed acceptance runner registered for {other}"),
    }
}

fn evidence(checks: impl IntoIterator<Item = ManagedAcceptanceCheck>) -> ManagedAcceptanceEvidence {
    ManagedAcceptanceEvidence::new(checks)
}

fn exercise_declaration_managed_acceptance(
    profile: &ManagedProjectionProfile,
    scenario: ManagedAcceptanceScenario,
) -> ManagedAcceptanceEvidence {
    assert!(profile.declaration_managed());
    if scenario == ManagedAcceptanceScenario::FreshLoadVerification {
        let registry = TargetRegistry::canonical();
        let adapter = registry
            .adapter(&HarnessId::new(profile.id()).unwrap())
            .expect("declaration-managed target is registered");
        assert!(adapter.effective_state_probe().is_none());
        assert!(adapter.native_lifecycle().is_none());
        return evidence([ManagedAcceptanceCheck::DeclarationStatusPending]);
    }
    exercise_fake_managed_acceptance(scenario)
}

struct FakeManagedFixture {
    _root: TempRoot,
    _native: FakeNativeProcess,
    paths: PlatformPaths,
    project: PathBuf,
    source: PathBuf,
}

impl FakeManagedFixture {
    fn new(prefix: &str, markers: &[&str]) -> Self {
        let root = TempRoot::new(prefix).unwrap();
        let paths = isolated_platform_paths(&root);
        let native = FakeHarnessProfile::codex()
            .builder(FakeNativeMode::VersionKnown)
            .build()
            .unwrap();
        let binary = native
            .install_alias(&root.join("fake-bin"), "fake-managed")
            .unwrap();
        enable_fake_managed_only(&paths, &binary);
        let project = root.join("project");
        let source = root.join("marketplace");
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(source.join("projection/skill/scripts")).unwrap();
        fs::create_dir_all(source.join("projection/skill/references")).unwrap();
        fs::write(
            source.join("projection/skill/SKILL.md"),
            "---\nname: demo\ndescription: fake managed fixture\n---\n",
        )
        .unwrap();
        fs::write(source.join("projection/skill/scripts/run.sh"), "exit 0\n").unwrap();
        fs::write(
            source.join("projection/skill/references/usage.md"),
            "# Usage\n",
        )
        .unwrap();
        fs::write(
            source.join("projection/mcp.conf"),
            "server = fake-managed\n",
        )
        .unwrap();
        for marker in markers {
            fs::write(source.join(marker), "fixture\n").unwrap();
        }
        Self {
            _root: root,
            _native: native,
            paths,
            project,
            source,
        }
    }

    fn add_marketplace(&self) -> Outcome {
        execute_fake_managed_lifecycle(
            &self.paths,
            &self.project,
            NativeLifecycleKind::MarketplaceAdd,
            Some(self.source.to_str().unwrap()),
            Some("team"),
            false,
        )
    }

    fn install_plugin(&self, acknowledged: bool) -> Outcome {
        execute_fake_managed_lifecycle(
            &self.paths,
            &self.project,
            NativeLifecycleKind::PluginInstall,
            Some("demo@team"),
            None,
            acknowledged,
        )
    }

    fn target_state(&self) -> TargetResourceState {
        managed_target_state(
            &self.paths,
            &self.project,
            &SystemFileSystem,
            &fake_target_id(),
            "plugin:demo@team",
        )
    }
}

fn exercise_fake_managed_acceptance(
    scenario: ManagedAcceptanceScenario,
) -> ManagedAcceptanceEvidence {
    match scenario {
        ManagedAcceptanceScenario::ApplyProjection => fake_apply_projection_acceptance(),
        ManagedAcceptanceScenario::ProjectionEvidence => fake_projection_evidence_acceptance(),
        ManagedAcceptanceScenario::RemovalWithoutCheckout => fake_removal_acceptance(),
        ManagedAcceptanceScenario::Compatibility => fake_compatibility_acceptance(),
        ManagedAcceptanceScenario::Ownership => fake_ownership_acceptance(),
        ManagedAcceptanceScenario::PendingRecovery => fake_pending_recovery_acceptance(),
        ManagedAcceptanceScenario::FreshLoadVerification => fake_fresh_load_acceptance(),
        ManagedAcceptanceScenario::ImmediateRepeat => fake_repeat_acceptance(),
    }
}

fn fake_apply_projection_acceptance() -> ManagedAcceptanceEvidence {
    let fixture = FakeManagedFixture::new("skilltap-fake-managed-apply", &[]);
    FAKE_APPLY_PLANS.store(0, Ordering::SeqCst);
    let added = fixture.add_marketplace();
    assert_eq!(added.result, ResultClass::Completed, "{added:?}");
    let calls_before_install = FAKE_APPLY_PLANS.load(Ordering::SeqCst);
    let installed = fixture.install_plugin(false);
    assert_eq!(installed.result, ResultClass::Completed, "{installed:?}");
    assert_eq!(
        FAKE_APPLY_PLANS.load(Ordering::SeqCst),
        calls_before_install + 1,
        "one port plan receives one authoritative checkout per apply lifecycle"
    );
    let skill = fixture.project.join(".fake/skills/demo");
    for relative in ["SKILL.md", "scripts/run.sh", "references/usage.md"] {
        assert!(skill.join(relative).is_file(), "missing {relative}");
    }
    evidence([
        ManagedAcceptanceCheck::OneApplyCheckout,
        ManagedAcceptanceCheck::CompleteSkillTree,
    ])
}

fn fake_projection_evidence_acceptance() -> ManagedAcceptanceEvidence {
    let fixture = FakeManagedFixture::new("skilltap-fake-managed-evidence", &[]);
    assert_eq!(fixture.add_marketplace().result, ResultClass::Completed);
    assert_eq!(fixture.install_plugin(false).result, ResultClass::Completed);
    let target = fixture.target_state();
    assert!(target.fingerprint().is_some());
    assert!(matches!(
        target.managed_projections(),
        [ManagedProjection::Skill { id: skill, .. }, ManagedProjection::Mcp { id: mcp, .. }]
            if skill.as_str() == "demo" && mcp.as_str() == "demo-docs"
    ));
    let repeated = fixture.install_plugin(false);
    assert_eq!(repeated.result, ResultClass::Completed, "{repeated:?}");
    assert_changed(&repeated, false);
    evidence([
        ManagedAcceptanceCheck::Manifest,
        ManagedAcceptanceCheck::CurrentFingerprint,
        ManagedAcceptanceCheck::DesiredFingerprint,
    ])
}

fn fake_removal_acceptance() -> ManagedAcceptanceEvidence {
    let fixture = FakeManagedFixture::new("skilltap-fake-managed-remove", &[]);
    assert_eq!(fixture.add_marketplace().result, ResultClass::Completed);
    assert_eq!(fixture.install_plugin(false).result, ResultClass::Completed);
    fs::remove_dir_all(&fixture.source).unwrap();
    FAKE_REMOVE_PLANS.store(0, Ordering::SeqCst);
    let plugin = execute_fake_managed_lifecycle(
        &fixture.paths,
        &fixture.project,
        NativeLifecycleKind::PluginRemove,
        None,
        Some("demo@team"),
        false,
    );
    assert_eq!(plugin.result, ResultClass::Completed, "{plugin:?}");
    assert!(!fixture.project.join(".fake/skills/demo").exists());
    assert!(!fixture.project.join(".fake/config.toml").exists());
    let marketplace = execute_fake_managed_lifecycle(
        &fixture.paths,
        &fixture.project,
        NativeLifecycleKind::MarketplaceRemove,
        None,
        Some("team"),
        false,
    );
    assert_eq!(
        marketplace.result,
        ResultClass::Completed,
        "{marketplace:?}"
    );
    assert!(!fixture.project.join(".fake/managed-marketplace").exists());
    assert_eq!(FAKE_REMOVE_PLANS.load(Ordering::SeqCst), 2);
    evidence([ManagedAcceptanceCheck::RemoveWithoutCheckout])
}

fn fake_compatibility_acceptance() -> ManagedAcceptanceEvidence {
    let optional =
        FakeManagedFixture::new("skilltap-fake-managed-optional", &["optional-omission"]);
    let added = optional.add_marketplace();
    assert_eq!(added.result, ResultClass::Completed, "{added:?}");
    let blocked = optional.install_plugin(false);
    assert_eq!(
        blocked.result,
        ResultClass::AttentionRequired,
        "{blocked:?}"
    );
    assert!(
        blocked
            .errors
            .iter()
            .any(|error| { error.code == "partial_operation_requires_acknowledgment" })
    );
    assert!(!optional.project.join(".fake/skills/demo").exists());
    let accepted = optional.install_plugin(true);
    assert_eq!(accepted.result, ResultClass::Completed, "{accepted:?}");
    assert!(matches!(
        optional.target_state().managed_projections(),
        [
            ManagedProjection::Skill { .. },
            ManagedProjection::Mcp { .. },
            ManagedProjection::Omitted { consequence, .. }
        ] if consequence.as_str() == "fake_optional_component_omitted"
    ));

    let required =
        FakeManagedFixture::new("skilltap-fake-managed-required", &["required-unsupported"]);
    assert_eq!(required.add_marketplace().result, ResultClass::Completed);
    let blocked = required.install_plugin(true);
    assert_eq!(
        blocked.result,
        ResultClass::AttentionRequired,
        "{blocked:?}"
    );
    assert!(
        blocked
            .errors
            .iter()
            .any(|error| error.code == "required_unsupported"),
        "{blocked:?}"
    );
    assert!(!required.project.join(".fake/skills/demo").exists());
    evidence([
        ManagedAcceptanceCheck::OmissionAcknowledgment,
        ManagedAcceptanceCheck::RequiredUnsupported,
    ])
}

fn fake_ownership_acceptance() -> ManagedAcceptanceEvidence {
    let fixture = FakeManagedFixture::new("skilltap-fake-managed-ownership", &[]);
    assert_eq!(fixture.add_marketplace().result, ResultClass::Completed);
    assert_eq!(fixture.install_plugin(false).result, ResultClass::Completed);
    let target_before = fixture.target_state();
    assert_eq!(target_before.ownership(), Ownership::Skilltap);

    let repository =
        FileStateRepository::new(&SystemFileSystem, fixture.paths.skilltap_config().clone())
            .unwrap();
    let mut document = match repository.load().unwrap() {
        DocumentState::Present(document) => document,
        DocumentState::Missing => unreachable!(),
    };
    let key = ResourceKey::new(
        ResourceId::new("plugin:demo@team").unwrap(),
        Scope::Project(absolute(&fs::canonicalize(&fixture.project).unwrap())),
    );
    let sibling = TargetResourceState::new(
        HarnessId::new("codex").unwrap(),
        Some(NativeId::new("demo@team").unwrap()),
        Provenance::Native,
        Ownership::Harness,
        None,
        None,
        Some(fingerprint_contents(b"codex-sibling")),
        None,
        None,
        Timestamp::new(1, 0).unwrap(),
        None,
    )
    .unwrap();
    let resource = document
        .resources()
        .get(&key)
        .unwrap()
        .clone()
        .with_target(sibling)
        .unwrap();
    document = document.refresh_resource_state(resource).unwrap();
    repository.replace(&document).unwrap();
    let repeat = fixture.install_plugin(false);
    assert_eq!(repeat.result, ResultClass::Completed, "{repeat:?}");
    let document = match repository.load().unwrap() {
        DocumentState::Present(document) => document,
        DocumentState::Missing => unreachable!(),
    };
    assert!(
        document
            .resources()
            .get(&key)
            .and_then(|resource| resource.target(&HarnessId::new("codex").unwrap()))
            .is_some()
    );

    fs::write(
        fixture.project.join(".fake/skills/demo/SKILL.md"),
        "drift\n",
    )
    .unwrap();
    let drifted = execute_fake_managed_lifecycle(
        &fixture.paths,
        &fixture.project,
        NativeLifecycleKind::PluginUpdate,
        None,
        Some("demo@team"),
        false,
    );
    assert_error_code(&drifted, "managed_project_drifted");

    let unowned = FakeManagedFixture::new("skilltap-fake-managed-unowned", &[]);
    assert_eq!(unowned.add_marketplace().result, ResultClass::Completed);
    fs::create_dir_all(unowned.project.join(".fake")).unwrap();
    fs::write(unowned.project.join(".fake/config.toml"), "foreign\n").unwrap();
    let rejected = unowned.install_plugin(false);
    assert_error_code(&rejected, "managed_project_unowned");

    let changed = FakeManagedFixture::new("skilltap-fake-managed-update", &[]);
    assert_eq!(changed.add_marketplace().result, ResultClass::Completed);
    assert_eq!(changed.install_plugin(false).result, ResultClass::Completed);
    fs::write(
        changed.source.join("projection/skill/SKILL.md"),
        "---\nname: demo\ndescription: changed\n---\n",
    )
    .unwrap();
    let rejected = changed.install_plugin(false);
    assert_error_code(&rejected, "managed_project_update_required");

    evidence([
        ManagedAcceptanceCheck::OwnedDestination,
        ManagedAcceptanceCheck::DriftRejected,
        ManagedAcceptanceCheck::UnownedRejected,
        ManagedAcceptanceCheck::UpdateRequired,
        ManagedAcceptanceCheck::TargetStateIsolated,
    ])
}

fn fake_pending_recovery_acceptance() -> ManagedAcceptanceEvidence {
    let fixture = FakeManagedFixture::new("skilltap-fake-managed-pending", &[]);
    let managed_filesystem = RecordingFaultFileSystem::new();
    let state_filesystem = RecordingFaultFileSystem::new();
    let add = execute_fake_managed_lifecycle_with_filesystems(
        &fixture.paths,
        &fixture.project,
        &state_filesystem,
        &managed_filesystem,
        ManagedLifecycleTestRequest::marketplace_add(fixture.source.to_str().unwrap()),
    );
    assert_eq!(add.result, ResultClass::Completed, "{add:?}");
    state_filesystem.fail_atomic_write_number(2);
    let failed = execute_fake_managed_lifecycle_with_filesystems(
        &fixture.paths,
        &fixture.project,
        &state_filesystem,
        &managed_filesystem,
        ManagedLifecycleTestRequest::plugin_install(),
    );
    assert_eq!(failed.result, ResultClass::AttentionRequired, "{failed:?}");
    let pending = managed_target_state(
        &fixture.paths,
        &fixture.project,
        &state_filesystem,
        &fake_target_id(),
        "plugin:demo@team",
    );
    assert!(pending.pending_managed_attempt().is_some());
    let publications = managed_filesystem.tree_publish_successes.get();
    let recovered = execute_fake_managed_lifecycle_with_filesystems(
        &fixture.paths,
        &fixture.project,
        &state_filesystem,
        &managed_filesystem,
        ManagedLifecycleTestRequest::plugin_install(),
    );
    assert_eq!(recovered.result, ResultClass::Completed, "{recovered:?}");
    assert_changed(&recovered, false);
    assert_eq!(
        managed_filesystem.tree_publish_successes.get(),
        publications
    );
    let recovered = managed_target_state(
        &fixture.paths,
        &fixture.project,
        &state_filesystem,
        &fake_target_id(),
        "plugin:demo@team",
    );
    assert!(recovered.pending_managed_attempt().is_none());
    evidence([
        ManagedAcceptanceCheck::PendingRetry,
        ManagedAcceptanceCheck::RetryNoChange,
    ])
}

fn fake_fresh_load_acceptance() -> ManagedAcceptanceEvidence {
    let fixture = FakeManagedFixture::new("skilltap-fake-managed-fresh-load", &[]);
    let managed_filesystem = RecordingFaultFileSystem::new();
    let state_filesystem = RecordingFaultFileSystem::new();
    managed_filesystem.fail_next_post_write_read();
    let failed = execute_fake_managed_lifecycle_with_filesystems(
        &fixture.paths,
        &fixture.project,
        &state_filesystem,
        &managed_filesystem,
        ManagedLifecycleTestRequest::marketplace_add(fixture.source.to_str().unwrap()),
    );
    assert_eq!(failed.result, ResultClass::AttentionRequired, "{failed:?}");
    assert!(managed_filesystem.post_write_read_calls.get() > 0);
    assert!(!fixture.project.join(".fake/managed-marketplace").exists());
    let retry = execute_fake_managed_lifecycle_with_filesystems(
        &fixture.paths,
        &fixture.project,
        &state_filesystem,
        &managed_filesystem,
        ManagedLifecycleTestRequest::marketplace_add(fixture.source.to_str().unwrap()),
    );
    assert_eq!(retry.result, ResultClass::Completed, "{retry:?}");
    evidence([ManagedAcceptanceCheck::FreshLoadObserved])
}

fn fake_repeat_acceptance() -> ManagedAcceptanceEvidence {
    let fixture = FakeManagedFixture::new("skilltap-fake-managed-repeat", &[]);
    let managed_filesystem = RecordingFaultFileSystem::new();
    let state_filesystem = RecordingFaultFileSystem::new();
    let add = execute_fake_managed_lifecycle_with_filesystems(
        &fixture.paths,
        &fixture.project,
        &state_filesystem,
        &managed_filesystem,
        ManagedLifecycleTestRequest::marketplace_add(fixture.source.to_str().unwrap()),
    );
    assert_eq!(add.result, ResultClass::Completed, "{add:?}");
    let install = execute_fake_managed_lifecycle_with_filesystems(
        &fixture.paths,
        &fixture.project,
        &state_filesystem,
        &managed_filesystem,
        ManagedLifecycleTestRequest::plugin_install(),
    );
    assert_eq!(install.result, ResultClass::Completed, "{install:?}");
    let project_before = snapshot_tree(&fixture.project).unwrap();
    let state_before = managed_target_state(
        &fixture.paths,
        &fixture.project,
        &state_filesystem,
        &fake_target_id(),
        "plugin:demo@team",
    );
    let resource_count = managed_state_document(&fixture.paths, &state_filesystem)
        .resources()
        .len();
    let publications = managed_filesystem.tree_publish_successes.get();
    let repeat = execute_fake_managed_lifecycle_with_filesystems(
        &fixture.paths,
        &fixture.project,
        &state_filesystem,
        &managed_filesystem,
        ManagedLifecycleTestRequest::plugin_install(),
    );
    assert_eq!(repeat.result, ResultClass::Completed, "{repeat:?}");
    assert_changed(&repeat, false);
    assert_eq!(snapshot_tree(&fixture.project).unwrap(), project_before);
    assert_eq!(
        managed_filesystem.tree_publish_successes.get(),
        publications
    );
    let state_after = managed_target_state(
        &fixture.paths,
        &fixture.project,
        &state_filesystem,
        &fake_target_id(),
        "plugin:demo@team",
    );
    assert_eq!(
        state_after.managed_projections(),
        state_before.managed_projections()
    );
    assert_eq!(state_after.managed_projections().len(), 2);
    assert_eq!(
        managed_state_document(&fixture.paths, &state_filesystem)
            .resources()
            .len(),
        resource_count
    );
    evidence([
        ManagedAcceptanceCheck::ImmediateRepeatNoChange,
        ManagedAcceptanceCheck::NoDuplicateArtifacts,
        ManagedAcceptanceCheck::NoDuplicateState,
    ])
}

fn assert_error_code(outcome: &Outcome, code: &str) {
    assert_eq!(
        outcome.result,
        ResultClass::AttentionRequired,
        "{outcome:?}"
    );
    assert!(
        outcome.errors.iter().any(|error| error.code == code),
        "expected {code}: {outcome:?}"
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

fn exercise_codex_managed_acceptance(
    scenario: ManagedAcceptanceScenario,
) -> ManagedAcceptanceEvidence {
    match scenario {
        ManagedAcceptanceScenario::ApplyProjection => codex_apply_projection_acceptance(),
        ManagedAcceptanceScenario::ProjectionEvidence => codex_projection_evidence_acceptance(),
        ManagedAcceptanceScenario::RemovalWithoutCheckout => {
            codex_removal_without_checkout_acceptance()
        }
        ManagedAcceptanceScenario::Compatibility => codex_compatibility_acceptance(),
        ManagedAcceptanceScenario::Ownership => codex_ownership_acceptance(),
        ManagedAcceptanceScenario::PendingRecovery => {
            managed_project_publication_failures_restore_then_retry_once_and_noop();
            managed_terminal_journal_failure_recovers_without_duplicate_projection_publication();
            evidence([
                ManagedAcceptanceCheck::PendingRetry,
                ManagedAcceptanceCheck::RetryNoChange,
            ])
        }
        ManagedAcceptanceScenario::FreshLoadVerification => codex_fresh_load_acceptance(),
        ManagedAcceptanceScenario::ImmediateRepeat => codex_repeat_acceptance(),
    }
}

fn codex_fixture(prefix: &str) -> (TempRoot, PlatformPaths, PathBuf, PathBuf) {
    let root = TempRoot::new(prefix).unwrap();
    let paths = isolated_platform_paths(&root);
    enable_codex_only(&paths);
    let project = root.join("project");
    let source = root.join("marketplace");
    fs::create_dir_all(&project).unwrap();
    write_managed_marketplace(&source);
    (root, paths, project, source)
}

fn install_codex_fixture(
    paths: &PlatformPaths,
    project: &Path,
    source: &Path,
    managed_filesystem: &dyn ManagedLifecycleFileSystem,
    state_filesystem: &dyn FileSystem,
    acknowledged: bool,
) -> Outcome {
    let add = execute_managed_lifecycle(
        paths,
        project,
        state_filesystem,
        managed_filesystem,
        NativeLifecycleKind::MarketplaceAdd,
        Some(source.to_str().unwrap()),
        Some("team"),
    );
    assert_eq!(add.result, ResultClass::Completed, "{add:?}");
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
        test_managed_filesystem: Some(managed_filesystem),
    }
    .execute_native_lifecycle(
        "managed acceptance install",
        NativeLifecycleKind::PluginInstall,
        &managed_scope(project),
        &codex_target(),
        NativeLifecycleValues {
            source: Some("demo@team"),
            name: None,
        },
        acknowledged,
    )
}

fn codex_apply_projection_acceptance() -> ManagedAcceptanceEvidence {
    let profile = ManagedProjectionProfile::codex();
    let (_root, paths, project, source) = codex_fixture("skilltap-codex-managed-apply");
    fs::create_dir_all(project.join(".codex")).unwrap();
    fs::write(
        project.join(".codex/config.toml"),
        "[mcp_servers.unmanaged]\ncommand = \"leave-me\"\n",
    )
    .unwrap();
    let managed_filesystem = RecordingFaultFileSystem::new();
    let state_filesystem = RecordingFaultFileSystem::new();
    let installed = install_codex_fixture(
        &paths,
        &project,
        &source,
        &managed_filesystem,
        &state_filesystem,
        false,
    );
    assert_eq!(installed.result, ResultClass::Completed, "{installed:?}");
    assert!(project.join(profile.catalog_destinations()[0]).is_file());
    assert!(!project.join(profile.catalog_destinations()[1]).exists());
    for relative in ["SKILL.md", "scripts/run.sh", "references/usage.md"] {
        assert!(
            project.join(".agents/skills/demo").join(relative).is_file(),
            "missing {relative}"
        );
    }
    let mcp = fs::read_to_string(project.join(profile.mcp_destination().unwrap())).unwrap();
    assert!(mcp.contains("[mcp_servers.demo-docs]"));
    assert!(mcp.contains("[mcp_servers.unmanaged]"));
    assert!(mcp.contains("command = \"leave-me\""));

    let (_root, paths, legacy_project, legacy_source) =
        codex_fixture("skilltap-codex-managed-legacy-catalog");
    fs::create_dir_all(legacy_source.join(".claude-plugin")).unwrap();
    fs::rename(
        legacy_source.join(profile.catalog_destinations()[0]),
        legacy_source.join(profile.catalog_destinations()[1]),
    )
    .unwrap();
    let managed_filesystem = RecordingFaultFileSystem::new();
    let state_filesystem = RecordingFaultFileSystem::new();
    let installed = install_codex_fixture(
        &paths,
        &legacy_project,
        &legacy_source,
        &managed_filesystem,
        &state_filesystem,
        false,
    );
    assert_eq!(installed.result, ResultClass::Completed, "{installed:?}");
    assert!(
        legacy_project
            .join(profile.catalog_destinations()[0])
            .is_file()
    );
    assert!(
        legacy_project
            .join(profile.skill_destination())
            .join("demo/SKILL.md")
            .is_file()
    );
    evidence([
        ManagedAcceptanceCheck::OneApplyCheckout,
        ManagedAcceptanceCheck::CompleteSkillTree,
    ])
}

fn codex_projection_evidence_acceptance() -> ManagedAcceptanceEvidence {
    let (_root, paths, project, source) = codex_fixture("skilltap-codex-managed-evidence");
    let managed_filesystem = RecordingFaultFileSystem::new();
    let state_filesystem = RecordingFaultFileSystem::new();
    let installed = install_codex_fixture(
        &paths,
        &project,
        &source,
        &managed_filesystem,
        &state_filesystem,
        false,
    );
    assert_eq!(installed.result, ResultClass::Completed, "{installed:?}");
    let target = managed_plugin_target(&paths, &project, &state_filesystem);
    assert!(target.fingerprint().is_some());
    assert!(matches!(
        target.managed_projections(),
        [ManagedProjection::Skill { id: skill, .. }, ManagedProjection::Mcp { id: mcp, .. }]
            if skill.as_str() == "demo" && mcp.as_str() == "demo-docs"
    ));
    let repeated = execute_managed_lifecycle(
        &paths,
        &project,
        &state_filesystem,
        &managed_filesystem,
        NativeLifecycleKind::PluginInstall,
        Some("demo@team"),
        None,
    );
    assert_eq!(repeated.result, ResultClass::Completed, "{repeated:?}");
    assert_changed(&repeated, false);
    evidence([
        ManagedAcceptanceCheck::Manifest,
        ManagedAcceptanceCheck::CurrentFingerprint,
        ManagedAcceptanceCheck::DesiredFingerprint,
    ])
}

fn codex_removal_without_checkout_acceptance() -> ManagedAcceptanceEvidence {
    managed_marketplace_removal_uses_observed_projection_without_source();
    let (_root, paths, project, source) = codex_fixture("skilltap-codex-managed-remove");
    let managed_filesystem = RecordingFaultFileSystem::new();
    let state_filesystem = RecordingFaultFileSystem::new();
    let installed = install_codex_fixture(
        &paths,
        &project,
        &source,
        &managed_filesystem,
        &state_filesystem,
        false,
    );
    assert_eq!(installed.result, ResultClass::Completed, "{installed:?}");
    fs::remove_dir_all(&source).unwrap();
    let plugin = execute_managed_lifecycle(
        &paths,
        &project,
        &state_filesystem,
        &managed_filesystem,
        NativeLifecycleKind::PluginRemove,
        None,
        Some("demo@team"),
    );
    assert_eq!(plugin.result, ResultClass::Completed, "{plugin:?}");
    assert!(!project.join(".agents/skills/demo").exists());
    assert!(!project.join(".codex/config.toml").exists());
    let marketplace = execute_managed_lifecycle(
        &paths,
        &project,
        &state_filesystem,
        &managed_filesystem,
        NativeLifecycleKind::MarketplaceRemove,
        None,
        Some("team"),
    );
    assert_eq!(
        marketplace.result,
        ResultClass::Completed,
        "{marketplace:?}"
    );
    assert!(!project.join(".agents/plugins/marketplace.json").exists());
    evidence([ManagedAcceptanceCheck::RemoveWithoutCheckout])
}

fn codex_compatibility_acceptance() -> ManagedAcceptanceEvidence {
    let (_root, paths, project, source) = codex_fixture("skilltap-codex-managed-optional");
    fs::write(
        source.join("plugins/demo/.codex-plugin/mcp.json"),
        r#"{"mcpServers":{"plugin-relative":{"command":"${CODEX_PLUGIN_ROOT}/bin/server"}}}"#,
    )
    .unwrap();
    let managed_filesystem = RecordingFaultFileSystem::new();
    let state_filesystem = RecordingFaultFileSystem::new();
    let blocked = install_codex_fixture(
        &paths,
        &project,
        &source,
        &managed_filesystem,
        &state_filesystem,
        false,
    );
    assert_error_code(&blocked, "partial_operation_requires_acknowledgment");
    assert!(!project.join(".agents/skills/demo").exists());
    let accepted = execute_managed_lifecycle_with_acknowledgment(
        &paths,
        &project,
        &state_filesystem,
        &managed_filesystem,
        ManagedLifecycleTestRequest {
            kind: NativeLifecycleKind::PluginInstall,
            source: Some("demo@team"),
            name: None,
            acknowledged: true,
        },
    );
    assert_eq!(accepted.result, ResultClass::Completed, "{accepted:?}");
    assert!(matches!(
        managed_plugin_target(&paths, &project, &state_filesystem).managed_projections(),
        [ManagedProjection::Skill { .. }, ManagedProjection::Omitted { consequence, .. }]
            if consequence.as_str() == "plugin_root_relative_mcp_omitted"
    ));

    let (_root, paths, project, source) = codex_fixture("skilltap-codex-managed-required");
    fs::remove_file(source.join("plugins/demo/skills/demo/SKILL.md")).unwrap();
    fs::write(
        source.join("plugins/demo/.codex-plugin/mcp.json"),
        r#"{"mcpServers":{"plugin-relative":{"command":"${CODEX_PLUGIN_ROOT}/bin/server"}}}"#,
    )
    .unwrap();
    let managed_filesystem = RecordingFaultFileSystem::new();
    let state_filesystem = RecordingFaultFileSystem::new();
    let blocked = install_codex_fixture(
        &paths,
        &project,
        &source,
        &managed_filesystem,
        &state_filesystem,
        true,
    );
    assert_eq!(
        blocked.result,
        ResultClass::AttentionRequired,
        "{blocked:?}"
    );
    assert!(
        blocked.errors.iter().any(|error| {
            matches!(
                error.code.as_str(),
                "managed_project_plugin_invalid" | "managed_project_plugin_unsupported"
            )
        }),
        "{blocked:?}"
    );
    assert!(!project.join(".agents/skills/demo").exists());
    assert!(!project.join(".codex/config.toml").exists());
    evidence([
        ManagedAcceptanceCheck::OmissionAcknowledgment,
        ManagedAcceptanceCheck::RequiredUnsupported,
    ])
}

fn codex_ownership_acceptance() -> ManagedAcceptanceEvidence {
    let (_root, paths, project, source) = codex_fixture("skilltap-codex-managed-ownership");
    let managed_filesystem = RecordingFaultFileSystem::new();
    let state_filesystem = RecordingFaultFileSystem::new();
    let installed = install_codex_fixture(
        &paths,
        &project,
        &source,
        &managed_filesystem,
        &state_filesystem,
        false,
    );
    assert_eq!(installed.result, ResultClass::Completed, "{installed:?}");
    assert_eq!(
        managed_plugin_target(&paths, &project, &state_filesystem).ownership(),
        Ownership::Skilltap
    );

    let repository =
        FileStateRepository::new(&state_filesystem, paths.skilltap_config().clone()).unwrap();
    let mut document = match repository.load().unwrap() {
        DocumentState::Present(document) => document,
        DocumentState::Missing => unreachable!(),
    };
    let key = ResourceKey::new(
        ResourceId::new("plugin:demo@team").unwrap(),
        Scope::Project(absolute(&fs::canonicalize(&project).unwrap())),
    );
    let sibling = TargetResourceState::new(
        fake_target_id(),
        Some(NativeId::new("demo@team").unwrap()),
        Provenance::Native,
        Ownership::Harness,
        None,
        None,
        Some(fingerprint_contents(b"fake-sibling")),
        None,
        None,
        Timestamp::new(1, 0).unwrap(),
        None,
    )
    .unwrap();
    let resource = document
        .resources()
        .get(&key)
        .unwrap()
        .clone()
        .with_target(sibling)
        .unwrap();
    document = document.refresh_resource_state(resource).unwrap();
    repository.replace(&document).unwrap();
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
    let document = match repository.load().unwrap() {
        DocumentState::Present(document) => document,
        DocumentState::Missing => unreachable!(),
    };
    assert!(
        document
            .resources()
            .get(&key)
            .and_then(|resource| resource.target(&fake_target_id()))
            .is_some()
    );

    fs::write(project.join(".agents/skills/demo/SKILL.md"), "drift\n").unwrap();
    let drifted = execute_managed_lifecycle(
        &paths,
        &project,
        &state_filesystem,
        &managed_filesystem,
        NativeLifecycleKind::PluginUpdate,
        None,
        Some("demo@team"),
    );
    assert_error_code(&drifted, "managed_project_drifted");

    let (_root, paths, project, source) = codex_fixture("skilltap-codex-managed-unowned");
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
    fs::create_dir_all(project.join(".codex")).unwrap();
    fs::write(
        project.join(".codex/config.toml"),
        "[mcp_servers.demo-docs]\ncommand = \"foreign\"\n",
    )
    .unwrap();
    let unowned = execute_managed_lifecycle(
        &paths,
        &project,
        &state_filesystem,
        &managed_filesystem,
        NativeLifecycleKind::PluginInstall,
        Some("demo@team"),
        None,
    );
    assert_error_code(&unowned, "managed_project_unowned");

    let (_root, paths, project, source) = codex_fixture("skilltap-codex-managed-update");
    let managed_filesystem = RecordingFaultFileSystem::new();
    let state_filesystem = RecordingFaultFileSystem::new();
    let installed = install_codex_fixture(
        &paths,
        &project,
        &source,
        &managed_filesystem,
        &state_filesystem,
        false,
    );
    assert_eq!(installed.result, ResultClass::Completed, "{installed:?}");
    fs::write(
        source.join("plugins/demo/skills/demo/SKILL.md"),
        "---\nname: demo\ndescription: changed\n---\n",
    )
    .unwrap();
    let update_required = execute_managed_lifecycle(
        &paths,
        &project,
        &state_filesystem,
        &managed_filesystem,
        NativeLifecycleKind::PluginInstall,
        Some("demo@team"),
        None,
    );
    assert_error_code(&update_required, "managed_project_update_required");

    evidence([
        ManagedAcceptanceCheck::OwnedDestination,
        ManagedAcceptanceCheck::DriftRejected,
        ManagedAcceptanceCheck::UnownedRejected,
        ManagedAcceptanceCheck::UpdateRequired,
        ManagedAcceptanceCheck::TargetStateIsolated,
    ])
}

fn codex_fresh_load_acceptance() -> ManagedAcceptanceEvidence {
    managed_project_tree_limits_preserve_planning_and_revalidation_failures();
    let (_root, paths, project, source) = codex_fixture("skilltap-codex-managed-fresh-load");
    let managed_filesystem = RecordingFaultFileSystem::new();
    let state_filesystem = RecordingFaultFileSystem::new();
    managed_filesystem.fail_next_post_write_read();
    let failed = execute_managed_lifecycle(
        &paths,
        &project,
        &state_filesystem,
        &managed_filesystem,
        NativeLifecycleKind::MarketplaceAdd,
        Some(source.to_str().unwrap()),
        Some("team"),
    );
    assert_eq!(failed.result, ResultClass::AttentionRequired, "{failed:?}");
    assert!(managed_filesystem.post_write_read_calls.get() > 0);
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
    evidence([ManagedAcceptanceCheck::FreshLoadObserved])
}

fn codex_repeat_acceptance() -> ManagedAcceptanceEvidence {
    let (_root, paths, project, source) = codex_fixture("skilltap-codex-managed-repeat");
    let managed_filesystem = RecordingFaultFileSystem::new();
    let state_filesystem = RecordingFaultFileSystem::new();
    let installed = install_codex_fixture(
        &paths,
        &project,
        &source,
        &managed_filesystem,
        &state_filesystem,
        false,
    );
    assert_eq!(installed.result, ResultClass::Completed, "{installed:?}");
    let project_before = snapshot_tree(&project).unwrap();
    let state_before = managed_plugin_target(&paths, &project, &state_filesystem);
    let resource_count = managed_state_document(&paths, &state_filesystem)
        .resources()
        .len();
    let publications = managed_filesystem.tree_publish_successes.get();
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
    assert_eq!(snapshot_tree(&project).unwrap(), project_before);
    assert_eq!(
        managed_filesystem.tree_publish_successes.get(),
        publications
    );
    let state_after = managed_plugin_target(&paths, &project, &state_filesystem);
    assert_eq!(
        state_after.managed_projections(),
        state_before.managed_projections()
    );
    assert_eq!(state_after.managed_projections().len(), 2);
    assert_eq!(
        managed_state_document(&paths, &state_filesystem)
            .resources()
            .len(),
        resource_count
    );
    evidence([
        ManagedAcceptanceCheck::ImmediateRepeatNoChange,
        ManagedAcceptanceCheck::NoDuplicateArtifacts,
        ManagedAcceptanceCheck::NoDuplicateState,
    ])
}

fn execute_managed_lifecycle_with_acknowledgment(
    paths: &PlatformPaths,
    project: &Path,
    state_filesystem: &dyn FileSystem,
    managed_filesystem: &dyn ManagedLifecycleFileSystem,
    request: ManagedLifecycleTestRequest<'_>,
) -> Outcome {
    let ManagedLifecycleTestRequest {
        kind,
        source,
        name,
        acknowledged,
    } = request;
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
        test_managed_filesystem: Some(managed_filesystem),
    }
    .execute_native_lifecycle(
        "managed lifecycle acceptance",
        kind,
        &managed_scope(project),
        &codex_target(),
        NativeLifecycleValues { source, name },
        acknowledged,
    )
}

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
                .set_len(managed_tree_observation_limits().file_bytes() + 1)
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
        test_managed_filesystem: None,
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
        let operation = plan.iter().next().unwrap().1;
        let operation_id = operation.id().clone();
        let target_id = operation.target().clone();
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
            validate_managed_ownership(
                if action == OperationAction::PluginUpdate {
                    NativeLifecycleKind::PluginUpdate
                } else {
                    NativeLifecycleKind::PluginInstall
                },
                document.resources().get(&key),
                &target_id,
                ManagedOwnershipEvidence {
                    current_fingerprint: Some(&desired_fingerprint),
                    desired_fingerprint: Some(&desired_fingerprint),
                    desired_projections: &desired_projections,
                    installed_revision: None,
                    operation_id: &operation_id,
                },
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
