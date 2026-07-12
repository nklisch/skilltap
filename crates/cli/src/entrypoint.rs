use std::ffi::{OsStr, OsString};

use clap::{CommandFactory, Parser, error::ErrorKind};
use skilltap_core::{
    domain::{AbsolutePath, ConfiguredBinary, HarnessReachability, NativeId},
    runtime::{
        CommandGitRoot, PlatformPaths, ProcessEnvironment, ScopeResolver, SystemCommandRunner,
        SystemFileSystem, SystemWorkingDirectory,
    },
    storage::{
        ConfigDocument, ConfigRepository, DocumentState, FileConfigRepository,
        FileInventoryRepository, FileStateRepository,
    },
};
use skilltap_harnesses::{HarnessKind, detect_configured_installation, select_profile};

use crate::{
    ErrorDetail, JsonRenderer, NextAction, Outcome, OutputEntry, PlainRenderer, Renderer,
    ResultClass,
    application::{
        NativeLifecycleKind, NativeObservationMode, SkillInstallRequest, StatusApplication,
    },
    command::{
        AdoptArgs, BootstrapArgs, Cli, HarnessChangeArgs, HarnessEnableArgs, OutputArgs, PlanArgs,
        ScopedOutputArgs, ScopedTargetArgs, SyncArgs,
    },
    dispatch::Dispatch,
};

#[path = "daemon_commands.rs"]
mod daemon_commands;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutputChannel {
    Stdout,
    Stderr,
}

#[derive(Debug, Eq, PartialEq)]
pub struct CommandExecution {
    pub document: String,
    pub exit_code: u8,
    pub channel: OutputChannel,
}

pub fn run_from<I, T>(arguments: I) -> CommandExecution
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let arguments = arguments
        .into_iter()
        .map(Into::into)
        .collect::<Vec<OsString>>();
    let json_requested = arguments.iter().any(|value| value == OsStr::new("--json"));
    let dispatch = match Cli::try_parse_from(arguments.clone()) {
        Ok(cli) => Dispatch::from_command(cli.command.expect("Clap requires a subcommand")),
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) =>
        {
            return CommandExecution {
                document: error.to_string(),
                exit_code: 0,
                channel: OutputChannel::Stdout,
            };
        }
        Err(error) => {
            let kind = error.kind();
            let boundary = parse_boundary(&arguments);
            let mut execution = render(
                parse_error(&arguments, kind),
                json_requested,
                OutputChannel::Stderr,
            );
            if kind == ErrorKind::MissingSubcommand && !json_requested {
                execution.document.push('\n');
                let mut command = Cli::command();
                for token in boundary.command.split_whitespace().skip(1) {
                    let Some(next) = command
                        .get_subcommands()
                        .find(|candidate| candidate.get_name() == token)
                        .cloned()
                    else {
                        break;
                    };
                    command = next;
                }
                execution
                    .document
                    .push_str(&command.render_usage().to_string());
                execution.document.push('\n');
            }
            return execution;
        }
    };
    let json = dispatch.json();
    let (outcome, plain_channel) = match dispatch {
        Dispatch::Status(args) => (execute_system_status(&args), OutputChannel::Stdout),
        Dispatch::Adopt(args) => (execute_system_adopt(&args), OutputChannel::Stdout),
        Dispatch::Plan(args) => (execute_system_plan(&args), OutputChannel::Stdout),
        Dispatch::Sync(args) => (execute_system_sync(&args), OutputChannel::Stdout),
        Dispatch::Bootstrap(args) => (execute_system_bootstrap(&args), OutputChannel::Stdout),
        Dispatch::SkillList(args) => (execute_system_skill_list(&args), OutputChannel::Stdout),
        Dispatch::MarketplaceList(args) => (
            execute_system_resource_list(
                "marketplace list",
                &args,
                skilltap_core::domain::ResourceKind::Marketplace,
            ),
            OutputChannel::Stdout,
        ),
        Dispatch::PluginList(args) => (
            execute_system_resource_list(
                "plugin list",
                &args,
                skilltap_core::domain::ResourceKind::Plugin,
            ),
            OutputChannel::Stdout,
        ),
        Dispatch::InstructionStatus(args) => (
            execute_system_instruction_status(&args),
            OutputChannel::Stdout,
        ),
        Dispatch::MarketplaceAdd(args) => (
            execute_system_native_lifecycle(
                "marketplace add",
                NativeLifecycleKind::MarketplaceAdd,
                &args.common.scope,
                &args.common.target,
                Some(args.source.as_str()),
                args.name.as_ref().map(|value| value.as_str()),
            ),
            OutputChannel::Stdout,
        ),
        Dispatch::MarketplaceRemove(args) => (
            execute_system_native_lifecycle(
                "marketplace remove",
                NativeLifecycleKind::MarketplaceRemove,
                &args.common.scope,
                &args.common.target,
                None,
                Some(args.name.as_str()),
            ),
            OutputChannel::Stdout,
        ),
        Dispatch::MarketplaceUpdate(args) => (
            execute_system_native_lifecycle(
                "marketplace update",
                NativeLifecycleKind::MarketplaceUpdate,
                &args.common.scope,
                &args.common.target,
                None,
                args.name.as_ref().map(|value| value.as_str()),
            ),
            OutputChannel::Stdout,
        ),
        Dispatch::PluginInstall(args) => (
            execute_system_native_lifecycle(
                "plugin install",
                NativeLifecycleKind::PluginInstall,
                &args.scope,
                &args.target,
                Some(args.plugin.as_str()),
                None,
            ),
            OutputChannel::Stdout,
        ),
        Dispatch::PluginRemove(args) => (
            execute_system_native_lifecycle(
                "plugin remove",
                NativeLifecycleKind::PluginRemove,
                &args.common.scope,
                &args.common.target,
                None,
                Some(args.plugin.as_str()),
            ),
            OutputChannel::Stdout,
        ),
        Dispatch::PluginUpdate(args) => (
            execute_system_native_lifecycle(
                "plugin update",
                NativeLifecycleKind::PluginUpdate,
                &args.scope,
                &args.target,
                None,
                args.plugin.as_ref().map(|value| value.as_str()),
            ),
            OutputChannel::Stdout,
        ),
        Dispatch::SkillInstall(args) => (
            execute_system_skill_install(
                "skill install",
                &args.scope,
                &args.target,
                args.acknowledgment.yes,
                SkillInstallRequest {
                    source: args.source.as_str(),
                    name: args.name.as_ref().map(|value| value.as_str()),
                    preserve_name: false,
                    requested_revision: args
                        .requested_revision
                        .as_ref()
                        .map(|value| value.as_str()),
                    subdirectory: args.path.as_ref().map(|value| value.as_str()),
                },
            ),
            OutputChannel::Stdout,
        ),
        Dispatch::SkillRemove(args) => (
            execute_system_skill_remove(
                "skill remove",
                &args.common.scope,
                &args.common.target,
                args.skill.as_str(),
                false,
            ),
            OutputChannel::Stdout,
        ),
        Dispatch::SkillUpdate(args) => (
            execute_system_skill_update(
                "skill update",
                &args.scope,
                &args.target,
                args.skill.as_ref().map(|value| value.as_str()),
                args.acknowledgment.yes,
            ),
            OutputChannel::Stdout,
        ),
        Dispatch::InstructionSetup(args) => (
            execute_system_instruction_setup(
                "instructions setup",
                &args.scope,
                args.mode,
                args.acknowledgment.yes,
                false,
            ),
            OutputChannel::Stdout,
        ),
        Dispatch::InstructionRepair(args) => (
            execute_system_instruction_setup(
                "instructions repair",
                &args.scope,
                None,
                args.acknowledgment.yes,
                true,
            ),
            OutputChannel::Stdout,
        ),
        Dispatch::HarnessList(args) => (execute_system_harness_list(&args), OutputChannel::Stdout),
        Dispatch::HarnessEnable(args) => {
            (execute_system_harness_enable(&args), OutputChannel::Stdout)
        }
        Dispatch::HarnessDisable(args) => {
            (execute_system_harness_disable(&args), OutputChannel::Stdout)
        }
        Dispatch::DaemonEnable(args) => (
            daemon_commands::execute_system_daemon_enable(&args),
            OutputChannel::Stdout,
        ),
        Dispatch::DaemonDisable(args) => (
            daemon_commands::execute_system_daemon_disable(&args),
            OutputChannel::Stdout,
        ),
        Dispatch::DaemonStatus(args) => (
            daemon_commands::execute_system_daemon_status(&args),
            OutputChannel::Stdout,
        ),
        Dispatch::DaemonRun(args) => (execute_system_daemon_run(&args), OutputChannel::Stdout),
    };
    render(outcome, json, plain_channel)
}

fn execute_system_plan(args: &PlanArgs) -> Outcome {
    execute_system_reconciliation("plan", |application| application.execute_plan(args))
}

fn execute_system_sync(args: &SyncArgs) -> Outcome {
    execute_system_reconciliation("sync", |application| application.execute_sync(args))
}

fn execute_system_bootstrap(args: &BootstrapArgs) -> Outcome {
    use skilltap_core::domain::{ConfiguredBinary, TargetSelection};
    use skilltap_harnesses::{
        HarnessBootstrapPolicy, HarnessSetupResult, setup_first_party_plugin,
    };

    let paths = match PlatformPaths::resolve(&ProcessEnvironment) {
        Ok(paths) => paths,
        Err(_) => return repository_composition_error("bootstrap"),
    };
    let filesystem = SystemFileSystem;
    let repository = match FileConfigRepository::new(&filesystem, paths.skilltap_config().clone()) {
        Ok(repository) => repository,
        Err(_) => return repository_composition_error("bootstrap"),
    };
    let config = match repository.load() {
        Ok(DocumentState::Missing) => ConfigDocument::defaults(),
        Ok(DocumentState::Present(value)) => value,
        Err(_) => return repository_composition_error("bootstrap"),
    };
    let selected = args.target.clone().unwrap_or(TargetSelection::All);
    let includes = |kind: HarnessKind| match &selected {
        TargetSelection::All => true,
        TargetSelection::Only(target) => target.as_str() == kind.id(),
    };
    let mut outcome = Outcome::new("bootstrap", ResultClass::Completed)
        .with_scope(crate::OutputScope::Global)
        .with_summary("binary", "pending")
        .with_summary("version", skilltap_core::VERSION)
        .with_summary("allow_major", args.allow_major);
    let binary = execute_binary_bootstrap(args, &paths);
    outcome = outcome.with_resource(binary.entry);
    if binary.attention {
        outcome.result = ResultClass::AttentionRequired;
    }
    for warning in binary.warnings {
        outcome = outcome.with_warning(warning);
    }
    for action in binary.next_actions {
        outcome = outcome.with_next_action(action);
    }
    let search_path = std::env::var_os("PATH");
    let process_limits =
        skilltap_core::runtime::ProcessLimits::new(30_000, 64 * 1024, 64 * 1024, 128 * 1024)
            .expect("bootstrap process limits are valid");
    let json_limits = skilltap_core::runtime::JsonLimits::new(128 * 1024, 32)
        .expect("bootstrap JSON limits are valid");
    for (kind, policy) in [
        (HarnessKind::Codex, &config.harnesses().codex),
        (HarnessKind::Claude, &config.harnesses().claude),
    ] {
        if !includes(kind) {
            continue;
        }
        let configured = if std::path::Path::new(policy.binary.as_str()).is_absolute() {
            match AbsolutePath::new(policy.binary.as_str()) {
                Ok(path) => ConfiguredBinary::absolute(path),
                Err(_) => {
                    outcome.result = ResultClass::AttentionRequired;
                    outcome = outcome.with_resource(OutputEntry::new(kind.id(), "invalid"));
                    continue;
                }
            }
        } else {
            match NativeId::new(policy.binary.as_str()).and_then(ConfiguredBinary::path_lookup) {
                Ok(binary) => binary,
                Err(_) => {
                    outcome.result = ResultClass::AttentionRequired;
                    outcome = outcome.with_resource(OutputEntry::new(kind.id(), "invalid"));
                    continue;
                }
            }
        };
        let bootstrap_policy = HarnessBootstrapPolicy {
            configured,
            search_path: search_path.clone(),
            process_limits,
            json_limits,
            plugin_name: NativeId::new("skilltap").expect("canonical plugin id is valid"),
            canonical_source: Some(
                skilltap_core::domain::SourceLocator::new("https://github.com/nklisch/skilltap")
                    .expect("canonical source is valid"),
            ),
        };
        let result = setup_first_party_plugin(kind, &bootstrap_policy);
        let (status, attention, next_action) = match &result {
            HarnessSetupResult::Installed { .. } => ("installed", false, None),
            HarnessSetupResult::AlreadyPresent { .. } => ("already-present", false, None),
            HarnessSetupResult::Unavailable { reason, .. } => {
                ("unavailable", true, Some(reason.to_string()))
            }
            HarnessSetupResult::Unsupported { next_action, .. } => {
                ("unsupported", true, Some(next_action.clone()))
            }
            HarnessSetupResult::Failed { reason, .. } => ("failed", true, Some(reason.to_string())),
        };
        outcome = outcome.with_resource(OutputEntry::new(kind.id(), status));
        if let Some(next_action) = next_action {
            outcome = outcome.with_next_action(NextAction::new(
                format!("bootstrap_{}", kind.id()),
                next_action,
            ));
        }
        if attention {
            outcome.result = ResultClass::AttentionRequired;
        }
    }
    outcome
}

struct BinaryBootstrapResult {
    entry: OutputEntry,
    attention: bool,
    warnings: Vec<crate::Warning>,
    next_actions: Vec<crate::NextAction>,
}

fn execute_binary_bootstrap(
    args: &BootstrapArgs,
    paths: &skilltap_core::runtime::PlatformPaths,
) -> BinaryBootstrapResult {
    use skilltap_core::{
        bootstrap::{ArtifactKey, BinaryDecision, ReleaseVersion, choose_binary_decision},
        runtime::{
            ArtifactError, ArtifactFetcher, BinaryInstaller, FileReleaseResolver, ReleaseResolver,
            SystemArtifactFetcher, SystemBinaryInstaller, SystemReleaseResolver,
        },
    };
    let key = match ArtifactKey::current() {
        Ok(key) => key,
        Err(_) => {
            return binary_attention(
                "unsupported-platform",
                "This platform has no published skilltap bootstrap artifact.",
            );
        }
    };
    let destination = match std::env::var_os("SKILLTAP_INSTALL")
        .and_then(|value| value.into_string().ok())
        .map(AbsolutePath::new)
        .transpose()
    {
        Ok(Some(path)) => path,
        Ok(None) => {
            match AbsolutePath::new(format!("{}/.local/bin/skilltap", paths.home().as_str())) {
                Ok(path) => path,
                Err(_) => {
                    return binary_attention(
                        "invalid-destination",
                        "The user-local skilltap install destination is invalid.",
                    );
                }
            }
        }
        Err(_) => {
            return binary_attention(
                "invalid-destination",
                "SKILLTAP_INSTALL must be a normalized absolute path.",
            );
        }
    };
    let manifest_path =
        std::env::var_os("SKILLTAP_RELEASE_MANIFEST").and_then(|value| value.into_string().ok());
    let manifest = match manifest_path {
        Some(path) => match AbsolutePath::new(path) {
            Ok(path) => FileReleaseResolver::new(path, key).latest(),
            Err(_) => Err(ArtifactError::InvalidManifest(
                "release manifest path is invalid",
            )),
        },
        None => SystemReleaseResolver::current(key).latest(),
    };
    let manifest = match manifest {
        Ok(manifest) => manifest,
        Err(error) => return binary_attention("release_manifest_failed", &error.to_string()),
    };
    let artifact = match manifest.artifact(key) {
        Ok(artifact) => artifact,
        Err(error) => return binary_attention("release_asset_failed", &error.to_string()),
    };
    let installer = SystemBinaryInstaller;
    let installed = match installer.inspect(&destination) {
        Ok(value) => value,
        Err(error) => return binary_attention("binary_inspection_failed", &error.to_string()),
    };
    let installed_version = installed.as_ref().and_then(|_| {
        std::env::var("SKILLTAP_INSTALLED_VERSION")
            .ok()
            .and_then(|value| value.trim_start_matches('v').parse::<ReleaseVersion>().ok())
            .or_else(|| probe_installed_version(&destination))
    });
    if installed.is_some() && installed_version.is_none() {
        return binary_attention(
            "unknown_version",
            "The existing skilltap executable version could not be verified; no replacement was attempted.",
        );
    }
    let decision = choose_binary_decision(
        installed_version.as_ref(),
        &manifest.version,
        args.allow_major,
    );
    if decision == BinaryDecision::MajorUpgradeBlocked {
        return BinaryBootstrapResult {
            entry: OutputEntry::new("binary", "major-upgrade-blocked")
                .with_field("available_version", manifest.version.to_string())
                .with_field("path_role", "user-local-bin/skilltap"),
            attention: true,
            warnings: vec![crate::Warning::new(
                "major_upgrade_blocked",
                "A newer major skilltap binary is available; no existing binary was changed.",
            )],
            next_actions: vec![crate::NextAction::new(
                "allow_major",
                "Rerun with --allow-major to accept the major-version consequence.",
            )],
        };
    }
    if decision == BinaryDecision::Noop {
        return BinaryBootstrapResult {
            entry: OutputEntry::new("binary", "no-op")
                .with_field("version", manifest.version.to_string())
                .with_field("path_role", "user-local-bin/skilltap"),
            attention: false,
            warnings: Vec::new(),
            next_actions: Vec::new(),
        };
    }
    let parent = std::path::Path::new(destination.as_str())
        .parent()
        .unwrap_or(std::path::Path::new("/"));
    if std::fs::create_dir_all(parent).is_err() {
        return binary_attention(
            "destination_unavailable",
            "The user-local binary directory could not be created safely.",
        );
    }
    let temporary = match AbsolutePath::new(
        parent
            .join(format!(".skilltap-bootstrap-{}", std::process::id()))
            .to_string_lossy()
            .into_owned(),
    ) {
        Ok(path) => path,
        Err(_) => {
            return binary_attention(
                "temporary_path_failed",
                "The private bootstrap temporary path is invalid.",
            );
        }
    };
    let fetch_result = if let Some(path) =
        std::env::var_os("SKILLTAP_RELEASE_ARTIFACT").and_then(|value| value.into_string().ok())
    {
        std::fs::copy(path, temporary.as_str())
            .map(|_| ())
            .map_err(|_| ArtifactError::DownloadFailed)
    } else {
        SystemArtifactFetcher.fetch(artifact.download_url().as_str(), &temporary)
    };
    if fetch_result.is_err() {
        let _ = std::fs::remove_file(temporary.as_str());
        return binary_attention(
            "release_download_failed",
            "The release artifact could not be downloaded; the existing binary was preserved.",
        );
    }
    let result = installer.install_verified(&temporary, &destination, artifact);
    let _ = std::fs::remove_file(temporary.as_str());
    if let Err(error) = result {
        return binary_attention("binary_install_failed", &error.to_string());
    }
    BinaryBootstrapResult {
        entry: OutputEntry::new(
            "binary",
            match decision {
                BinaryDecision::Install => "installed",
                _ => "updated",
            },
        )
        .with_field("version", manifest.version.to_string())
        .with_field("path_role", "user-local-bin/skilltap"),
        attention: false,
        warnings: Vec::new(),
        next_actions: Vec::new(),
    }
}

fn probe_installed_version(
    path: &AbsolutePath,
) -> Option<skilltap_core::bootstrap::ReleaseVersion> {
    use skilltap_core::runtime::{
        ExecutableResolutionRequest, ExecutableResolver, NativeProcessRequest, NativeProcessRunner,
        ProcessLimits, SystemExecutableResolver, SystemNativeProcessRunner,
    };
    let executable = SystemExecutableResolver
        .resolve(&ExecutableResolutionRequest::new(
            skilltap_core::domain::ConfiguredBinary::absolute(path.clone()),
            None,
        ))
        .ok()?;
    let limits = ProcessLimits::new(5_000, 4 * 1024, 4 * 1024, 8 * 1024).ok()?;
    let output = SystemNativeProcessRunner
        .run(&NativeProcessRequest::new(
            executable,
            [std::ffi::OsString::from("--version")],
            std::collections::BTreeMap::new(),
            None,
            limits,
        ))
        .ok()?;
    if !output.status().success() {
        return None;
    }
    let text = String::from_utf8(output.stdout().to_vec()).ok()?;
    text.split_whitespace()
        .find_map(|token| token.trim_start_matches('v').parse().ok())
}

fn binary_attention(code: &str, detail: &str) -> BinaryBootstrapResult {
    BinaryBootstrapResult {
        entry: OutputEntry::new("binary", "unavailable").with_field("policy", "latest-compatible"),
        attention: true,
        warnings: vec![crate::Warning::new(code, detail)],
        next_actions: vec![crate::NextAction::new(
            "bootstrap_help",
            "Run `skilltap bootstrap --help` for the release and platform requirements.",
        )],
    }
}

fn execute_system_daemon_run(_args: &crate::command::OutputArgs) -> Outcome {
    execute_system_reconciliation("daemon run", |application| {
        application.execute_daemon_cycle()
    })
}

fn execute_system_skill_list(args: &ScopedTargetArgs) -> Outcome {
    execute_system_reconciliation("skill list", |application| {
        application.execute_skill_list(args)
    })
}

fn execute_system_resource_list(
    command: &'static str,
    args: &ScopedTargetArgs,
    kind: skilltap_core::domain::ResourceKind,
) -> Outcome {
    execute_system_reconciliation(command, |application| {
        application.execute_resource_list(command, args, kind)
    })
}

fn execute_system_instruction_status(args: &ScopedOutputArgs) -> Outcome {
    execute_system_reconciliation("instructions status", |application| {
        application.execute_instruction_status(args)
    })
}

#[allow(dead_code)]
fn execute_system_lifecycle_preview(
    command: &'static str,
    scope: &crate::command::ScopeArgs,
    target: &crate::command::TargetArgs,
    source: Option<&str>,
    name: Option<&str>,
) -> Outcome {
    execute_system_reconciliation(command, |application| {
        application.execute_lifecycle_preview(
            command,
            scope,
            target,
            skilltap_core::domain::ResourceKind::Plugin,
            source,
            name,
        )
    })
}

fn execute_system_skill_install(
    command: &'static str,
    scope: &crate::command::ScopeArgs,
    target: &crate::command::TargetArgs,
    acknowledged: bool,
    request: SkillInstallRequest<'_>,
) -> Outcome {
    execute_system_reconciliation(command, |application| {
        application.execute_skill_install(command, scope, target, acknowledged, request)
    })
}

fn execute_system_skill_remove(
    command: &'static str,
    scope: &crate::command::ScopeArgs,
    target: &crate::command::TargetArgs,
    skill: &str,
    acknowledged: bool,
) -> Outcome {
    execute_system_reconciliation(command, |application| {
        application.execute_skill_remove(command, scope, target, skill, acknowledged)
    })
}

fn execute_system_skill_update(
    command: &'static str,
    scope: &crate::command::ScopeArgs,
    target: &crate::command::TargetArgs,
    skill: Option<&str>,
    acknowledged: bool,
) -> Outcome {
    execute_system_reconciliation(command, |application| {
        application.execute_skill_update(command, scope, target, skill, acknowledged)
    })
}

fn execute_system_instruction_setup(
    command: &'static str,
    scope: &crate::command::ScopeArgs,
    mode: Option<skilltap_core::storage::ClaudeInstructionMode>,
    acknowledged: bool,
    repair: bool,
) -> Outcome {
    execute_system_reconciliation(command, |application| {
        application.execute_instruction_setup(command, scope, mode, acknowledged, repair)
    })
}

fn execute_system_native_lifecycle(
    command: &'static str,
    kind: NativeLifecycleKind,
    scope: &crate::command::ScopeArgs,
    target: &crate::command::TargetArgs,
    source: Option<&str>,
    name: Option<&str>,
) -> Outcome {
    execute_system_reconciliation(command, |application| {
        application.execute_native_lifecycle(command, kind, scope, target, source, name)
    })
}

fn execute_system_reconciliation(
    command: &'static str,
    execute: impl FnOnce(StatusApplication<'_>) -> Outcome,
) -> Outcome {
    with_system_application(command, repository_composition_error, execute)
}

fn with_system_application(
    command: &'static str,
    paths_error: fn(&'static str) -> Outcome,
    execute: impl FnOnce(StatusApplication<'_>) -> Outcome,
) -> Outcome {
    let paths = match PlatformPaths::resolve(&ProcessEnvironment) {
        Ok(paths) => paths,
        Err(_) => return paths_error(command),
    };
    let filesystem = SystemFileSystem;
    let config = match FileConfigRepository::new(&filesystem, paths.skilltap_config().clone()) {
        Ok(repository) => repository,
        Err(_) => return repository_composition_error(command),
    };
    let inventory = match FileInventoryRepository::new(&filesystem, paths.skilltap_config().clone())
    {
        Ok(repository) => repository,
        Err(_) => return repository_composition_error(command),
    };
    let state = match FileStateRepository::new(&filesystem, paths.skilltap_config().clone()) {
        Ok(repository) => repository,
        Err(_) => return repository_composition_error(command),
    };
    let runner = SystemCommandRunner;
    let git = CommandGitRoot::new(
        &runner,
        NativeId::new("git").expect("known command identifier"),
    );
    let working_directory = SystemWorkingDirectory;
    let scopes = ScopeResolver::new(&filesystem, &working_directory, &git);
    execute(StatusApplication {
        config: &config,
        inventory: &inventory,
        state: &state,
        scopes: &scopes,
        working_directory: &working_directory,
        native_observation: NativeObservationMode::System,
    })
}

fn execute_system_adopt(args: &AdoptArgs) -> Outcome {
    with_system_application("adopt", repository_composition_error, |application| {
        application.execute_adopt(args)
    })
}

fn execute_system_status(args: &crate::command::StatusArgs) -> Outcome {
    with_system_application("status", status_paths_error, |application| {
        application.execute(args)
    })
}

fn status_paths_error(_command: &'static str) -> Outcome {
    Outcome::new("status", ResultClass::Invalid).with_error(ErrorDetail::new(
        "platform_paths_unavailable",
        "The skilltap configuration paths could not be resolved.",
    ))
}

fn with_harness_repository(
    command: &'static str,
    operation: impl FnOnce(&FileConfigRepository<'_>) -> Outcome,
) -> Outcome {
    let paths = match PlatformPaths::resolve(&ProcessEnvironment) {
        Ok(paths) => paths,
        Err(_) => return repository_composition_error(command),
    };
    let filesystem = SystemFileSystem;
    let repository = match FileConfigRepository::new(&filesystem, paths.skilltap_config().clone()) {
        Ok(repository) => repository,
        Err(_) => return repository_composition_error(command),
    };
    operation(&repository)
}

fn execute_system_harness_list(_args: &OutputArgs) -> Outcome {
    with_harness_repository("harness list", |repository| {
        let config = match repository.load() {
            Ok(DocumentState::Missing) => ConfigDocument::defaults(),
            Ok(DocumentState::Present(value)) => value,
            Err(_) => return repository_composition_error("harness list"),
        };
        let paths = match PlatformPaths::resolve(&ProcessEnvironment) {
            Ok(paths) => paths,
            Err(_) => return repository_composition_error("harness list"),
        };
        let process_limits =
            skilltap_core::runtime::ProcessLimits::new(5_000, 256 * 1024, 256 * 1024, 512 * 1024)
                .expect("bounded list process limits are valid");
        let json_limits = skilltap_core::runtime::JsonLimits::new(256 * 1024, 64)
            .expect("bounded list JSON limits are valid");
        let search_path = std::env::var_os("PATH");
        let mut outcome = Outcome::new("harness list", ResultClass::Completed);
        for (id, kind, policy, native_root) in [
            (
                "codex",
                HarnessKind::Codex,
                &config.harnesses().codex,
                paths.codex_home().as_str(),
            ),
            (
                "claude",
                HarnessKind::Claude,
                &config.harnesses().claude,
                paths.claude_home().as_str(),
            ),
        ] {
            let mut entry = OutputEntry::new(
                id,
                if policy.enabled {
                    "enabled"
                } else {
                    "disabled"
                },
            )
            .with_field("enabled", policy.enabled)
            .with_field("binary", policy.binary.as_str())
            .with_field("native_root", native_root);
            let configured = if std::path::Path::new(policy.binary.as_str()).is_absolute() {
                AbsolutePath::new(policy.binary.as_str())
                    .map(ConfiguredBinary::absolute)
                    .map_err(|_| ())
            } else {
                NativeId::new(policy.binary.as_str())
                    .map_err(|_| ())
                    .and_then(|id| ConfiguredBinary::path_lookup(id).map_err(|_| ()))
            };
            match configured.and_then(|configured| {
                detect_configured_installation(
                    kind,
                    configured,
                    search_path.clone(),
                    process_limits,
                    json_limits,
                )
                .map_err(|_| ())
            }) {
                Ok(installation) => {
                    if let HarnessReachability::Reachable { native_version, .. } =
                        installation.reachability()
                    {
                        let profile = select_profile(kind, native_version);
                        entry = entry
                            .with_field("reachable", true)
                            .with_field("version", native_version.as_str())
                            .with_field(
                                "profile_authority",
                                match profile.authority() {
                                    skilltap_core::domain::ProfileAuthority::VerifiedCompiled => {
                                        "verified_compiled"
                                    }
                                    skilltap_core::domain::ProfileAuthority::ObserveOnly => {
                                        "observe_only"
                                    }
                                },
                            );
                        if profile.mutation_capabilities().is_none() {
                            outcome.result = ResultClass::AttentionRequired;
                            outcome = outcome.with_warning(
                                crate::Warning::new(
                                    "harness_profile_observe_only",
                                    "The detected harness version is observable but not mutation-authorized.",
                                )
                                .with_context("harness", id),
                            );
                        }
                    }
                }
                Err(_) => {
                    entry = entry.with_field("reachable", false);
                    outcome.result = ResultClass::AttentionRequired;
                    outcome = outcome.with_warning(
                        crate::Warning::new(
                            "native_detection_failed",
                            "The configured harness could not be detected.",
                        )
                        .with_context("harness", id),
                    );
                }
            }
            outcome = outcome.with_resource(entry);
        }
        outcome
    })
}

fn execute_system_harness_enable(args: &HarnessEnableArgs) -> Outcome {
    execute_harness_change("harness enable", &args.harness, true, args.binary.as_ref())
}

fn execute_system_harness_disable(args: &HarnessChangeArgs) -> Outcome {
    execute_harness_change("harness disable", &args.harness, false, None)
}

fn execute_harness_change(
    command: &'static str,
    harness: &skilltap_core::domain::HarnessId,
    enabled: bool,
    binary: Option<&skilltap_core::storage::HarnessBinary>,
) -> Outcome {
    with_harness_repository(command, |repository| {
        let current = match repository.load() {
            Ok(DocumentState::Missing) => ConfigDocument::defaults(),
            Ok(DocumentState::Present(value)) => value,
            Err(_) => return repository_composition_error(command),
        };
        let next = match current.with_harness_policy(harness, enabled, binary) {
            Ok(value) => value,
            Err(_) => {
                return Outcome::new(command, ResultClass::Invalid).with_error(ErrorDetail::new(
                    "invalid_harness",
                    "The requested harness is not supported.",
                ));
            }
        };
        if !enabled && next == current {
            return Outcome::new(command, ResultClass::Invalid).with_error(ErrorDetail::new(
                "harness_already_disabled",
                "The requested harness is already disabled.",
            ));
        }
        if next == current {
            return Outcome::new(command, ResultClass::Completed).with_resource(OutputEntry::new(
                harness.as_str(),
                if enabled { "enabled" } else { "disabled" },
            ));
        }
        if repository.replace(&next).is_err() {
            return repository_composition_error(command);
        }
        Outcome::new(command, ResultClass::Completed).with_resource(OutputEntry::new(
            harness.as_str(),
            if enabled { "enabled" } else { "disabled" },
        ))
    })
}

fn repository_composition_error(command: &'static str) -> Outcome {
    Outcome::new(command, ResultClass::Invalid).with_error(ErrorDetail::new(
        "storage_unavailable",
        "The skilltap storage repositories could not be composed.",
    ))
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ParseBoundary {
    command: String,
    help_command: String,
}

fn parse_boundary(arguments: &[OsString]) -> ParseBoundary {
    let mut command = Cli::command();
    let mut path = Vec::new();
    let mut tokens = arguments.iter().skip(1).peekable();

    while let Some(token) = tokens.next() {
        let Some(token) = token.to_str() else {
            break;
        };
        if token == "--" {
            break;
        }
        if token.starts_with('-') {
            let argument = command.get_arguments().find(|argument| {
                argument
                    .get_long()
                    .is_some_and(|long| token == format!("--{long}"))
                    || argument
                        .get_short()
                        .is_some_and(|short| token == format!("-{short}"))
            });
            if let Some(argument) = argument
                && argument
                    .get_num_args()
                    .is_some_and(|range| range.takes_values())
                && tokens
                    .peek()
                    .is_some_and(|value| !value.to_string_lossy().starts_with('-'))
            {
                tokens.next();
            } else if argument.is_none() && path.is_empty() {
                break;
            }
            continue;
        }
        let Some(next) = command
            .get_subcommands()
            .find(|candidate| candidate.get_name() == token)
            .cloned()
        else {
            break;
        };
        path.push(token.to_owned());
        command = next;
    }

    let command = if path.is_empty() {
        "skilltap".to_owned()
    } else {
        format!("skilltap {}", path.join(" "))
    };
    ParseBoundary {
        help_command: format!("{command} --help"),
        command,
    }
}

fn parse_error(arguments: &[OsString], kind: ErrorKind) -> Outcome {
    let boundary = parse_boundary(arguments);
    let (code, summary) = match kind {
        ErrorKind::MissingSubcommand => ("missing_command", "A command is required."),
        ErrorKind::InvalidUtf8 => (
            "invalid_utf8_argument",
            "A command argument is not valid UTF-8.",
        ),
        _ => ("invalid_arguments", "The command arguments are invalid."),
    };
    Outcome::new(
        boundary
            .command
            .strip_prefix("skilltap ")
            .unwrap_or(&boundary.command),
        ResultClass::Invalid,
    )
    .with_error(ErrorDetail::new(code, summary).with_context("boundary", boundary.command.clone()))
    .with_next_action(
        NextAction::new("show_help", "Review the command grammar.")
            .with_command(boundary.help_command),
    )
}

fn render(outcome: Outcome, json: bool, plain_channel: OutputChannel) -> CommandExecution {
    let rendered = if json {
        JsonRenderer.render(&outcome)
    } else {
        PlainRenderer.render(&outcome)
    };
    let (document, exit_code) = match rendered {
        Ok(document) => (document, outcome.exit_code()),
        Err(_) if json => (
            r#"{"schema":1,"command":"skilltap","result":"invalid","summary":{},"resources":[],"operations":[],"warnings":[],"errors":[{"code":"output_unavailable","summary":"The command outcome could not be rendered."}],"next_actions":[]}"#
                .to_owned(),
            1,
        ),
        Err(_) => (
            "Error: The command outcome could not be rendered.\nCode: output_unavailable\n\nResult: invalid\n"
                .to_owned(),
            1,
        ),
    };
    CommandExecution {
        document,
        exit_code,
        channel: if json {
            OutputChannel::Stdout
        } else {
            plain_channel
        },
    }
}

#[cfg(test)]
mod tests;
