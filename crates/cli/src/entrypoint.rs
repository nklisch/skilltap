use std::ffi::{OsStr, OsString};

use clap::{CommandFactory, Parser, error::ErrorKind};
use skilltap_core::{
    domain::{AbsolutePath, ConfiguredBinary, HarnessReachability, NativeId},
    runtime::{
        CommandGitRoot, ConfigurationLock, ConfigurationLockGuard, FileSystem, PlatformPaths,
        ProcessEnvironment, ScopeResolver, SystemCommandRunner, SystemConfigurationLock,
        SystemFileSystem, SystemWorkingDirectory,
    },
    storage::{
        ConfigDocument, ConfigRepository, DocumentState, FileConfigRepository,
        FileInventoryRepository, FileStateRepository,
    },
};
use skilltap_harnesses::{HarnessKind, detect_configured_installation, select_profile};

use crate::{
    ErrorDetail, JsonRenderer, NextAction, Outcome, OutputEntry, OutputScope, PlainRenderer,
    Renderer, ResultClass,
    application::{
        NativeLifecycleKind, NativeObservationMode, SkillInstallRequest, StatusApplication,
        detection_diagnostic,
    },
    command::{
        AdoptArgs, BootstrapArgs, Cli, HarnessChangeArgs, HarnessEnableArgs, OutputArgs, PlanArgs,
        ScopedOutputArgs, ScopedTargetArgs, SyncArgs,
    },
    dispatch::Dispatch,
};

#[path = "bootstrap_commands.rs"]
mod bootstrap_commands;
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
        Dispatch::Bootstrap(args) => (
            bootstrap_commands::execute_system_bootstrap(&args),
            OutputChannel::Stdout,
        ),
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
                false,
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
                false,
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
                false,
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
                args.acknowledgment.yes,
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
                false,
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
                args.acknowledgment.yes,
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

fn execute_system_daemon_run(_args: &crate::command::OutputArgs) -> Outcome {
    let binary = bootstrap_commands::execute_system_daemon_binary_policy();
    execute_system_reconciliation("daemon run", |application| {
        application.execute_daemon_cycle_with_binary(Some(binary))
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
    acknowledged: bool,
) -> Outcome {
    execute_system_reconciliation(command, |application| {
        application.execute_native_lifecycle(
            command,
            kind,
            scope,
            target,
            crate::application::NativeLifecycleValues { source, name },
            acknowledged,
        )
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
        let native_environment = match paths.native_process_environment(search_path.clone()) {
            Ok(environment) => environment,
            Err(_) => return repository_composition_error("harness list"),
        };
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
            let configured = match configured {
                Ok(configured) => configured,
                Err(()) => {
                    entry = entry.with_field("reachable", false);
                    outcome.result = ResultClass::AttentionRequired;
                    outcome = outcome.with_warning(
                        crate::Warning::new(
                            "invalid_harness_binary",
                            "The configured harness binary is invalid.",
                        )
                        .with_context("harness", id),
                    );
                    outcome = outcome.with_resource(entry);
                    continue;
                }
            };
            match detect_configured_installation(
                kind,
                configured,
                search_path.clone(),
                &native_environment,
                process_limits,
                json_limits,
            ) {
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
                Err(error) => {
                    entry = entry.with_field("reachable", false);
                    outcome.result = ResultClass::AttentionRequired;
                    let diagnostic = detection_diagnostic(&error, id);
                    outcome = outcome
                        .with_warning(diagnostic.warning)
                        .with_next_action(diagnostic.next_action);
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
