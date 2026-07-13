use std::{cell::RefCell, collections::BTreeMap, fs, path::PathBuf};

use skilltap_core::{
    domain::{AbsolutePath, HarnessId},
    runtime::{GitRoot, ScopeResolver, SystemFileSystem, WorkingDirectory},
    storage::{
        ConfigDocument, ConfigRepository, FileConfigRepository, FileInventoryRepository,
        FileStateRepository, HarnessPolicies, HarnessPolicy, StateRepository,
    },
};
use skilltap_test_support::TempRoot;

use super::*;
use crate::command::{OutputArgs, ScopeArgs, TargetArgs};

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
    StatusApplication {
        config: &config,
        inventory: &inventory,
        state: &state,
        scopes: &scopes,
        working_directory: &working_directory,
        native_observation: NativeObservationMode::Disabled,
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
    let enabled = ConfigDocument::new(
        defaults.schema(),
        HarnessPolicies {
            codex: HarnessPolicy {
                enabled: true,
                binary: defaults.harnesses().codex.binary.clone(),
            },
            claude: HarnessPolicy {
                enabled: true,
                binary: defaults.harnesses().claude.binary.clone(),
            },
        },
        defaults.instructions().clone(),
        defaults.updates().clone(),
    )
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
    let disabled = ConfigDocument::new(
        defaults.schema(),
        HarnessPolicies {
            codex: HarnessPolicy {
                enabled: false,
                binary: defaults.harnesses().codex.binary.clone(),
            },
            claude: HarnessPolicy {
                enabled: false,
                binary: defaults.harnesses().claude.binary.clone(),
            },
        },
        defaults.instructions().clone(),
        defaults.updates().clone(),
    )
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
    let config = ConfigDocument::new(
        defaults.schema(),
        HarnessPolicies {
            codex: HarnessPolicy {
                enabled: true,
                binary: defaults.harnesses().codex.binary.clone(),
            },
            claude: HarnessPolicy {
                enabled: false,
                binary: defaults.harnesses().claude.binary.clone(),
            },
        },
        defaults.instructions().clone(),
        defaults.updates().clone(),
    )
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
            .record(
                &OperationResult::new(operation_id, OperationOutcome::NoChange).unwrap(),
            )
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
