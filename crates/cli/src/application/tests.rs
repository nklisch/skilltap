use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use skilltap_core::{
    domain::{AbsolutePath, HarnessId},
    runtime::{GitRoot, ScopeResolver, SystemFileSystem, WorkingDirectory},
    storage::{
        ConfigDocument, ConfigRepository, FileConfigRepository, FileInventoryRepository,
        FileStateRepository, HarnessPolicies, HarnessPolicy,
    },
};

use super::*;
use crate::command::{OutputArgs, ScopeArgs, TargetArgs};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct TempRoot(PathBuf);

impl TempRoot {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "skilltap-cli-application-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        Self(path)
    }

    fn absolute(&self) -> AbsolutePath {
        AbsolutePath::new(self.0.to_str().unwrap()).unwrap()
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
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

fn execute(root: &TempRoot, args: &StatusArgs, cwd: AbsolutePath) -> Outcome {
    let filesystem = SystemFileSystem;
    let config = FileConfigRepository::new(&filesystem, root.absolute()).unwrap();
    let inventory = FileInventoryRepository::new(&filesystem, root.absolute()).unwrap();
    let state = FileStateRepository::new(&filesystem, root.absolute()).unwrap();
    let working_directory = FixedWorkingDirectory(cwd);
    let git = NoGitRoot;
    let scopes = ScopeResolver::new(&filesystem, &working_directory, &git);
    StatusApplication {
        config: &config,
        inventory: &inventory,
        state: &state,
        scopes: &scopes,
        working_directory: &working_directory,
    }
    .execute(args)
}

#[test]
fn first_use_status_uses_defaults_and_creates_nothing() {
    let root = TempRoot::new();
    let cwd = AbsolutePath::new(std::env::current_dir().unwrap().to_str().unwrap()).unwrap();
    assert!(!root.0.exists());

    let outcome = execute(&root, &status_args(ScopeArgs::default()), cwd);

    assert_eq!(outcome.result, ResultClass::AttentionRequired);
    assert_eq!(outcome.scope, Some(OutputScope::Global));
    assert_eq!(outcome.summary.get("targets"), Some(&2_u64.into()));
    assert!(
        outcome
            .warnings
            .iter()
            .any(|warning| warning.code == "native_observation_unavailable")
    );
    assert!(!root.0.exists());
}

#[test]
fn missing_inventory_makes_all_scopes_global_only() {
    let root = TempRoot::new();
    let cwd = AbsolutePath::new(std::env::current_dir().unwrap().to_str().unwrap()).unwrap();
    let args = status_args(ScopeArgs {
        project: None,
        all_scopes: true,
    });

    let outcome = execute(&root, &args, cwd);

    assert_eq!(outcome.scope, Some(OutputScope::Global));
    assert_eq!(outcome.summary.get("scopes"), Some(&1_u64.into()));
}

#[test]
fn relative_project_is_resolved_against_the_working_directory() {
    let root = TempRoot::new();
    let workspace = TempRoot::new();
    let current = workspace.0.join("current");
    let project = workspace.0.join("project");
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
    let root = TempRoot::new();
    let filesystem = SystemFileSystem;
    let repository = FileConfigRepository::new(&filesystem, root.absolute()).unwrap();
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
        let root = TempRoot::new();
        fs::create_dir_all(&root.0).unwrap();
        fs::write(root.0.join(file), "SECRET invalid [[[\n").unwrap();
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
    let root = TempRoot::new();
    let filesystem = SystemFileSystem;
    let repository = FileConfigRepository::new(&filesystem, root.absolute()).unwrap();
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
