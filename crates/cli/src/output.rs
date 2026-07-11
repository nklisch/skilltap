use std::fmt::Write as _;

use crate::outcome::{ErrorDetail, NextAction, Outcome, OutputEntry, ResultClass, Warning};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum ExitCode {
    Completed = 0,
    Invalid = 1,
    AttentionRequired = 2,
    PartialApply = 3,
}

impl From<ResultClass> for ExitCode {
    fn from(result: ResultClass) -> Self {
        match result {
            ResultClass::Completed => Self::Completed,
            ResultClass::Invalid => Self::Invalid,
            ResultClass::AttentionRequired => Self::AttentionRequired,
            ResultClass::PartialApply => Self::PartialApply,
        }
    }
}

impl From<&Outcome> for ExitCode {
    fn from(outcome: &Outcome) -> Self {
        outcome.result.into()
    }
}

impl ExitCode {
    pub const fn value(self) -> u8 {
        self as u8
    }
}

impl Outcome {
    pub fn exit_code(&self) -> u8 {
        ExitCode::from(self).value()
    }
}

#[derive(Debug)]
pub struct RenderError(serde_json::Error);

impl std::fmt::Display for RenderError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("could not encode command outcome")
    }
}

impl std::error::Error for RenderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

pub trait Renderer {
    fn render(&self, outcome: &Outcome) -> Result<String, RenderError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct JsonRenderer;

impl Renderer for JsonRenderer {
    fn render(&self, outcome: &Outcome) -> Result<String, RenderError> {
        serde_json::to_string(outcome).map_err(RenderError)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PlainRenderer;

impl Renderer for PlainRenderer {
    fn render(&self, outcome: &Outcome) -> Result<String, RenderError> {
        Ok(render_plain(outcome))
    }
}

fn render_plain(outcome: &Outcome) -> String {
    let mut rendered = String::new();

    if let Some(scope) = &outcome.scope {
        line(&mut rendered, "Scope", scope.plain_label());
    }
    for (name, value) in &outcome.summary {
        line(&mut rendered, name, value);
    }
    render_entries(&mut rendered, "Resources", &outcome.resources);
    render_entries(&mut rendered, "Operations", &outcome.operations);
    render_warnings(&mut rendered, &outcome.warnings);
    render_errors(&mut rendered, &outcome.errors);
    render_next_actions(&mut rendered, "Next actions", &outcome.next_actions);

    if !rendered.is_empty() {
        rendered.push('\n');
    }
    let _ = writeln!(rendered, "Result: {}", outcome.result.plain_label());
    rendered
}

fn line(output: &mut String, label: impl std::fmt::Display, value: impl std::fmt::Display) {
    let _ = writeln!(
        output,
        "{}  {}",
        plain_text(&label.to_string()),
        plain_text(&value.to_string())
    );
}

fn render_entries(output: &mut String, heading: &str, entries: &[OutputEntry]) {
    if entries.is_empty() {
        return;
    }
    section_break(output);
    let _ = writeln!(output, "{heading}");
    for entry in entries {
        let _ = writeln!(
            output,
            "  {}  {}",
            plain_text(&entry.id),
            plain_text(&entry.status)
        );
        for (name, value) in &entry.fields {
            let _ = writeln!(
                output,
                "    {}  {}",
                plain_text(name),
                plain_text(&value.to_string())
            );
        }
    }
}

fn render_warnings(output: &mut String, warnings: &[Warning]) {
    if warnings.is_empty() {
        return;
    }
    section_break(output);
    output.push_str("Warnings\n");
    for warning in warnings {
        let _ = writeln!(
            output,
            "  {}: {}",
            plain_text(&warning.code),
            plain_text(&warning.summary)
        );
        for (name, value) in &warning.context {
            let _ = writeln!(output, "    {}  {}", plain_text(name), plain_text(value));
        }
    }
}

fn render_errors(output: &mut String, errors: &[ErrorDetail]) {
    if errors.is_empty() {
        return;
    }
    section_break(output);
    for (index, error) in errors.iter().enumerate() {
        if index > 0 {
            output.push('\n');
        }
        let _ = writeln!(output, "Error: {}", plain_text(&error.summary));
        let _ = writeln!(output, "Code: {}", plain_text(&error.code));
        for (name, value) in &error.context {
            line(output, name, value);
        }
        render_next_actions(output, "Next actions", &error.next_actions);
    }
}

fn render_next_actions(output: &mut String, heading: &str, actions: &[NextAction]) {
    if actions.is_empty() {
        return;
    }
    section_break(output);
    let _ = writeln!(output, "{heading}");
    for action in actions {
        let _ = writeln!(
            output,
            "  {}: {}",
            plain_text(&action.code),
            plain_text(&action.summary)
        );
        if let Some(command) = &action.command {
            let _ = writeln!(output, "    {}", plain_text(command));
        }
    }
}

fn plain_text(value: &str) -> String {
    value.chars().fold(String::new(), |mut safe, character| {
        match character {
            '\n' => safe.push_str("\\n"),
            '\r' => safe.push_str("\\r"),
            '\t' => safe.push_str("\\t"),
            character if character.is_control() => {
                let _ = write!(safe, "\\u{{{:x}}}", character as u32);
            }
            character => safe.push(character),
        }
        safe
    })
}

fn section_break(output: &mut String) {
    if !output.is_empty() && !output.ends_with("\n\n") {
        output.push('\n');
    }
}

#[cfg(test)]
mod tests {
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
}
