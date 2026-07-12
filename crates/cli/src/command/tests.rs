use clap::{CommandFactory, Parser, error::ErrorKind};

use super::*;

fn parse(args: &[&str]) -> Cli {
    Cli::try_parse_from(args).unwrap()
}

fn rejects(args: &[&str], kind: ErrorKind) {
    assert_eq!(Cli::try_parse_from(args).unwrap_err().kind(), kind);
}

fn assert_native_ids(_: &[NativeId]) {}

#[test]
fn command_tree_matches_the_documented_v3_surface() {
    let root = Cli::command();
    let root_names = root
        .get_subcommands()
        .map(|command| command.get_name())
        .collect::<Vec<_>>();
    assert_eq!(
        root_names,
        [
            "harness",
            "adopt",
            "status",
            "plan",
            "sync",
            "marketplace",
            "plugin",
            "skill",
            "instructions",
            "daemon"
        ]
    );

    for (parent, expected) in [
        ("harness", &["list", "enable", "disable"][..]),
        ("marketplace", &["add", "remove", "update", "list"][..]),
        ("plugin", &["install", "remove", "update", "list"][..]),
        ("skill", &["install", "remove", "update", "list"][..]),
        ("instructions", &["setup", "status", "repair"][..]),
        ("daemon", &["enable", "disable", "status", "run"][..]),
    ] {
        let names = root
            .get_subcommands()
            .find(|command| command.get_name() == parent)
            .unwrap()
            .get_subcommands()
            .map(|command| command.get_name())
            .collect::<Vec<_>>();
        assert_eq!(names, expected, "{parent}");
    }
}

#[test]
fn no_subcommand_is_an_input_error_with_usage() {
    let error = Cli::try_parse_from(["skilltap"]).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::MissingSubcommand);
    assert!(error.to_string().contains("Usage: skilltap <COMMAND>"));
}

#[test]
fn help_and_version_are_returned_without_process_exit() {
    for (args, kind) in [
        (&["skilltap", "--help"][..], ErrorKind::DisplayHelp),
        (&["skilltap", "--version"][..], ErrorKind::DisplayVersion),
        (
            &["skilltap", "plugin", "install", "--help"][..],
            ErrorKind::DisplayHelp,
        ),
    ] {
        rejects(args, kind);
    }
}

#[test]
fn project_distinguishes_absent_current_and_explicit_path() {
    let Command::Status(global) = parse(&["skilltap", "status"]).command.unwrap() else {
        panic!("expected status")
    };
    assert_eq!(global.scope.argument(), ScopeArgument::Global);

    let Command::Status(current) = parse(&["skilltap", "status", "--project"]).command.unwrap()
    else {
        panic!("expected status")
    };
    assert_eq!(current.scope.argument(), ScopeArgument::Project(None));

    let Command::Status(explicit) = parse(&["skilltap", "status", "--project", "../workspace"])
        .command
        .unwrap()
    else {
        panic!("expected status")
    };
    assert_eq!(
        explicit.scope.argument(),
        ScopeArgument::Project(Some(PathBuf::from("../workspace")))
    );
}

#[test]
fn project_does_not_consume_the_following_option() {
    let Command::Status(args) = parse(&["skilltap", "status", "--project", "--target", "claude"])
        .command
        .unwrap()
    else {
        panic!("expected status")
    };
    assert_eq!(args.scope.argument(), ScopeArgument::Project(None));
    assert_eq!(
        args.target.target,
        Some(TargetSelection::Only(HarnessId::new("claude").unwrap()))
    );
}

#[test]
fn project_and_all_scopes_conflict_in_either_order() {
    for args in [
        &["skilltap", "status", "--project", "--all-scopes"][..],
        &["skilltap", "status", "--all-scopes", "--project"][..],
    ] {
        rejects(args, ErrorKind::ArgumentConflict);
    }
}

#[test]
fn explicit_project_path_is_validated_at_the_boundary() {
    rejects(
        &["skilltap", "status", "--project", ""],
        ErrorKind::ValueValidation,
    );
    rejects(
        &["skilltap", "status", "--project", "bad\npath"],
        ErrorKind::ValueValidation,
    );
}

#[test]
fn targets_are_restricted_and_converted_to_core_types() {
    let Command::Plan(args) = parse(&["skilltap", "plan", "--target", "codex"])
        .command
        .unwrap()
    else {
        panic!("expected plan")
    };
    assert_eq!(
        args.target.target,
        Some(TargetSelection::Only(HarnessId::new("codex").unwrap()))
    );
    rejects(
        &["skilltap", "plan", "--target", "pi"],
        ErrorKind::ValueValidation,
    );
    rejects(
        &["skilltap", "harness", "enable", "all"],
        ErrorKind::ValueValidation,
    );

    let Command::Harness(HarnessArgs {
        command: HarnessCommand::Enable(enable),
    }) = parse(&[
        "skilltap",
        "harness",
        "enable",
        "codex",
        "--binary",
        "/opt/bin/codex",
    ])
    .command
    .unwrap()
    else {
        panic!("expected harness enable")
    };
    assert_eq!(enable.binary.unwrap().as_str(), "/opt/bin/codex");
}

#[test]
fn sync_preserves_repeatable_selection_and_acknowledgment() {
    let Command::Sync(args) = parse(&[
        "skilltap",
        "sync",
        "--include",
        "plugin:review-tools@personal",
        "--include",
        "skill:release-helper",
        "--exclude",
        "hook:unsafe",
        "--yes",
        "--json",
    ])
    .command
    .unwrap() else {
        panic!("expected sync")
    };
    assert_native_ids(&args.selection.include);
    assert_native_ids(&args.selection.exclude);
    assert_eq!(
        args.selection
            .include
            .iter()
            .map(NativeId::as_str)
            .collect::<Vec<_>>(),
        ["plugin:review-tools@personal", "skill:release-helper"]
    );
    assert_eq!(args.selection.exclude[0].as_str(), "hook:unsafe");
    assert!(args.acknowledgment.yes);
    assert!(args.output.json);
}

#[test]
fn representative_nested_commands_convert_values() {
    let Command::Marketplace(MarketplaceArgs {
        command: MarketplaceCommand::Add(args),
    }) = parse(&[
        "skilltap",
        "marketplace",
        "add",
        "anthropics/claude-plugins",
        "--name",
        "anthropic",
        "--target",
        "claude",
        "--project",
        "/workspace",
        "--json",
    ])
    .command
    .unwrap()
    else {
        panic!("expected marketplace add")
    };
    assert_eq!(args.source.as_str(), "anthropics/claude-plugins");
    assert_eq!(args.name.unwrap().as_str(), "anthropic");

    let Command::Skill(SkillArgs {
        command: SkillCommand::Install(args),
    }) = parse(&[
        "skilltap",
        "skill",
        "install",
        "https://github.com/example/agent-tools",
        "--path",
        "skills/commit-helper",
        "--ref",
        "main",
        "--name",
        "commit-helper",
        "--yes",
    ])
    .command
    .unwrap()
    else {
        panic!("expected skill install")
    };
    assert_eq!(args.path.unwrap().as_str(), "skills/commit-helper");
    assert_eq!(args.requested_revision.unwrap().as_str(), "main");
}

#[test]
fn plugin_install_requires_an_exact_marketplace_selector() {
    let Command::Plugin(PluginArgs {
        command: PluginCommand::Install(args),
    }) = parse(&["skilltap", "plugin", "install", "formatter@team-tools"])
        .command
        .unwrap()
    else {
        panic!("expected plugin install")
    };
    assert_eq!(args.plugin.as_str(), "formatter@team-tools");

    for invalid in ["formatter", "@team-tools", "formatter@", "a@b@c"] {
        rejects(
            &["skilltap", "plugin", "install", invalid],
            ErrorKind::ValueValidation,
        );
    }
}

#[test]
fn invalid_skill_paths_and_daemon_intervals_fail_at_parse_time() {
    rejects(
        &["skilltap", "skill", "install", "repo", "--path", "../skill"],
        ErrorKind::ValueValidation,
    );
    rejects(
        &["skilltap", "daemon", "enable", "--interval", "0h"],
        ErrorKind::ValueValidation,
    );
    let Command::Daemon(DaemonArgs {
        command: DaemonCommand::Enable(args),
    }) = parse(&["skilltap", "daemon", "enable", "--interval", "6h"])
        .command
        .unwrap()
    else {
        panic!("expected daemon enable")
    };
    assert_eq!(args.interval.unwrap().to_string(), "6h");
}

#[test]
fn irrelevant_flags_are_rejected_by_their_commands() {
    for args in [
        &["skilltap", "harness", "list", "--target", "codex"][..],
        &[
            "skilltap", "harness", "disable", "codex", "--binary", "codex",
        ][..],
        &["skilltap", "adopt", "--yes"][..],
        &["skilltap", "status", "--yes"][..],
        &["skilltap", "plan", "--include", "skill:one"][..],
        &["skilltap", "marketplace", "add", "source", "--yes"][..],
        &["skilltap", "plugin", "remove", "name", "--yes"][..],
        &["skilltap", "skill", "list", "--ref", "main"][..],
        &["skilltap", "instructions", "status", "--target", "codex"][..],
    ] {
        rejects(args, ErrorKind::UnknownArgument);
    }
}

#[test]
fn daemon_run_accepts_structured_output() {
    let Command::Daemon(DaemonArgs {
        command: DaemonCommand::Run(args),
    }) = parse(&["skilltap", "daemon", "run", "--json"])
        .command
        .unwrap()
    else {
        panic!("expected daemon run")
    };
    assert!(args.json);
}

#[test]
fn empty_and_control_character_values_are_rejected_at_the_boundary() {
    rejects(
        &["skilltap", "marketplace", "add", ""],
        ErrorKind::ValueValidation,
    );
    rejects(
        &["skilltap", "plugin", "remove", "bad\nname"],
        ErrorKind::ValueValidation,
    );
}

#[test]
fn non_utf8_values_are_rejected_without_panicking() {
    #[cfg(unix)]
    {
        use std::{ffi::OsString, os::unix::ffi::OsStringExt};

        let invalid = OsString::from_vec(vec![0xff]);
        let error = Cli::try_parse_from([
            OsString::from("skilltap"),
            OsString::from("plugin"),
            OsString::from("remove"),
            invalid,
        ])
        .unwrap_err();
        assert_eq!(error.kind(), ErrorKind::InvalidUtf8);
    }
}
