use std::{fs, path::PathBuf};

use skilltap_core::{
    domain::{AbsolutePath, HarnessId},
    runtime::{GitRoot, ScopeResolver, SystemFileSystem, WorkingDirectory},
    storage::{
        ConfigDocument, ConfigRepository, FileConfigRepository, FileInventoryRepository,
        FileStateRepository, HarnessPolicies, HarnessPolicy,
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
            path: project.to_str().unwrap().to_owned(),
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
