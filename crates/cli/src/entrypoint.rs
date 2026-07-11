use std::{
    collections::BTreeMap,
    ffi::{OsStr, OsString},
};

use clap::{CommandFactory, Parser, error::ErrorKind};
use skilltap_core::{
    domain::{AbsolutePath, ConfiguredBinary, HarnessReachability, NativeId},
    runtime::{
        CommandGitRoot, ExecutableResolutionRequest, ExecutableResolver, FileKind, FileSystem,
        NativeProcessRequest, NativeProcessRunner, PlatformPaths, ProcessEnvironment,
        ProcessLimits, ScopeResolver, SystemCommandRunner, SystemExecutableResolver,
        SystemFileSystem, SystemNativeProcessRunner, SystemWorkingDirectory,
    },
    storage::{
        ConfigDocument, ConfigRepository, DocumentState, FileConfigRepository,
        FileInventoryRepository, FileStateRepository,
    },
};
use skilltap_harnesses::{HarnessKind, detect_configured_installation, select_profile};

use crate::{
    ErrorDetail, JsonRenderer, NextAction, Outcome, OutputEntry, PlainRenderer, Renderer,
    ResultClass, Warning,
    application::{
        NativeLifecycleKind, NativeObservationMode, SkillInstallRequest, StatusApplication,
    },
    command::{
        AdoptArgs, Cli, HarnessChangeArgs, HarnessEnableArgs, OutputArgs, PlanArgs,
        ScopedOutputArgs, ScopedTargetArgs, SyncArgs,
    },
    dispatch::Dispatch,
};

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
    let dispatch = match Cli::try_parse_from(arguments) {
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
            let mut execution = render(parse_error(kind), json_requested, OutputChannel::Stderr);
            if kind == ErrorKind::MissingSubcommand && !json_requested {
                execution.document.push('\n');
                execution
                    .document
                    .push_str(&Cli::command().render_usage().to_string());
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
                SkillInstallRequest {
                    source: args.source.as_str(),
                    name: args.name.as_ref().map(|value| value.as_str()),
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
        Dispatch::DaemonEnable(args) => {
            (execute_system_daemon_enable(&args), OutputChannel::Stdout)
        }
        Dispatch::DaemonDisable(args) => {
            (execute_system_daemon_disable(&args), OutputChannel::Stdout)
        }
        Dispatch::DaemonStatus(args) => {
            (execute_system_daemon_status(&args), OutputChannel::Stdout)
        }
        Dispatch::DaemonRun => (execute_system_daemon_run(), OutputChannel::Stdout),
    };
    render(outcome, json, plain_channel)
}

fn execute_system_plan(args: &PlanArgs) -> Outcome {
    execute_system_reconciliation("plan", |application| application.execute_plan(args))
}

fn execute_system_sync(args: &SyncArgs) -> Outcome {
    execute_system_reconciliation("sync", |application| application.execute_sync(args))
}

fn execute_system_daemon_run() -> Outcome {
    execute_system_reconciliation("daemon run", |application| {
        application.execute_daemon_cycle()
    })
}

fn execute_system_daemon_enable(args: &crate::command::DaemonEnableArgs) -> Outcome {
    let command = "daemon enable";
    let paths = match PlatformPaths::resolve(&ProcessEnvironment) {
        Ok(paths) => paths,
        Err(_) => return repository_composition_error(command),
    };
    let filesystem = SystemFileSystem;
    let config = match FileConfigRepository::new(&filesystem, paths.skilltap_config().clone())
        .and_then(|repository| match repository.load() {
            Ok(DocumentState::Present(config)) => Ok(config),
            Ok(DocumentState::Missing) => Ok(ConfigDocument::defaults()),
            Err(error) => Err(error),
        }) {
        Ok(config) => config,
        Err(_) => return repository_composition_error(command),
    };
    let interval = args.interval.unwrap_or(config.updates().interval);
    let platform = crate::daemon::platform(&paths);
    let executable = match std::env::current_exe()
        .ok()
        .and_then(|path| path.to_str().map(str::to_owned))
        .and_then(|path| AbsolutePath::new(path).ok())
    {
        Some(path) => path,
        None => {
            return Outcome::new(command, ResultClass::Invalid).with_error(ErrorDetail::new(
                "daemon_executable_unavailable",
                "The skilltap executable path could not be represented safely.",
            ));
        }
    };
    let definition =
        match skilltap_core::daemon::render_service(&skilltap_core::daemon::DaemonServiceSpec {
            platform,
            interval,
            executable,
        }) {
            Ok(definition) => definition,
            Err(_) => {
                return Outcome::new(command, ResultClass::Invalid).with_error(ErrorDetail::new(
                    "daemon_definition_invalid",
                    "The daemon service definition could not be validated.",
                ));
            }
        };
    let files = crate::daemon::files(&paths, &definition);
    let root = crate::daemon::root(&paths, platform);
    if let Err(error) = filesystem.create_directory_all(&root) {
        return Outcome::new(command, ResultClass::AttentionRequired).with_warning(
            Warning::new(
                "daemon_definition_write_failed",
                "The daemon service directory could not be created.",
            )
            .with_context("path", root.as_str())
            .with_context("detail", error.to_string()),
        );
    }
    for (path, file) in &files {
        match filesystem.read_regular_no_follow(path) {
            Ok(Some(existing)) if !crate::daemon::owns(platform, &existing) => {
                return Outcome::new(command, ResultClass::AttentionRequired)
                    .with_resource(OutputEntry::new(path.as_str(), "conflict"))
                    .with_warning(Warning::new(
                        "daemon_definition_conflict",
                        "An unmanaged service definition already occupies the skilltap path.",
                    ));
            }
            Ok(_) => {}
            Err(_) => {
                return Outcome::new(command, ResultClass::AttentionRequired).with_warning(
                    Warning::new(
                        "daemon_definition_unreadable",
                        "The existing daemon service definition could not be read safely.",
                    )
                    .with_context("path", path.as_str()),
                );
            }
        }
        if let Err(error) = filesystem.atomic_write(path, file.contents().as_bytes()) {
            return Outcome::new(command, ResultClass::AttentionRequired).with_warning(
                Warning::new(
                    "daemon_definition_write_failed",
                    "The daemon service definition could not be published atomically.",
                )
                .with_context("path", path.as_str())
                .with_context("detail", error.to_string()),
            );
        }
    }
    if run_service_manager(platform, ServiceManagerAction::Enable, &files[0].0).is_err() {
        return Outcome::new(command, ResultClass::AttentionRequired)
            .with_warning(Warning::new(
                "daemon_manager_unavailable",
                "The service definition was written, but the user service manager did not activate it.",
            ))
            .with_next_action(NextAction::new(
                "retry_daemon_enable",
                "Retry daemon enable after checking the user service manager.",
            ))
            .with_resource(OutputEntry::new(files[0].0.as_str(), "installed"));
    }
    Outcome::new(command, ResultClass::Completed)
        .with_resource(
            OutputEntry::new(files[0].0.as_str(), "enabled")
                .with_field("interval", interval.to_string())
                .with_field("platform", format!("{platform:?}").to_lowercase()),
        )
        .with_summary("changed", true)
}

fn execute_system_daemon_disable(_args: &OutputArgs) -> Outcome {
    let command = "daemon disable";
    let paths = match PlatformPaths::resolve(&ProcessEnvironment) {
        Ok(paths) => paths,
        Err(_) => return repository_composition_error(command),
    };
    let platform = crate::daemon::platform(&paths);
    let root = crate::daemon::root(&paths, platform);
    let names = match platform {
        skilltap_core::daemon::ServicePlatform::Launchd => {
            vec![format!("{}.plist", skilltap_core::daemon::SERVICE_LABEL)]
        }
        skilltap_core::daemon::ServicePlatform::SystemdUser => vec![
            skilltap_core::daemon::SYSTEMD_UNIT.to_owned(),
            skilltap_core::daemon::SYSTEMD_TIMER.to_owned(),
        ],
    };
    let files = names
        .iter()
        .map(|name| AbsolutePath::new(format!("{}/{}", root.as_str(), name)).unwrap())
        .collect::<Vec<_>>();
    for path in &files {
        match SystemFileSystem.read_regular_no_follow(path) {
            Ok(Some(contents)) if !crate::daemon::owns(platform, &contents) => {
                return Outcome::new(command, ResultClass::AttentionRequired).with_warning(
                    Warning::new(
                        "daemon_definition_conflict",
                        "An unmanaged service definition occupies the skilltap path; it was not removed.",
                    )
                    .with_context("path", path.as_str()),
                );
            }
            Ok(_) => {}
            Err(_) => {
                return Outcome::new(command, ResultClass::AttentionRequired).with_warning(
                    Warning::new(
                        "daemon_definition_unreadable",
                        "The daemon service definition could not be inspected safely.",
                    )
                    .with_context("path", path.as_str()),
                );
            }
        }
    }
    if run_service_manager(platform, ServiceManagerAction::Disable, &files[0]).is_err() {
        return Outcome::new(command, ResultClass::AttentionRequired).with_warning(Warning::new(
            "daemon_manager_unavailable",
            "The user service manager did not disable the daemon; owned definitions were retained.",
        ));
    }
    let mut changed = false;
    for path in &files {
        if let Ok(metadata) = SystemFileSystem.inspect(path)
            && metadata.kind() != FileKind::Missing
        {
            if SystemFileSystem.remove(path).is_err() {
                return Outcome::new(command, ResultClass::AttentionRequired).with_warning(
                    Warning::new(
                        "daemon_definition_remove_failed",
                        "The owned daemon service definition could not be removed safely.",
                    )
                    .with_context("path", path.as_str()),
                );
            }
            changed = true;
        }
    }
    Outcome::new(command, ResultClass::Completed)
        .with_resource(OutputEntry::new(root.as_str(), "disabled"))
        .with_summary("changed", changed)
}

fn execute_system_daemon_status(_args: &OutputArgs) -> Outcome {
    let command = "daemon status";
    let paths = match PlatformPaths::resolve(&ProcessEnvironment) {
        Ok(paths) => paths,
        Err(_) => return repository_composition_error(command),
    };
    let platform = crate::daemon::platform(&paths);
    let root = crate::daemon::root(&paths, platform);
    let names = match platform {
        skilltap_core::daemon::ServicePlatform::Launchd => {
            vec![format!("{}.plist", skilltap_core::daemon::SERVICE_LABEL)]
        }
        skilltap_core::daemon::ServicePlatform::SystemdUser => vec![
            skilltap_core::daemon::SYSTEMD_UNIT.to_owned(),
            skilltap_core::daemon::SYSTEMD_TIMER.to_owned(),
        ],
    };
    let mut installed = true;
    for name in &names {
        let path = AbsolutePath::new(format!("{}/{}", root.as_str(), name)).unwrap();
        match SystemFileSystem.read_regular_no_follow(&path) {
            Ok(Some(contents)) if crate::daemon::owns(platform, &contents) => {}
            Ok(None) => installed = false,
            Ok(Some(_)) => {
                return Outcome::new(command, ResultClass::AttentionRequired)
                    .with_resource(OutputEntry::new(path.as_str(), "conflict"))
                    .with_warning(Warning::new(
                        "daemon_definition_conflict",
                        "An unmanaged service definition occupies the skilltap path.",
                    ));
            }
            Err(_) => installed = false,
        }
    }
    if !installed {
        return Outcome::new(command, ResultClass::Completed)
            .with_resource(OutputEntry::new(root.as_str(), "disabled"));
    }
    let manager = run_service_manager(
        platform,
        ServiceManagerAction::Status,
        &AbsolutePath::new(format!("{}/{}", root.as_str(), names[0])).unwrap(),
    );
    if manager.is_err() {
        return Outcome::new(command, ResultClass::AttentionRequired)
            .with_resource(OutputEntry::new(root.as_str(), "installed"))
            .with_warning(Warning::new(
                "daemon_manager_unavailable",
                "The owned daemon definition exists, but manager state could not be confirmed.",
            ));
    }
    Outcome::new(command, ResultClass::Completed)
        .with_resource(OutputEntry::new(root.as_str(), "enabled"))
}

#[derive(Clone, Copy)]
enum ServiceManagerAction {
    Enable,
    Disable,
    Status,
}

fn run_service_manager(
    platform: skilltap_core::daemon::ServicePlatform,
    action: ServiceManagerAction,
    definition: &AbsolutePath,
) -> Result<(), ()> {
    let (binary, arguments) = match (platform, action) {
        (skilltap_core::daemon::ServicePlatform::Launchd, ServiceManagerAction::Enable) => {
            ("launchctl", vec!["load", "-w", definition.as_str()])
        }
        (skilltap_core::daemon::ServicePlatform::Launchd, ServiceManagerAction::Disable) => {
            ("launchctl", vec!["unload", "-w", definition.as_str()])
        }
        (skilltap_core::daemon::ServicePlatform::Launchd, ServiceManagerAction::Status) => (
            "launchctl",
            vec!["list", skilltap_core::daemon::SERVICE_LABEL],
        ),
        (skilltap_core::daemon::ServicePlatform::SystemdUser, ServiceManagerAction::Enable) => (
            "systemctl",
            vec![
                "--user",
                "enable",
                "--now",
                skilltap_core::daemon::SYSTEMD_TIMER,
            ],
        ),
        (skilltap_core::daemon::ServicePlatform::SystemdUser, ServiceManagerAction::Disable) => (
            "systemctl",
            vec![
                "--user",
                "disable",
                "--now",
                skilltap_core::daemon::SYSTEMD_TIMER,
            ],
        ),
        (skilltap_core::daemon::ServicePlatform::SystemdUser, ServiceManagerAction::Status) => (
            "systemctl",
            vec!["--user", "is-enabled", skilltap_core::daemon::SYSTEMD_TIMER],
        ),
    };
    let configured =
        ConfiguredBinary::path_lookup(NativeId::new(binary).map_err(|_| ())?).map_err(|_| ())?;
    let executable = SystemExecutableResolver
        .resolve(&ExecutableResolutionRequest::new(
            configured,
            std::env::var_os("PATH"),
        ))
        .map_err(|_| ())?;
    let limits = ProcessLimits::new(5_000, 64 * 1024, 64 * 1024, 128 * 1024).map_err(|_| ())?;
    let request = NativeProcessRequest::new(
        executable,
        arguments.into_iter().map(OsString::from),
        BTreeMap::new(),
        None,
        limits,
    );
    let output = SystemNativeProcessRunner.run(&request).map_err(|_| ())?;
    output.status().success().then_some(()).ok_or(())
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
        application.execute_lifecycle_preview(command, scope, target, source, name)
    })
}

fn execute_system_skill_install(
    command: &'static str,
    scope: &crate::command::ScopeArgs,
    target: &crate::command::TargetArgs,
    request: SkillInstallRequest<'_>,
) -> Outcome {
    execute_system_reconciliation(command, |application| {
        application.execute_skill_install(command, scope, target, request)
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
) -> Outcome {
    execute_system_reconciliation(command, |application| {
        application.execute_skill_update(command, scope, target, skill)
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
    let paths = match PlatformPaths::resolve(&ProcessEnvironment) {
        Ok(paths) => paths,
        Err(_) => return repository_composition_error(command),
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
    let paths = match PlatformPaths::resolve(&ProcessEnvironment) {
        Ok(paths) => paths,
        Err(_) => return repository_composition_error("adopt"),
    };
    let filesystem = SystemFileSystem;
    let config = match FileConfigRepository::new(&filesystem, paths.skilltap_config().clone()) {
        Ok(repository) => repository,
        Err(_) => return repository_composition_error("adopt"),
    };
    let inventory = match FileInventoryRepository::new(&filesystem, paths.skilltap_config().clone())
    {
        Ok(repository) => repository,
        Err(_) => return repository_composition_error("adopt"),
    };
    let state = match FileStateRepository::new(&filesystem, paths.skilltap_config().clone()) {
        Ok(repository) => repository,
        Err(_) => return repository_composition_error("adopt"),
    };
    let runner = SystemCommandRunner;
    let git = CommandGitRoot::new(
        &runner,
        NativeId::new("git").expect("known command identifier"),
    );
    let working_directory = SystemWorkingDirectory;
    let scopes = ScopeResolver::new(&filesystem, &working_directory, &git);
    StatusApplication {
        config: &config,
        inventory: &inventory,
        state: &state,
        scopes: &scopes,
        working_directory: &working_directory,
        native_observation: NativeObservationMode::System,
    }
    .execute_adopt(args)
}

fn execute_system_status(args: &crate::command::StatusArgs) -> Outcome {
    let paths = match PlatformPaths::resolve(&ProcessEnvironment) {
        Ok(paths) => paths,
        Err(_) => {
            return Outcome::new("status", ResultClass::Invalid).with_error(ErrorDetail::new(
                "platform_paths_unavailable",
                "The skilltap configuration paths could not be resolved.",
            ));
        }
    };
    let filesystem = SystemFileSystem;
    let config = match FileConfigRepository::new(&filesystem, paths.skilltap_config().clone()) {
        Ok(repository) => repository,
        Err(_) => return repository_composition_error("status"),
    };
    let inventory = match FileInventoryRepository::new(&filesystem, paths.skilltap_config().clone())
    {
        Ok(repository) => repository,
        Err(_) => return repository_composition_error("status"),
    };
    let state = match FileStateRepository::new(&filesystem, paths.skilltap_config().clone()) {
        Ok(repository) => repository,
        Err(_) => return repository_composition_error("status"),
    };
    let runner = SystemCommandRunner;
    let git = CommandGitRoot::new(
        &runner,
        NativeId::new("git").expect("known command identifier"),
    );
    let working_directory = SystemWorkingDirectory;
    let scopes = ScopeResolver::new(&filesystem, &working_directory, &git);
    StatusApplication {
        config: &config,
        inventory: &inventory,
        state: &state,
        scopes: &scopes,
        working_directory: &working_directory,
        native_observation: NativeObservationMode::System,
    }
    .execute(args)
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

fn parse_error(kind: ErrorKind) -> Outcome {
    let (code, summary) = match kind {
        ErrorKind::MissingSubcommand => ("missing_command", "A command is required."),
        ErrorKind::InvalidUtf8 => (
            "invalid_utf8_argument",
            "A command argument is not valid UTF-8.",
        ),
        _ => ("invalid_arguments", "The command arguments are invalid."),
    };
    Outcome::new("skilltap", ResultClass::Invalid)
        .with_error(ErrorDetail::new(code, summary))
        .with_next_action(
            NextAction::new("show_help", "Review the command grammar.")
                .with_command("skilltap --help"),
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
