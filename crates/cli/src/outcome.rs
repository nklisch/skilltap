use std::collections::BTreeMap;

use serde::Serialize;

pub const OUTPUT_SCHEMA_VERSION: u8 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultClass {
    Completed,
    Invalid,
    AttentionRequired,
    PartialApply,
}

impl ResultClass {
    pub const fn plain_label(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Invalid => "invalid",
            Self::AttentionRequired => "attention required",
            Self::PartialApply => "partial apply; recovery required",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OutputScope {
    Global,
    Project { path: String },
    All,
}

impl OutputScope {
    pub(crate) fn plain_label(&self) -> &str {
        match self {
            Self::Global => "global",
            Self::Project { path } => path,
            Self::All => "all scopes",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum OutputValue {
    Boolean(bool),
    Integer(i64),
    Unsigned(u64),
    Text(String),
}

impl From<bool> for OutputValue {
    fn from(value: bool) -> Self {
        Self::Boolean(value)
    }
}

impl From<i64> for OutputValue {
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}

impl From<u64> for OutputValue {
    fn from(value: u64) -> Self {
        Self::Unsigned(value)
    }
}

impl From<String> for OutputValue {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<&str> for OutputValue {
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}

impl std::fmt::Display for OutputValue {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Boolean(value) => value.fmt(formatter),
            Self::Integer(value) => value.fmt(formatter),
            Self::Unsigned(value) => value.fmt(formatter),
            Self::Text(value) => value.fmt(formatter),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct OutputEntry {
    pub id: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, OutputValue>,
}

impl OutputEntry {
    pub fn new(id: impl Into<String>, status: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            status: status.into(),
            fields: BTreeMap::new(),
        }
    }

    pub fn with_field(mut self, name: impl Into<String>, value: impl Into<OutputValue>) -> Self {
        self.fields.insert(name.into(), value.into());
        self
    }
}

pub type ResourceOutcome = OutputEntry;
pub type OperationOutcome = OutputEntry;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Warning {
    pub code: String,
    pub summary: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub context: BTreeMap<String, String>,
}

impl Warning {
    pub fn new(code: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            summary: summary.into(),
            context: BTreeMap::new(),
        }
    }

    pub fn with_context(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.insert(name.into(), value.into());
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct NextAction {
    pub code: String,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
}

impl NextAction {
    pub fn new(code: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            summary: summary.into(),
            command: None,
        }
    }

    pub fn with_command(mut self, command: impl Into<String>) -> Self {
        self.command = Some(command.into());
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ErrorDetail {
    pub code: String,
    pub summary: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub context: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub next_actions: Vec<NextAction>,
}

impl ErrorDetail {
    pub fn new(code: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            summary: summary.into(),
            context: BTreeMap::new(),
            next_actions: Vec::new(),
        }
    }

    pub fn with_context(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.insert(name.into(), value.into());
        self
    }

    pub fn with_next_action(mut self, action: NextAction) -> Self {
        self.next_actions.push(action);
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Outcome {
    schema: u8,
    pub command: String,
    pub result: ResultClass,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<OutputScope>,
    pub summary: BTreeMap<String, OutputValue>,
    pub resources: Vec<ResourceOutcome>,
    pub operations: Vec<OperationOutcome>,
    pub warnings: Vec<Warning>,
    pub errors: Vec<ErrorDetail>,
    pub next_actions: Vec<NextAction>,
}

impl Outcome {
    pub fn new(command: impl Into<String>, result: ResultClass) -> Self {
        Self {
            schema: OUTPUT_SCHEMA_VERSION,
            command: command.into(),
            result,
            scope: None,
            summary: BTreeMap::new(),
            resources: Vec::new(),
            operations: Vec::new(),
            warnings: Vec::new(),
            errors: Vec::new(),
            next_actions: Vec::new(),
        }
    }

    pub const fn schema(&self) -> u8 {
        self.schema
    }

    pub fn with_scope(mut self, scope: OutputScope) -> Self {
        self.scope = Some(scope);
        self
    }

    pub fn with_summary(mut self, name: impl Into<String>, value: impl Into<OutputValue>) -> Self {
        self.summary.insert(name.into(), value.into());
        self
    }

    pub fn with_resource(mut self, resource: ResourceOutcome) -> Self {
        self.resources.push(resource);
        self
    }

    pub fn with_operation(mut self, operation: OperationOutcome) -> Self {
        self.operations.push(operation);
        self
    }

    pub fn with_warning(mut self, warning: Warning) -> Self {
        self.warnings.push(warning);
        self
    }

    pub fn with_error(mut self, error: ErrorDetail) -> Self {
        self.errors.push(error);
        self
    }

    pub fn with_next_action(mut self, action: NextAction) -> Self {
        self.next_actions.push(action);
        self
    }
}
