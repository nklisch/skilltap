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
        let mut normalized = outcome.clone();
        normalized.normalize_next_actions();
        serde_json::to_string(&normalized).map_err(RenderError)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PlainRenderer;

impl Renderer for PlainRenderer {
    fn render(&self, outcome: &Outcome) -> Result<String, RenderError> {
        let mut normalized = outcome.clone();
        normalized.normalize_next_actions();
        Ok(render_plain(&normalized))
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
mod tests;
