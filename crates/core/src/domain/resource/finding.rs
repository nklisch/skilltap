use std::{collections::BTreeMap, fmt};

use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{MapAccess, Visitor},
    ser::SerializeMap,
};

use crate::domain::{CapabilityId, HarnessId, NativeId, ResourceKey, Scope};

use super::{ObservationLayer, ResourceKind};

const MAX_FINDING_FIELDS: usize = 32;

macro_rules! registered_vocabulary {
    ($name:ident, $error:literal, { $($variant:ident => $wire:literal),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub enum $name { $($variant),+ }

        impl $name {
            pub const fn as_str(self) -> &'static str {
                match self { $(Self::$variant => $wire),+ }
            }

            pub fn from_registered(value: &str) -> Option<Self> {
                match value { $($wire => Some(Self::$variant),)+ _ => None }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where S: Serializer {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where D: Deserializer<'de> {
                let value = String::deserialize(deserializer)?;
                Self::from_registered(&value).ok_or_else(|| serde::de::Error::custom($error))
            }
        }
    };
}

registered_vocabulary!(ObservationFindingCode, "unregistered observation finding code", {
    NativeEntryMalformed => "native.entry.malformed",
    NativeStateUnreadable => "native.state.unreadable",
    NativeShapeUnsupported => "native.shape.unsupported",
    NativeStateConflict => "native.state.conflict",
    NativeStateIncomplete => "native.state.incomplete",
    ResourceUnmanaged => "resource.unmanaged",
    ResourceDrifted => "resource.drifted",
    CapabilityUnverified => "capability.unverified",
    HigherPrecedenceConfiguration => "configuration.higher-precedence",
    TrustRequired => "trust.required",
    ConsentRequired => "consent.required",
    ScopeUnsupported => "scope.unsupported",
    UnsafeFilesystemEntry => "filesystem.entry.unsafe",
    SkillFormatInvalid => "skill.format.invalid",
    SkillTargetIncompatible => "skill.target.incompatible",
    SkillLinkMissing => "skill.link.missing",
    SkillLinkBroken => "skill.link.broken",
    SkillLinkDivergent => "skill.link.divergent",
    SkillDestinationUnmanaged => "skill.destination.unmanaged",
    ProfileComponentMissing => "profile.component.missing",
    ProfileComponentVersionUnverified => "profile.component.version-unverified",
    ProfileComponentInactive => "profile.component.inactive",
    ProfileComponentIncompatible => "profile.component.incompatible",
    CompoundProfileUnavailable => "profile.compound.unavailable",
});

registered_vocabulary!(ObservationFieldCode, "unregistered observation field code", {
    AffectedCount => "affected_count",
    ExpectedResourceKind => "expected_resource_kind",
    ObservedResourceKind => "observed_resource_kind",
    Capability => "capability",
    Layer => "layer",
    RelatedHarness => "related_harness",
    RelatedResource => "related_resource",
    Enabled => "enabled",
    Reachable => "reachable",
    Required => "required",
    Adoptable => "adoptable",
    ProfileComponent => "profile_component",
});

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
    SkillFormatInvalid,
    SkillTargetIncompatible,
    SkillLinkMissing,
    SkillLinkBroken,
    SkillLinkDivergent,
    SkillDestinationUnmanaged,
    ProfileComponentMissing,
    ProfileComponentVersionUnverified,
    ProfileComponentInactive,
    ProfileComponentIncompatible,
    CompoundProfileUnavailable,
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
            Self::SkillFormatInvalid => "A project skill's canonical format is invalid.",
            Self::SkillTargetIncompatible => {
                "A project skill is not loadable by the selected target."
            }
            Self::SkillLinkMissing => "A project skill target link is missing.",
            Self::SkillLinkBroken => "A project skill target link is broken.",
            Self::SkillLinkDivergent => "A project skill target link points elsewhere.",
            Self::SkillDestinationUnmanaged => {
                "A project skill destination is occupied by unmanaged content."
            }
            Self::ProfileComponentMissing => "A required profile component is missing.",
            Self::ProfileComponentVersionUnverified => "A profile component version is unverified.",
            Self::ProfileComponentInactive => "A required profile component is inactive.",
            Self::ProfileComponentIncompatible => "A profile component is not compatible.",
            Self::CompoundProfileUnavailable => {
                "The conditional profile cannot authorize mutation."
            }
        }
    }

    fn from_authored(value: &str) -> Option<Self> {
        ALL_SUMMARIES
            .into_iter()
            .find(|summary| summary.as_str() == value)
    }
}

const ALL_SUMMARIES: [ObservationSummary; 24] = [
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
    ObservationSummary::SkillFormatInvalid,
    ObservationSummary::SkillTargetIncompatible,
    ObservationSummary::SkillLinkMissing,
    ObservationSummary::SkillLinkBroken,
    ObservationSummary::SkillLinkDivergent,
    ObservationSummary::SkillDestinationUnmanaged,
    ObservationSummary::ProfileComponentMissing,
    ObservationSummary::ProfileComponentVersionUnverified,
    ObservationSummary::ProfileComponentInactive,
    ObservationSummary::ProfileComponentIncompatible,
    ObservationSummary::CompoundProfileUnavailable,
];

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
        Self::from_authored(&value)
            .ok_or_else(|| serde::de::Error::custom("unknown authored observation summary"))
    }
}

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

/// Registered field names and their only valid scalar type.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ObservationField {
    AffectedCount(u64),
    ExpectedResourceKind(ResourceKind),
    ObservedResourceKind(ResourceKind),
    Capability(CapabilityId),
    Layer(ObservationLayer),
    RelatedHarness(HarnessId),
    RelatedResource(ResourceKey),
    Enabled(bool),
    Reachable(bool),
    Required(bool),
    Adoptable(bool),
    ProfileComponent(NativeId),
}

impl ObservationField {
    pub const fn code(&self) -> ObservationFieldCode {
        match self {
            Self::AffectedCount(_) => ObservationFieldCode::AffectedCount,
            Self::ExpectedResourceKind(_) => ObservationFieldCode::ExpectedResourceKind,
            Self::ObservedResourceKind(_) => ObservationFieldCode::ObservedResourceKind,
            Self::Capability(_) => ObservationFieldCode::Capability,
            Self::Layer(_) => ObservationFieldCode::Layer,
            Self::RelatedHarness(_) => ObservationFieldCode::RelatedHarness,
            Self::RelatedResource(_) => ObservationFieldCode::RelatedResource,
            Self::Enabled(_) => ObservationFieldCode::Enabled,
            Self::Reachable(_) => ObservationFieldCode::Reachable,
            Self::Required(_) => ObservationFieldCode::Required,
            Self::Adoptable(_) => ObservationFieldCode::Adoptable,
            Self::ProfileComponent(_) => ObservationFieldCode::ProfileComponent,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ObservationFields(BTreeMap<ObservationFieldCode, ObservationField>);

impl ObservationFields {
    pub fn new(
        fields: impl IntoIterator<Item = ObservationField>,
    ) -> Result<Self, ObservationFindingError> {
        let mut collected = BTreeMap::new();
        for field in fields {
            let code = field.code();
            if collected.insert(code, field).is_some() {
                return Err(ObservationFindingError::DuplicateField { code });
            }
            if collected.len() > MAX_FINDING_FIELDS {
                return Err(ObservationFindingError::TooManyFields {
                    max: MAX_FINDING_FIELDS,
                    actual: collected.len(),
                });
            }
        }
        Ok(Self(collected))
    }

    pub fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = (&ObservationFieldCode, &ObservationField)> {
        self.0.iter()
    }
    pub fn get(&self, code: ObservationFieldCode) -> Option<&ObservationField> {
        self.0.get(&code)
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
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (code, field) in &self.0 {
            match field {
                ObservationField::AffectedCount(value) => map.serialize_entry(code, value)?,
                ObservationField::ExpectedResourceKind(value)
                | ObservationField::ObservedResourceKind(value) => {
                    map.serialize_entry(code, value)?;
                }
                ObservationField::Capability(value) => map.serialize_entry(code, value)?,
                ObservationField::Layer(value) => map.serialize_entry(code, value)?,
                ObservationField::RelatedHarness(value) => map.serialize_entry(code, value)?,
                ObservationField::RelatedResource(value) => map.serialize_entry(code, value)?,
                ObservationField::Enabled(value)
                | ObservationField::Reachable(value)
                | ObservationField::Required(value)
                | ObservationField::Adoptable(value) => map.serialize_entry(code, value)?,
                ObservationField::ProfileComponent(value) => map.serialize_entry(code, value)?,
            }
        }
        map.end()
    }
}

struct ObservationFieldsVisitor;

impl<'de> Visitor<'de> for ObservationFieldsVisitor {
    type Value = ObservationFields;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a bounded map of registered observation fields")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut fields = BTreeMap::new();
        while let Some(code) = map.next_key::<ObservationFieldCode>()? {
            let field = match code {
                ObservationFieldCode::AffectedCount => {
                    ObservationField::AffectedCount(map.next_value()?)
                }
                ObservationFieldCode::ExpectedResourceKind => {
                    ObservationField::ExpectedResourceKind(map.next_value()?)
                }
                ObservationFieldCode::ObservedResourceKind => {
                    ObservationField::ObservedResourceKind(map.next_value()?)
                }
                ObservationFieldCode::Capability => ObservationField::Capability(map.next_value()?),
                ObservationFieldCode::Layer => ObservationField::Layer(map.next_value()?),
                ObservationFieldCode::RelatedHarness => {
                    ObservationField::RelatedHarness(map.next_value()?)
                }
                ObservationFieldCode::RelatedResource => {
                    ObservationField::RelatedResource(map.next_value()?)
                }
                ObservationFieldCode::Enabled => ObservationField::Enabled(map.next_value()?),
                ObservationFieldCode::Reachable => ObservationField::Reachable(map.next_value()?),
                ObservationFieldCode::Required => ObservationField::Required(map.next_value()?),
                ObservationFieldCode::Adoptable => ObservationField::Adoptable(map.next_value()?),
                ObservationFieldCode::ProfileComponent => {
                    ObservationField::ProfileComponent(map.next_value()?)
                }
            };
            if fields.insert(code, field).is_some() {
                return Err(serde::de::Error::custom("duplicate observation field"));
            }
            if fields.len() > MAX_FINDING_FIELDS {
                return Err(serde::de::Error::custom("too many observation fields"));
            }
        }
        Ok(ObservationFields(fields))
    }
}

impl<'de> Deserialize<'de> for ObservationFields {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(ObservationFieldsVisitor)
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
    pub const fn code(&self) -> ObservationFindingCode {
        self.code
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
    DuplicateField { code: ObservationFieldCode },
    TooManyFields { max: usize, actual: usize },
}

impl fmt::Display for ObservationFindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateField { code } => {
                write!(formatter, "duplicate observation field `{code}`")
            }
            Self::TooManyFields { max, actual } => {
                write!(
                    formatter,
                    "observation finding allows at most {max} fields, got {actual}"
                )
            }
        }
    }
}
impl std::error::Error for ObservationFindingError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{AbsolutePath, ResourceId};
    use serde_json::json;

    const RAW_SECRET: &str = "sk-test-auth=must-not-enter\n--token raw-secret";
    const IDENTIFIER_SECRET: &str = "native.secret.valid-identifier";

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
            ObservationFindingCode::NativeEntryMalformed,
            ObservationSummary::MalformedNativeEntry,
            ObservationSeverity::Warning,
            subject(),
            fields,
        )
    }

    #[test]
    fn authored_finding_round_trips_with_deterministic_typed_fields() {
        let fields = ObservationFields::new([
            ObservationField::ExpectedResourceKind(ResourceKind::Plugin),
            ObservationField::ProfileComponent(NativeId::new("pi-mcp-adapter").unwrap()),
            ObservationField::AffectedCount(2),
        ])
        .unwrap();
        let finding = finding(fields);
        let encoded = serde_json::to_string(&finding).unwrap();
        assert_eq!(
            serde_json::from_str::<ObservationFinding>(&encoded).unwrap(),
            finding
        );
        assert!(
            encoded.find("affected_count").unwrap()
                < encoded.find("expected_resource_kind").unwrap()
        );
        assert!(encoded.contains(r#""profile_component":"pi-mcp-adapter""#));
        assert_eq!(
            finding.subject().scope(),
            &Scope::Project(AbsolutePath::new("/work/project").unwrap())
        );
    }

    #[test]
    fn identifier_valid_secrets_are_not_registered_codes() {
        assert_eq!(
            ObservationFindingCode::from_registered(IDENTIFIER_SECRET),
            None
        );
        assert_eq!(
            ObservationFieldCode::from_registered(IDENTIFIER_SECRET),
            None
        );

        let mut payload = serde_json::to_value(finding(ObservationFields::default())).unwrap();
        payload["code"] = json!(IDENTIFIER_SECRET);
        let error = serde_json::from_value::<ObservationFinding>(payload).unwrap_err();
        assert!(!error.to_string().contains(IDENTIFIER_SECRET));

        let fields_error = serde_json::from_str::<ObservationFields>(&format!(
            r#"{{"{IDENTIFIER_SECRET}":true}}"#
        ))
        .unwrap_err();
        assert!(!fields_error.to_string().contains(IDENTIFIER_SECRET));

        for value in [
            serde_json::to_string(&finding(ObservationFields::default())).unwrap(),
            format!("{:?}", finding(ObservationFields::default())),
            finding(ObservationFields::default()).to_string(),
        ] {
            assert!(!value.contains(IDENTIFIER_SECRET));
        }
    }

    #[test]
    fn arbitrary_native_payload_channels_are_rejected() {
        let base = serde_json::to_value(finding(ObservationFields::default())).unwrap();
        for field in [
            "argv", "stdout", "stderr", "settings", "metadata", "message",
        ] {
            let mut payload = base.clone();
            payload[field] = json!(RAW_SECRET);
            assert!(serde_json::from_value::<ObservationFinding>(payload).is_err());
        }
        let mut dynamic_summary = base.clone();
        dynamic_summary["summary"] = json!(RAW_SECRET);
        assert!(serde_json::from_value::<ObservationFinding>(dynamic_summary).is_err());
        let mut raw_field = base;
        raw_field["fields"] = json!({"stdout":RAW_SECRET});
        assert!(serde_json::from_value::<ObservationFinding>(raw_field).is_err());
    }

    #[test]
    fn profile_component_field_rejects_arbitrary_payload_shapes() {
        let finding = finding(
            ObservationFields::new([ObservationField::ProfileComponent(
                NativeId::new("pi-hooks").unwrap(),
            )])
            .unwrap(),
        );
        let mut payload = serde_json::to_value(finding).unwrap();
        payload["fields"]["profile_component"] = json!({"stdout":"secret"});
        assert!(serde_json::from_value::<ObservationFinding>(payload).is_err());
    }

    #[test]
    fn duplicate_semantic_fields_are_rejected_by_constructor_and_serde() {
        assert!(matches!(
            ObservationFields::new([
                ObservationField::AffectedCount(1),
                ObservationField::AffectedCount(2),
            ]),
            Err(ObservationFindingError::DuplicateField {
                code: ObservationFieldCode::AffectedCount
            })
        ));
        let error =
            serde_json::from_str::<ObservationFields>(r#"{"affected_count":1,"affected_count":2}"#)
                .unwrap_err();
        assert!(error.to_string().contains("duplicate observation field"));
    }

    #[test]
    fn typed_fields_reject_wrong_scalar_types_and_owned_shapes_are_strict() {
        assert!(serde_json::from_str::<ObservationFields>(r#"{"affected_count":"two"}"#).is_err());
        assert!(serde_json::from_value::<ObservationFinding>(json!({
            "code":"native.entry.malformed", "summary":"A native entry is malformed.", "severity":"warning",
            "subject":{"kind":"harness","harness":"codex","scope":{"kind":"global"}}, "future":true
        })).is_err());
    }
}
