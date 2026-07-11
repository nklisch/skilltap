use serde_json::Value;

use super::*;

#[test]
fn unavailable_commands_fail_before_composition_in_plain_and_json() {
    let plain = run_from(["skilltap", "plan"]);
    assert_eq!(plain.exit_code, 1);
    assert_eq!(plain.channel, OutputChannel::Stderr);
    assert!(plain.document.contains("Code: capability_unavailable"));

    let json = run_from(["skilltap", "plugin", "install", "format@team", "--json"]);
    assert_eq!(json.exit_code, 1);
    assert_eq!(json.channel, OutputChannel::Stdout);
    let value: Value = serde_json::from_str(&json.document).unwrap();
    assert_eq!(value["command"], "plugin install");
    assert_eq!(value["errors"][0]["code"], "capability_unavailable");
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
fn parse_failures_are_normalized_as_one_json_document_when_requested() {
    let execution = run_from(["skilltap", "status", "--target", "pi", "--json"]);

    assert_eq!(execution.exit_code, 1);
    assert_eq!(execution.channel, OutputChannel::Stdout);
    let value: Value = serde_json::from_str(&execution.document).unwrap();
    assert_eq!(value["result"], "invalid");
    assert_eq!(value["errors"][0]["code"], "invalid_arguments");
    assert!(!execution.document.contains("pi"));
    assert_eq!(execution.document.lines().count(), 1);
}

#[test]
fn missing_command_is_a_stable_input_error() {
    let execution = run_from(["skilltap"]);

    assert_eq!(execution.exit_code, 1);
    assert_eq!(execution.channel, OutputChannel::Stderr);
    assert!(execution.document.contains("Code: missing_command"));
    assert!(!execution.document.contains("Usage:"));
}

#[test]
fn help_and_version_complete_on_stdout() {
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
