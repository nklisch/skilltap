//! Harness capability contracts.

use std::{collections::BTreeMap, fmt};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{ValidationError, validate_text};

const CAPABILITY_ID_MAX_BYTES: usize = 192;

/// An open, namespaced identifier for a harness operation or component.
///
/// Capability identifiers have at least two dot-separated segments. Segments
/// are intentionally not enumerated so adapters can add capabilities without a
/// core release.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CapabilityId(String);

impl CapabilityId {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        validate_text(&value, "capability id", CAPABILITY_ID_MAX_BYTES)?;

        let mut segments = value.split('.');
        let first = segments.next().expect("validated non-empty text");
        let second = segments.next();
        if !valid_segment(first)
            || second.is_none_or(|segment| !valid_segment(segment))
            || !segments.all(valid_segment)
        {
            return Err(ValidationError::InvalidFormat {
                kind: "capability id",
                expected: "contain at least two non-empty dot-separated lowercase ASCII segments using letters, digits, `-`, or `_`",
            });
        }

        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn valid_segment(segment: &str) -> bool {
    let mut characters = segment.chars();
    characters
        .next()
        .is_some_and(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
        && characters.all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || matches!(character, '-' | '_')
        })
}

impl fmt::Display for CapabilityId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl TryFrom<String> for CapabilityId {
    type Error = ValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl Serialize for CapabilityId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for CapabilityId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilitySupport {
    Supported,
    Unsupported,
    Unverified,
}

/// A deterministic capability profile observed for one harness installation.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilitySet {
    capabilities: BTreeMap<CapabilityId, CapabilitySupport>,
}

impl CapabilitySet {
    pub fn new(capabilities: impl IntoIterator<Item = (CapabilityId, CapabilitySupport)>) -> Self {
        Self {
            capabilities: capabilities.into_iter().collect(),
        }
    }

    pub fn support(&self, capability: &CapabilityId) -> Option<CapabilitySupport> {
        self.capabilities.get(capability).copied()
    }

    pub fn insert(
        &mut self,
        capability: CapabilityId,
        support: CapabilitySupport,
    ) -> Option<CapabilitySupport> {
        self.capabilities.insert(capability, support)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&CapabilityId, CapabilitySupport)> {
        self.capabilities
            .iter()
            .map(|(capability, support)| (capability, *support))
    }

    pub fn is_empty(&self) -> bool {
        self.capabilities.is_empty()
    }

    pub fn len(&self) -> usize {
        self.capabilities.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_ids_are_open_but_strictly_dotted() {
        for value in [
            "plugin.install",
            "component.future-kind.observe_v2",
            "vendor7.experimental-feature.apply",
        ] {
            assert_eq!(CapabilityId::new(value).unwrap().as_str(), value);
        }

        for value in [
            "plugin",
            ".install",
            "plugin.",
            "plugin..install",
            "Plugin.install",
            "plugin:install",
            " plugin.install",
        ] {
            assert!(
                CapabilityId::new(value).is_err(),
                "expected `{value}` to be rejected"
            );
        }
    }

    #[test]
    fn deserialization_cannot_bypass_capability_id_validation() {
        let error = serde_json::from_str::<CapabilityId>(r#""plugin..install""#).unwrap_err();
        assert!(error.to_string().contains("capability id must contain"));
    }

    #[test]
    fn capability_sets_preserve_all_three_support_states_deterministically() {
        let set = CapabilitySet::new([
            (
                CapabilityId::new("skill.update").unwrap(),
                CapabilitySupport::Unverified,
            ),
            (
                CapabilityId::new("plugin.install").unwrap(),
                CapabilitySupport::Supported,
            ),
            (
                CapabilityId::new("component.lsp").unwrap(),
                CapabilitySupport::Unsupported,
            ),
        ]);

        let json = serde_json::to_string(&set).unwrap();
        assert_eq!(
            json,
            r#"{"capabilities":{"component.lsp":"unsupported","plugin.install":"supported","skill.update":"unverified"}}"#
        );
        assert_eq!(serde_json::from_str::<CapabilitySet>(&json).unwrap(), set);
    }

    #[test]
    fn capability_sets_reject_unknown_fields_and_invalid_map_keys() {
        assert!(
            serde_json::from_str::<CapabilitySet>(
                r#"{"capabilities":{},"future_assumption":true}"#
            )
            .is_err()
        );
        assert!(
            serde_json::from_str::<CapabilitySet>(
                r#"{"capabilities":{"plugin..install":"supported"}}"#
            )
            .is_err()
        );
    }
}
