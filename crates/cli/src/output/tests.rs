use super::*;
use crate::outcome::{OutputScope, OutputValue};

fn representative_outcome(result: ResultClass) -> Outcome {
    Outcome::new("sync", result)
        .with_scope(OutputScope::Project {
            path: "/tmp/project".to_owned(),
        })
        .with_summary("blocked", 1_u64)
        .with_summary("healthy", false)
        .with_resource(
            OutputEntry::new("plugin:review-tools", "blocked").with_field("target", "codex"),
        )
        .with_operation(OutputEntry::new("operation:1", "not_started"))
        .with_warning(
            Warning::new("compatibility_unknown", "Compatibility is unverified.")
                .with_context("target", "codex"),
        )
        .with_error(
            ErrorDetail::new(
                "partial_materialization",
                "A component cannot be transferred.",
            )
            .with_context("resource", "plugin:review-tools")
            .with_next_action(
                NextAction::new("explain_partial", "Explain the missing component.")
                    .with_command("skilltap plan --include plugin:review-tools"),
            ),
        )
        .with_next_action(NextAction::new(
            "inspect_plan",
            "Inspect the blocked operation.",
        ))
}

#[test]
fn result_class_is_the_only_exit_code_input() {
    let cases = [
        (ResultClass::Completed, ExitCode::Completed, 0),
        (ResultClass::Invalid, ExitCode::Invalid, 1),
        (
            ResultClass::AttentionRequired,
            ExitCode::AttentionRequired,
            2,
        ),
        (ResultClass::PartialApply, ExitCode::PartialApply, 3),
    ];
    for (result, expected, value) in cases {
        assert_eq!(ExitCode::from(result), expected);
        assert_eq!(expected.value(), value);
        assert_eq!(Outcome::new("status", result).exit_code(), value);
    }
}

#[test]
fn json_is_one_compact_schema_one_document_with_stable_fields() {
    let rendered = JsonRenderer
        .render(&representative_outcome(ResultClass::AttentionRequired))
        .unwrap();
    assert!(!rendered.starts_with('\n'));
    assert!(!rendered.ends_with('\n'));
    assert!(!rendered.contains("\u{1b}["));

    let value: serde_json::Value = serde_json::from_str(&rendered).unwrap();
    assert_eq!(value["schema"], 1);
    assert_eq!(value["command"], "sync");
    assert_eq!(value["result"], "attention_required");
    assert_eq!(value["scope"]["kind"], "project");
    assert_eq!(value["summary"]["blocked"], 1);
    assert!(value["resources"].is_array());
    assert!(value["operations"].is_array());
    assert!(value["warnings"].is_array());
    assert!(value["errors"].is_array());
    assert!(value["next_actions"].is_array());
}

#[test]
fn json_keeps_all_required_collections_when_empty_and_omits_only_scope() {
    let rendered = JsonRenderer
        .render(&Outcome::new("status", ResultClass::Completed))
        .unwrap();
    assert_eq!(
        rendered,
        r#"{"schema":1,"command":"status","result":"completed","summary":{},"resources":[],"operations":[],"warnings":[],"errors":[],"next_actions":[]}"#
    );
}

#[test]
fn plain_and_json_are_derived_from_the_same_outcome() {
    let outcome = representative_outcome(ResultClass::PartialApply);
    let plain = PlainRenderer.render(&outcome).unwrap();
    let json = JsonRenderer.render(&outcome).unwrap();

    for semantic_value in [
        "sync",
        "/tmp/project",
        "plugin:review-tools",
        "partial_materialization",
        "inspect_plan",
    ] {
        assert!(json.contains(semantic_value));
    }
    for semantic_value in [
        "/tmp/project",
        "plugin:review-tools",
        "partial_materialization",
        "inspect_plan",
    ] {
        assert!(plain.contains(semantic_value));
    }
    assert!(plain.ends_with("Result: partial apply; recovery required\n"));
}

#[test]
fn renderers_deduplicate_exact_actions_without_collapsing_distinct_commands() {
    let inspect = NextAction::new("inspect", "Inspect the exact failure.")
        .with_command("skilltap status --json");
    let project = NextAction::new("inspect", "Inspect the project failure.")
        .with_command("skilltap status --project --json");
    let mut outcome = Outcome::new("status", ResultClass::AttentionRequired);
    outcome.next_actions = vec![inspect.clone(), inspect, project];

    let json = JsonRenderer.render(&outcome).unwrap();
    let plain = PlainRenderer.render(&outcome).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(value["next_actions"].as_array().unwrap().len(), 2);
    assert_eq!(plain.matches("skilltap status --json").count(), 1);
    assert_eq!(plain.matches("skilltap status --project --json").count(), 1);
    assert_eq!(outcome.next_actions.len(), 3, "rendering is read-only");
    assert_eq!(outcome.result, ResultClass::AttentionRequired);
}

#[test]
fn safe_errors_do_not_have_a_serialized_source_or_debug_channel() {
    let outcome = Outcome::new("sync", ResultClass::Invalid).with_error(
        ErrorDetail::new("native_operation_failed", "Codex rejected the operation.")
            .with_context("exit_status", "1"),
    );
    let json = JsonRenderer.render(&outcome).unwrap();
    let plain = PlainRenderer.render(&outcome).unwrap();

    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    let error = value["errors"][0].as_object().unwrap();

    assert!(!error.contains_key("source"));
    assert!(!error.contains_key("debug"));
    assert!(plain.starts_with("Error: Codex rejected the operation."));
}

#[test]
fn plain_output_neutralizes_terminal_controls_and_line_injection() {
    let outcome = Outcome::new("status", ResultClass::Invalid).with_error(
        ErrorDetail::new("native_failure", "Rejected.\nResult: completed")
            .with_context("native", "\u{1b}[31msecret\u{1b}[0m"),
    );
    let plain = PlainRenderer.render(&outcome).unwrap();

    assert!(!plain.contains('\u{1b}'));
    assert!(plain.contains(r#"Rejected.\nResult: completed"#));
    assert!(plain.contains(r#"\u{1b}[31msecret\u{1b}[0m"#));
    assert_eq!(
        plain
            .lines()
            .filter(|line| line.starts_with("Result:"))
            .count(),
        1
    );
    assert!(plain.ends_with("Result: invalid\n"));
}

#[test]
fn scalar_output_values_preserve_json_types() {
    let outcome = Outcome::new("status", ResultClass::Completed)
        .with_summary("signed", OutputValue::Integer(-1))
        .with_summary("unsigned", OutputValue::Unsigned(u64::MAX))
        .with_summary("boolean", OutputValue::Boolean(true))
        .with_summary("text", OutputValue::Text("ok".to_owned()));
    let json = JsonRenderer.render(&outcome).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["summary"]["signed"], -1);
    assert_eq!(value["summary"]["unsigned"], u64::MAX);
    assert_eq!(value["summary"]["boolean"], true);
    assert_eq!(value["summary"]["text"], "ok");
}
