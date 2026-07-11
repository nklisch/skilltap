use std::ffi::{OsStr, OsString};

use clap::{CommandFactory, Parser, error::ErrorKind};
use skilltap_core::{
    domain::NativeId,
    runtime::{
        CommandGitRoot, PlatformPaths, ProcessEnvironment, ScopeResolver, SystemCommandRunner,
        SystemFileSystem, SystemWorkingDirectory,
    },
    storage::{
        ConfigDocument, ConfigRepository, DocumentState, FileConfigRepository,
        FileInventoryRepository, FileStateRepository,
    },
};

use crate::{
    ErrorDetail, JsonRenderer, NextAction, Outcome, OutputEntry, PlainRenderer, Renderer,
    ResultClass,
    application::StatusApplication,
    command::{Cli, HarnessChangeArgs, HarnessEnableArgs, OutputArgs},
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
        Dispatch::HarnessList(args) => (execute_system_harness_list(&args), OutputChannel::Stdout),
        Dispatch::HarnessEnable(args) => {
            (execute_system_harness_enable(&args), OutputChannel::Stdout)
        }
        Dispatch::HarnessDisable(args) => {
            (execute_system_harness_disable(&args), OutputChannel::Stdout)
        }
        Dispatch::Unavailable { command, .. } => {
            (capability_unavailable(command), OutputChannel::Stderr)
        }
    };
    render(outcome, json, plain_channel)
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
        Err(_) => return repository_composition_error(),
    };
    let inventory = match FileInventoryRepository::new(&filesystem, paths.skilltap_config().clone())
    {
        Ok(repository) => repository,
        Err(_) => return repository_composition_error(),
    };
    let state = match FileStateRepository::new(&filesystem, paths.skilltap_config().clone()) {
        Ok(repository) => repository,
        Err(_) => return repository_composition_error(),
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
    }
    .execute(args)
}

fn with_harness_repository(
    operation: impl FnOnce(&FileConfigRepository<'_>) -> Outcome,
) -> Outcome {
    let paths = match PlatformPaths::resolve(&ProcessEnvironment) {
        Ok(paths) => paths,
        Err(_) => return repository_composition_error(),
    };
    let filesystem = SystemFileSystem;
    let repository = match FileConfigRepository::new(&filesystem, paths.skilltap_config().clone()) {
        Ok(repository) => repository,
        Err(_) => return repository_composition_error(),
    };
    operation(&repository)
}

fn execute_system_harness_list(_args: &OutputArgs) -> Outcome {
    with_harness_repository(|repository| {
        let config = match repository.load() {
            Ok(DocumentState::Missing) => ConfigDocument::defaults(),
            Ok(DocumentState::Present(value)) => value,
            Err(_) => return repository_composition_error(),
        };
        Outcome::new("harness list", ResultClass::Completed)
            .with_resource(OutputEntry::new(
                "codex",
                if config.harnesses().codex.enabled {
                    "enabled"
                } else {
                    "disabled"
                },
            ))
            .with_resource(OutputEntry::new(
                "claude",
                if config.harnesses().claude.enabled {
                    "enabled"
                } else {
                    "disabled"
                },
            ))
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
    with_harness_repository(|repository| {
        let current = match repository.load() {
            Ok(DocumentState::Missing) => ConfigDocument::defaults(),
            Ok(DocumentState::Present(value)) => value,
            Err(_) => return repository_composition_error(),
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
        if next == current {
            return Outcome::new(command, ResultClass::Completed).with_resource(OutputEntry::new(
                harness.as_str(),
                if enabled { "enabled" } else { "disabled" },
            ));
        }
        if repository.replace(&next).is_err() {
            return repository_composition_error();
        }
        Outcome::new(command, ResultClass::Completed).with_resource(OutputEntry::new(
            harness.as_str(),
            if enabled { "enabled" } else { "disabled" },
        ))
    })
}

fn repository_composition_error() -> Outcome {
    Outcome::new("status", ResultClass::Invalid).with_error(ErrorDetail::new(
        "storage_unavailable",
        "The skilltap storage repositories could not be composed.",
    ))
}

fn capability_unavailable(command: &str) -> Outcome {
    Outcome::new(command, ResultClass::Invalid)
        .with_error(ErrorDetail::new(
            "capability_unavailable",
            "This command capability is not available in this build.",
        ))
        .with_next_action(NextAction::new(
            "retry_after_capability_available",
            "Retry when this command capability is available.",
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
