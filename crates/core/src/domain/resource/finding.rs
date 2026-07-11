use std::{collections::BTreeMap, fmt};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::domain::{
    CapabilityId, HarnessId, ResourceKey, Scope, validate_identifier,
    validated_newtype::validated_string_newtype,
};

use super::{ObservationLayer, ResourceKind};

const MAX_FINDING_FIELDS: usize = 32;

validated_string_newtype!(
    ObservationFindingCode,
    "observation finding code",
    128,
    validate_identifier,
    try_from
);
validated_string_newtype!(
    ObservationFieldCode,
    "observation field code",
    64,
    validate_identifier,
    try_from
);

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationSeverity {
    Info,
    Warning,
    Error,
    Blocking,
}

/// Authored user-facing text. The wire accepts only these exact static summaries.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ObservationSummary {
    MalformedNativeEntry,
    NativeStateUnreadable,
    NativeShapeUnsupported,
    NativeStateConflict,
    NativeStateIncomplete,
    ResourceUnmanaged,
    ResourceDrifted,
    CapabilityUnverified,
    HigherPrecedenceConfiguration,
    TrustRequired,
    ConsentRequired,
    ScopeUnsupported,
    UnsafeFilesystemEntry,
}

impl ObservationSummary {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MalformedNativeEntry => "A native entry is malformed.",
            Self::NativeStateUnreadable => "Native state could not be read.",
            Self::NativeShapeUnsupported => "The native state shape is unsupported.",
            Self::NativeStateConflict => "Native state contains a conflict.",
            Self::NativeStateIncomplete => "Native state is incomplete.",
            Self::ResourceUnmanaged => "The resource is not managed by skilltap.",
            Self::ResourceDrifted => "The resource differs from its managed state.",
            Self::CapabilityUnverified => "The required harness capability is unverified.",
            Self::HigherPrecedenceConfiguration => {
                "Higher-precedence native configuration is effective."
            }
            Self::TrustRequired => "The harness requires project trust.",
            Self::ConsentRequired => "The harness requires user consent.",
            Self::ScopeUnsupported => "The harness does not support this scope.",
            Self::UnsafeFilesystemEntry => "A native filesystem entry is unsafe to inspect.",
        }
    }
}

impl fmt::Display for ObservationSummary {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl Serialize for ObservationSummary {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ObservationSummary {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        ALL_SUMMARIES
            .into_iter()
            .find(|summary| summary.as_str() == value)
            .ok_or_else(|| serde::de::Error::custom("unknown authored observation summary"))
    }
}

const ALL_SUMMARIES: [ObservationSummary; 13] = [
    ObservationSummary::MalformedNativeEntry,
    ObservationSummary::NativeStateUnreadable,
    ObservationSummary::NativeShapeUnsupported,
    ObservationSummary::NativeStateConflict,
    ObservationSummary::NativeStateIncomplete,
    ObservationSummary::ResourceUnmanaged,
    ObservationSummary::ResourceDrifted,
    ObservationSummary::CapabilityUnverified,
    ObservationSummary::HigherPrecedenceConfiguration,
    ObservationSummary::TrustRequired,
    ObservationSummary::ConsentRequired,
    ObservationSummary::ScopeUnsupported,
    ObservationSummary::UnsafeFilesystemEntry,
];

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObservationSubject {
    Harness {
        harness: HarnessId,
        scope: Scope,
    },
    Resource {
        harness: HarnessId,
        resource: ResourceKey,
    },
}

impl ObservationSubject {
    pub fn harness(&self) -> &HarnessId {
        match self {
            Self::Harness { harness, .. } | Self::Resource { harness, .. } => harness,
        }
    }

    pub const fn scope(&self) -> &Scope {
        match self {
            Self::Harness { scope, .. } => scope,
            Self::Resource { resource, .. } => resource.scope(),
        }
    }

    pub const fn resource(&self) -> Option<&ResourceKey> {
        match self {
            Self::Harness { .. } => None,
            Self::Resource { resource, .. } => Some(resource),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ObservationFieldValue {
    Boolean(bool),
    Count(u64),
    Harness(HarnessId),
    Resource(ResourceKey),
    ResourceKind(ResourceKind),
    Capability(CapabilityId),
    Layer(ObservationLayer),
}

#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ObservationFields(BTreeMap<ObservationFieldCode, ObservationFieldValue>);

impl ObservationFields {
    pub fn new(
        fields: impl IntoIterator<Item = (ObservationFieldCode, ObservationFieldValue)>,
    ) -> Result<Self, ObservationFindingError> {
        let fields = fields.into_iter().collect::<BTreeMap<_, _>>();
        if fields.len() > MAX_FINDING_FIELDS {
            return Err(ObservationFindingError::TooManyFields {
                max: MAX_FINDING_FIELDS,
                actual: fields.len(),
            });
        }
        Ok(Self(fields))
    }

    pub fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = (&ObservationFieldCode, &ObservationFieldValue)> {
        self.0.iter()
    }
    pub fn get(&self, code: &ObservationFieldCode) -> Option<&ObservationFieldValue> {
        self.0.get(code)
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Serialize for ObservationFields {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ObservationFields {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(
            BTreeMap::<ObservationFieldCode, ObservationFieldValue>::deserialize(deserializer)?,
        )
        .map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationFinding {
    code: ObservationFindingCode,
    summary: ObservationSummary,
    severity: ObservationSeverity,
    subject: ObservationSubject,
    #[serde(default, skip_serializing_if = "ObservationFields::is_empty")]
    fields: ObservationFields,
}

impl ObservationFinding {
    pub const fn new(
        code: ObservationFindingCode,
        summary: ObservationSummary,
        severity: ObservationSeverity,
        subject: ObservationSubject,
        fields: ObservationFields,
    ) -> Self {
        Self {
            code,
            summary,
            severity,
            subject,
            fields,
        }
    }
    pub fn code(&self) -> &ObservationFindingCode {
        &self.code
    }
    pub const fn summary(&self) -> ObservationSummary {
        self.summary
    }
    pub const fn severity(&self) -> ObservationSeverity {
        self.severity
    }
    pub const fn subject(&self) -> &ObservationSubject {
        &self.subject
    }
    pub const fn fields(&self) -> &ObservationFields {
        &self.fields
    }
}

impl fmt::Display for ObservationFinding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.summary)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ObservationFindingError {
    TooManyFields { max: usize, actual: usize },
}

impl fmt::Display for ObservationFindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyFields { max, actual } => write!(
                formatter,
                "observation finding allows at most {max} fields, got {actual}"
            ),
        }
    }
}
impl std::error::Error for ObservationFindingError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{AbsolutePath, ResourceId};
    use serde_json::json;

    const SECRET: &str = "sk-test-auth=must-not-enter\n--token raw-secret";

    fn subject() -> ObservationSubject {
        ObservationSubject::Resource {
            harness: HarnessId::new("codex").unwrap(),
            resource: ResourceKey::new(
                ResourceId::new("plugin:formatter@official").unwrap(),
                Scope::Project(AbsolutePath::new("/work/project").unwrap()),
            ),
        }
    }
    fn finding(fields: ObservationFields) -> ObservationFinding {
        ObservationFinding::new(
            ObservationFindingCode::new("native.entry.malformed").unwrap(),
            ObservationSummary::MalformedNativeEntry,
            ObservationSeverity::Warning,
            subject(),
            fields,
        )
    }

    #[test]
    fn authored_finding_round_trips_with_deterministic_typed_fields() {
        let fields = ObservationFields::new([
            (
                ObservationFieldCode::new("expected-kind").unwrap(),
                ObservationFieldValue::ResourceKind(ResourceKind::Plugin),
            ),
            (
                ObservationFieldCode::new("affected-count").unwrap(),
                ObservationFieldValue::Count(2),
            ),
        ])
        .unwrap();
        let finding = finding(fields);
        let encoded = serde_json::to_string(&finding).unwrap();
        assert_eq!(
            serde_json::from_str::<ObservationFinding>(&encoded).unwrap(),
            finding
        );
        assert!(encoded.find("affected-count").unwrap() < encoded.find("expected-kind").unwrap());
        assert_eq!(
            finding.subject().scope(),
            &Scope::Project(AbsolutePath::new("/work/project").unwrap())
        );
    }

    #[test]
    fn arbitrary_native_payload_channels_are_rejected() {
        let base = serde_json::to_value(finding(ObservationFields::default())).unwrap();
        for field in [
            "argv", "stdout", "stderr", "settings", "metadata", "message",
        ] {
            let mut payload = base.clone();
            payload[field] = json!(SECRET);
            assert!(
                serde_json::from_value::<ObservationFinding>(payload).is_err(),
                "accepted raw `{field}` channel"
            );
        }
        let mut dynamic_summary = base.clone();
        dynamic_summary["summary"] = json!(SECRET);
        assert!(serde_json::from_value::<ObservationFinding>(dynamic_summary).is_err());
        let mut raw_field = base;
        raw_field["fields"] = json!({"native-output":{"kind":"string","value":SECRET}});
        assert!(serde_json::from_value::<ObservationFinding>(raw_field).is_err());
    }

    #[test]
    fn secret_canary_never_appears_in_safe_forms() {
        for value in [
            serde_json::to_string(&finding(ObservationFields::default())).unwrap(),
            format!("{:?}", finding(ObservationFields::default())),
            finding(ObservationFields::default()).to_string(),
        ] {
            assert!(!value.contains(SECRET));
            assert!(!value.contains("raw-secret"));
        }
        assert!(ObservationFindingCode::new(SECRET).is_err());
        assert!(ObservationFieldCode::new(SECRET).is_err());
    }

    #[test]
    fn fields_are_bounded_and_owned_shapes_are_strict() {
        let fields = (0..=MAX_FINDING_FIELDS).map(|index| {
            (
                ObservationFieldCode::new(format!("field-{index}")).unwrap(),
                ObservationFieldValue::Count(index as u64),
            )
        });
        assert!(matches!(
            ObservationFields::new(fields),
            Err(ObservationFindingError::TooManyFields { .. })
        ));
        assert!(serde_json::from_value::<ObservationFinding>(json!({
            "code":"native.entry.malformed", "summary":"A native entry is malformed.", "severity":"warning",
            "subject":{"kind":"harness","harness":"codex","scope":{"kind":"global"}}, "future":true
        })).is_err());
    }

    #[test]
    fn open_codes_are_validated_without_closing_the_vocabulary() {
        for code in ["native.entry.malformed", "vendor7.future-shape.warning"] {
            assert_eq!(ObservationFindingCode::new(code).unwrap().as_str(), code);
        }
        for invalid in ["Native.entry", "native entry", ".native", "native/entry"] {
            assert!(ObservationFindingCode::new(invalid).is_err(), "{invalid}");
        }
    }
}
