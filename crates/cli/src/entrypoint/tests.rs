use std::ffi::OsString;

use serde_json::Value;

use super::*;

#[test]
fn plan_composes_as_an_attention_report_and_other_unavailable_commands_remain_explicit() {
    let plain = run_from(["skilltap", "plan"]);
    assert_eq!(plain.exit_code, 2);
    assert_eq!(plain.channel, OutputChannel::Stdout);
    assert!(plain.document.contains("Result: attention required"));

    let json = run_from(["skilltap", "plugin", "install", "format@team", "--json"]);
    assert_eq!(json.exit_code, 2);
    assert_eq!(json.channel, OutputChannel::Stdout);
    let value: Value = serde_json::from_str(&json.document).unwrap();
    assert_eq!(value["command"], "plugin install");
    assert_eq!(value["result"], "attention_required");
    assert_eq!(value["errors"][0]["code"], "no_enabled_harnesses");
}

#[test]
fn first_use_plain_status_is_an_attention_report_on_stdout() {
    let outcome =
        Outcome::new("status", ResultClass::AttentionRequired).with_warning(crate::Warning::new(
            "native_observation_unavailable",
            "Native harness observation is not available in this build.",
        ));

    let execution = render(outcome, false, OutputChannel::Stdout);

    assert_eq!(execution.exit_code, 2);
    assert_eq!(execution.channel, OutputChannel::Stdout);
    assert!(execution.document.contains("Result: attention required"));
}

#[test]
fn unregistered_targets_are_rejected_at_composition_before_dispatch() {
    let execution = run_from(["skilltap", "status", "--target", "not-real", "--json"]);

    assert_eq!(execution.exit_code, 1);
    assert_eq!(execution.channel, OutputChannel::Stdout);
    let value: Value = serde_json::from_str(&execution.document).unwrap();
    assert_eq!(value["result"], "invalid");
    assert_eq!(value["errors"][0]["code"], "target_not_registered");
    assert_eq!(value["errors"][0]["context"]["harness"], "not-real");
    assert_eq!(execution.document.lines().count(), 1);
}

#[test]
fn nested_missing_commands_point_at_the_deepest_safe_help_boundary() {
    let execution = run_from(["skilltap", "plugin", "--json"]);

    assert_eq!(execution.exit_code, 1);
    assert_eq!(execution.channel, OutputChannel::Stdout);
    let value: Value = serde_json::from_str(&execution.document).unwrap();
    assert_eq!(value["command"], "plugin");
    assert_eq!(value["errors"][0]["code"], "invalid_arguments");
    assert_eq!(value["errors"][0]["context"]["boundary"], "skilltap plugin");
    assert_eq!(
        value["next_actions"][0]["command"],
        "skilltap plugin --help"
    );
    assert_eq!(execution.document.lines().count(), 1);
}

#[test]
fn unknown_commands_fall_back_to_root_without_echoing_the_token() {
    let execution = run_from(["skilltap", "secret-command", "--json"]);

    assert_eq!(execution.exit_code, 1);
    let value: Value = serde_json::from_str(&execution.document).unwrap();
    assert_eq!(value["command"], "skilltap");
    assert_eq!(value["errors"][0]["context"]["boundary"], "skilltap");
    assert_eq!(value["next_actions"][0]["command"], "skilltap --help");
    assert!(!execution.document.contains("secret-command"));
}

#[test]
fn invalid_source_diagnostics_redact_locator_values() {
    let execution = run_from([
        "skilltap",
        "marketplace",
        "add",
        "https://user:token@example.test/repo.git",
        "--json",
    ]);

    assert_eq!(execution.exit_code, 1);
    assert_eq!(execution.channel, OutputChannel::Stdout);
    let value: Value = serde_json::from_str(&execution.document).unwrap();
    assert_eq!(value["command"], "marketplace add");
    assert_eq!(
        value["errors"][0]["context"]["boundary"],
        "skilltap marketplace add"
    );
    assert_eq!(
        value["next_actions"][0]["command"],
        "skilltap marketplace add --help"
    );
    assert!(!execution.document.contains("user:token"));
    assert!(!execution.document.contains("example.test"));
}

#[test]
fn non_utf8_parse_arguments_are_redacted_from_json_diagnostics() {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;

        let execution = run_from([
            OsString::from("skilltap"),
            OsString::from("plugin"),
            OsString::from("remove"),
            OsString::from_vec(vec![0xff]),
            OsString::from("--json"),
        ]);
        assert_eq!(execution.exit_code, 1);
        assert_eq!(execution.channel, OutputChannel::Stdout);
        let value: Value = serde_json::from_str(&execution.document).unwrap();
        assert_eq!(value["command"], "plugin remove");
        assert_eq!(value["errors"][0]["code"], "invalid_utf8_argument");
        assert_eq!(
            value["errors"][0]["context"]["boundary"],
            "skilltap plugin remove"
        );
    }
}

#[test]
fn missing_command_is_a_stable_input_error() {
    let execution = run_from(["skilltap"]);

    assert_eq!(execution.exit_code, 1);
    assert_eq!(execution.channel, OutputChannel::Stderr);
    assert!(execution.document.contains("Code: missing_command"));
    assert!(execution.document.contains("Usage: skilltap <COMMAND>"));
}

#[test]
fn json_requested_without_a_command_remains_one_normalized_document() {
    let execution = run_from(["skilltap", "--json"]);

    assert_eq!(execution.exit_code, 1);
    assert_eq!(execution.channel, OutputChannel::Stdout);
    assert_eq!(execution.document.lines().count(), 1);
    let value: Value = serde_json::from_str(&execution.document).unwrap();
    assert_eq!(value["command"], "skilltap");
    assert_eq!(value["errors"][0]["code"], "invalid_arguments");
    assert!(!execution.document.contains("--json"));
    assert!(!execution.document.contains("Usage:"));
}

#[test]
fn help_and_version_complete_on_stdout() {
    let root_help = run_from(["skilltap", "--help"]);
    assert!(
        root_help
            .document
            .contains("Registered harnesses: codex|claude")
    );

    for arguments in [
        &["skilltap", "--help"][..],
        &["skilltap", "--version"][..],
        &["skilltap", "status", "--help"][..],
    ] {
        let execution = run_from(arguments.iter().copied());
        assert_eq!(execution.exit_code, 0);
        assert_eq!(execution.channel, OutputChannel::Stdout);
        assert!(!execution.document.is_empty());
    }
}
