use std::{collections::BTreeSet, fmt, path::Component};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{HarnessId, ValidationError, validate_text};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AbsolutePath(String);

impl AbsolutePath {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        validate_text(&value, "absolute path", 4096)?;
        let path = std::path::Path::new(&value);
        if !path.is_absolute() {
            return Err(ValidationError::PathNotAbsolute);
        }
        if path
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
        {
            return Err(ValidationError::InvalidAbsolutePathComponent);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RelativeArtifactPath(String);

impl RelativeArtifactPath {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        validate_text(&value, "artifact path", 4096)?;
        let path = std::path::Path::new(&value);
        if path.is_absolute() {
            return Err(ValidationError::PathNotRelative);
        }
        if path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(ValidationError::InvalidRelativePathComponent);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

macro_rules! path_serde {
    ($name:ident) => {
        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(&self.0)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::new(value).map_err(serde::de::Error::custom)
            }
        }
    };
}

path_serde!(AbsolutePath);
path_serde!(RelativeArtifactPath);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "path", rename_all = "snake_case")]
pub enum Scope {
    Global,
    Project(AbsolutePath),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "path", rename_all = "snake_case")]
pub enum ScopeSelection {
    Global,
    Project(AbsolutePath),
    AllScopes,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HarnessSet(BTreeSet<HarnessId>);

impl HarnessSet {
    pub fn new(harnesses: impl IntoIterator<Item = HarnessId>) -> Result<Self, ValidationError> {
        let harnesses = harnesses.into_iter().collect::<BTreeSet<_>>();
        if harnesses.is_empty() {
            return Err(ValidationError::EmptyHarnessSet);
        }
        Ok(Self(harnesses))
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = &HarnessId> {
        self.0.iter()
    }

    pub fn contains(&self, harness: &HarnessId) -> bool {
        self.0.contains(harness)
    }
}

impl Serialize for HarnessSet {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for HarnessSet {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let harnesses = BTreeSet::<HarnessId>::deserialize(deserializer)?;
        Self::new(harnesses).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "harness", rename_all = "snake_case")]
pub enum TargetSelection {
    All,
    Only(HarnessId),
}

impl TargetSelection {
    pub fn resolve(&self, enabled: &HarnessSet) -> Result<HarnessSet, ValidationError> {
        match self {
            Self::All => Ok(enabled.clone()),
            Self::Only(harness) if enabled.contains(harness) => HarnessSet::new([harness.clone()]),
            Self::Only(harness) => Err(ValidationError::HarnessNotEnabled {
                harness: harness.clone(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn harness(value: &str) -> HarnessId {
        HarnessId::new(value).unwrap()
    }

    #[test]
    fn paths_reject_wrong_or_non_normalized_forms() {
        assert_eq!(
            AbsolutePath::new("projects/skilltap").unwrap_err(),
            ValidationError::PathNotAbsolute
        );
        assert_eq!(
            AbsolutePath::new("/home/nathan/../skilltap").unwrap_err(),
            ValidationError::InvalidAbsolutePathComponent
        );
        for invalid in ["/absolute", "../escape", "assets/../escape", "."] {
            assert!(RelativeArtifactPath::new(invalid).is_err(), "{invalid}");
        }
        assert_eq!(
            serde_json::from_str::<RelativeArtifactPath>(r#""../escape""#)
                .unwrap_err()
                .to_string(),
            ValidationError::InvalidRelativePathComponent.to_string()
        );
    }

    #[test]
    fn harness_sets_are_non_empty_sorted_and_unique() {
        assert_eq!(
            HarnessSet::new(Vec::<HarnessId>::new()).unwrap_err(),
            ValidationError::EmptyHarnessSet
        );
        let set = HarnessSet::new([harness("codex"), harness("claude"), harness("codex")]).unwrap();
        assert_eq!(
            set.iter().map(HarnessId::as_str).collect::<Vec<_>>(),
            ["claude", "codex"]
        );
        assert_eq!(
            serde_json::to_string(&set).unwrap(),
            r#"["claude","codex"]"#
        );
        assert!(serde_json::from_str::<HarnessSet>("[]").is_err());
    }

    #[test]
    fn targets_resolve_against_enabled_harnesses() {
        let enabled = HarnessSet::new([harness("codex"), harness("claude")]).unwrap();
        assert_eq!(
            TargetSelection::All
                .resolve(&enabled)
                .unwrap()
                .iter()
                .map(HarnessId::as_str)
                .collect::<Vec<_>>(),
            ["claude", "codex"]
        );
        assert_eq!(
            TargetSelection::Only(harness("codex"))
                .resolve(&enabled)
                .unwrap()
                .iter()
                .map(HarnessId::as_str)
                .collect::<Vec<_>>(),
            ["codex"]
        );
        assert!(matches!(
            TargetSelection::Only(harness("pi")).resolve(&enabled),
            Err(ValidationError::HarnessNotEnabled { .. })
        ));
    }

    #[test]
    fn scope_and_target_enums_have_stable_snake_case_forms() {
        let scope = ScopeSelection::Project(AbsolutePath::new("/tmp/project").unwrap());
        assert_eq!(
            serde_json::to_string(&scope).unwrap(),
            r#"{"kind":"project","path":"/tmp/project"}"#
        );
        assert_eq!(
            serde_json::to_string(&ScopeSelection::AllScopes).unwrap(),
            r#"{"kind":"all_scopes"}"#
        );
        let target = TargetSelection::Only(harness("claude"));
        let json = serde_json::to_string(&target).unwrap();
        assert_eq!(json, r#"{"kind":"only","harness":"claude"}"#);
        assert_eq!(
            serde_json::from_str::<TargetSelection>(&json).unwrap(),
            target
        );
    }
}
