//! Strict Agent Skills metadata validation and target loadability evidence.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_yaml::Value;

use crate::{
    domain::{CompatibilityClass, HarnessId, HarnessSet},
    skill::ValidatedSkillTree,
};

/// The normative Agent Skills directory/name identifier.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AgentSkillName(String);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AgentSkillNameError {
    Empty,
    TooLong { actual: usize },
    InvalidCharacter { index: usize },
    InvalidBoundary,
    ConsecutiveHyphens,
}

impl fmt::Display for AgentSkillNameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("skill name must not be empty"),
            Self::TooLong { actual } => {
                write!(
                    formatter,
                    "skill name must be at most 64 characters, got {actual}"
                )
            }
            Self::InvalidCharacter { index } => write!(
                formatter,
                "skill name contains a non-lowercase ASCII letter, digit, or hyphen at byte {index}"
            ),
            Self::InvalidBoundary => {
                formatter.write_str("skill name must not begin or end with a hyphen")
            }
            Self::ConsecutiveHyphens => {
                formatter.write_str("skill name must not contain consecutive hyphens")
            }
        }
    }
}

impl std::error::Error for AgentSkillNameError {}

impl AgentSkillName {
    pub fn new(value: impl Into<String>) -> Result<Self, AgentSkillNameError> {
        let value = value.into();
        let length = value.chars().count();
        if value.is_empty() {
            return Err(AgentSkillNameError::Empty);
        }
        if length > 64 {
            return Err(AgentSkillNameError::TooLong { actual: length });
        }
        if value.starts_with('-') || value.ends_with('-') {
            return Err(AgentSkillNameError::InvalidBoundary);
        }
        if value.contains("--") {
            return Err(AgentSkillNameError::ConsecutiveHyphens);
        }
        for (index, character) in value.char_indices() {
            if !(character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-') {
                return Err(AgentSkillNameError::InvalidCharacter { index });
            }
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AgentSkillName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Serialize for AgentSkillName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for AgentSkillName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

impl TryFrom<String> for AgentSkillName {
    type Error = AgentSkillNameError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentSkillMetadata {
    pub name: AgentSkillName,
    pub description: String,
    pub license: Option<String>,
    pub compatibility: Option<String>,
    pub metadata: BTreeMap<String, String>,
    pub allowed_tools: Option<String>,
    pub extension_fields: BTreeSet<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgentSkillConformance {
    Conforming,
    Nonconforming,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AgentSkillFormatFinding {
    InvalidUtf8,
    MissingFrontmatter,
    UnterminatedFrontmatter,
    InvalidYaml,
    FrontmatterNotMapping,
    MissingName,
    InvalidName,
    DirectoryNameMismatch,
    MissingDescription,
    DescriptionTooLong,
    InvalidLicense,
    InvalidCompatibility,
    InvalidMetadata,
    InvalidAllowedTools,
    ExtensionField(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentSkillValidation {
    metadata: Option<AgentSkillMetadata>,
    conformance: AgentSkillConformance,
    loadable_shape: bool,
    findings: Vec<AgentSkillFormatFinding>,
}

impl AgentSkillValidation {
    pub fn metadata(&self) -> Option<&AgentSkillMetadata> {
        self.metadata.as_ref()
    }

    pub const fn conformance(&self) -> AgentSkillConformance {
        self.conformance
    }

    pub const fn is_conforming(&self) -> bool {
        matches!(self.conformance, AgentSkillConformance::Conforming)
    }

    pub const fn loadable_shape(&self) -> bool {
        self.loadable_shape
    }

    pub fn findings(&self) -> &[AgentSkillFormatFinding] {
        &self.findings
    }
}

/// Validate the bounded top-level SKILL.md captured in a complete tree.
///
/// This function intentionally receives the already validated tree rather than
/// reopening its source path. The complete-tree snapshot is the authority for
/// both metadata and publication, so source replacement cannot change the
/// result between validation and planning.
pub fn validate_agent_skill(
    tree: &ValidatedSkillTree,
    directory_name: &AgentSkillName,
) -> AgentSkillValidation {
    let bytes = tree
        .tree()
        .files()
        .iter()
        .find(|(path, _)| path.as_str() == "SKILL.md")
        .expect("validated skill tree contains SKILL.md")
        .1
        .contents();
    let Ok(text) = std::str::from_utf8(bytes) else {
        return invalid_validation(vec![AgentSkillFormatFinding::InvalidUtf8], false);
    };

    let lines = text
        .split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line));
    let lines = lines.collect::<Vec<_>>();
    if lines.first().copied() != Some("---") {
        return invalid_validation(vec![AgentSkillFormatFinding::MissingFrontmatter], false);
    }
    let closing = lines.iter().skip(1).position(|line| *line == "---");
    let (end, unterminated) = match closing {
        Some(end) => (end + 1, false),
        None => (lines.len(), true),
    };
    let yaml = lines[1..end].join("\n");
    let value = match serde_yaml::from_str::<Value>(&yaml) {
        Ok(value) => value,
        Err(_) if unterminated => {
            // Some clients tolerate a missing delimiter and stop reading when
            // the metadata mapping ends. Retain that loadability evidence
            // without weakening strict conformance: bound the tolerant parse
            // at the first unindented body line.
            let mut metadata_lines = Vec::new();
            for line in &lines[1..end] {
                if !metadata_lines.is_empty()
                    && !line.starts_with(char::is_whitespace)
                    && !line.contains(':')
                {
                    break;
                }
                metadata_lines.push(*line);
            }
            let candidate = metadata_lines.join("\n");
            let Ok(value) = serde_yaml::from_str::<Value>(&candidate) else {
                return invalid_validation(vec![AgentSkillFormatFinding::InvalidYaml], false);
            };
            value
        }
        Err(_) => {
            return invalid_validation(vec![AgentSkillFormatFinding::InvalidYaml], false);
        }
    };
    let Value::Mapping(mapping) = value else {
        return invalid_validation(vec![AgentSkillFormatFinding::FrontmatterNotMapping], false);
    };

    let mut findings = Vec::new();
    if unterminated {
        findings.push(AgentSkillFormatFinding::UnterminatedFrontmatter);
    }
    let mut name = None;
    let mut description = None;
    let mut license = None;
    let mut compatibility = None;
    let mut metadata = BTreeMap::new();
    let mut allowed_tools = None;
    let mut extension_fields = BTreeSet::new();
    let mut required_name_present = false;
    let mut required_description_present = false;

    for (key, value) in mapping {
        let Some(key) = key.as_str() else {
            findings.push(AgentSkillFormatFinding::InvalidMetadata);
            continue;
        };
        match key {
            "name" => {
                required_name_present = true;
                match value
                    .as_str()
                    .and_then(|value| AgentSkillName::new(value).ok())
                {
                    Some(value) => {
                        if value != *directory_name {
                            findings.push(AgentSkillFormatFinding::DirectoryNameMismatch);
                        }
                        name = Some(value);
                    }
                    None => findings.push(AgentSkillFormatFinding::InvalidName),
                }
            }
            "description" => {
                required_description_present = true;
                match value.as_str() {
                    Some(value) if !value.is_empty() => {
                        if value.chars().count() > 1024 {
                            findings.push(AgentSkillFormatFinding::DescriptionTooLong);
                        }
                        description = Some(value.to_owned());
                    }
                    _ => findings.push(AgentSkillFormatFinding::MissingDescription),
                }
            }
            "license" => match value.as_str() {
                Some(value) if !value.is_empty() => license = Some(value.to_owned()),
                _ => findings.push(AgentSkillFormatFinding::InvalidLicense),
            },
            "compatibility" => match value.as_str() {
                Some(value) if !value.is_empty() && value.chars().count() <= 500 => {
                    compatibility = Some(value.to_owned());
                }
                _ => findings.push(AgentSkillFormatFinding::InvalidCompatibility),
            },
            "metadata" => {
                let Value::Mapping(values) = value else {
                    findings.push(AgentSkillFormatFinding::InvalidMetadata);
                    continue;
                };
                let mut valid = true;
                for (metadata_key, metadata_value) in values {
                    let (Some(metadata_key), Some(metadata_value)) =
                        (metadata_key.as_str(), metadata_value.as_str())
                    else {
                        valid = false;
                        continue;
                    };
                    metadata.insert(metadata_key.to_owned(), metadata_value.to_owned());
                }
                if !valid {
                    findings.push(AgentSkillFormatFinding::InvalidMetadata);
                }
            }
            "allowed-tools" => match value.as_str() {
                Some(value) => allowed_tools = Some(value.to_owned()),
                None => findings.push(AgentSkillFormatFinding::InvalidAllowedTools),
            },
            other => {
                extension_fields.insert(other.to_owned());
                findings.push(AgentSkillFormatFinding::ExtensionField(other.to_owned()));
            }
        }
    }
    if !required_name_present {
        findings.push(AgentSkillFormatFinding::MissingName);
    }
    if !required_description_present {
        findings.push(AgentSkillFormatFinding::MissingDescription);
    }

    // A client may tolerate a strict-format violation, but it still needs the
    // required scalar shape before it can load the skill at all.
    let loadable_shape = required_name_present
        && required_description_present
        && name.is_some()
        && description.is_some();
    let metadata = match (name, description) {
        (Some(name), Some(description)) => Some(AgentSkillMetadata {
            name,
            description,
            license,
            compatibility,
            metadata,
            allowed_tools,
            extension_fields,
        }),
        _ => None,
    };
    let conformance = if findings.is_empty() {
        AgentSkillConformance::Conforming
    } else {
        AgentSkillConformance::Nonconforming
    };
    AgentSkillValidation {
        metadata,
        conformance,
        loadable_shape,
        findings,
    }
}

fn invalid_validation(
    findings: Vec<AgentSkillFormatFinding>,
    loadable_shape: bool,
) -> AgentSkillValidation {
    AgentSkillValidation {
        metadata: None,
        conformance: AgentSkillConformance::Nonconforming,
        loadable_shape,
        findings,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SkillLoadability {
    Loadable,
    Unknown,
    Blocked,
}

/// Compatibility is target-specific evidence, not a synonym for strict format
/// conformance. The default is conservative until an adapter supplies stronger
/// loadability evidence for a nonconforming but parseable document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SkillCompatibility {
    target: HarnessId,
    class: CompatibilityClass,
    loadability: SkillLoadability,
    findings: Vec<SkillCompatibilityFinding>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SkillCompatibilityFinding {
    Format(AgentSkillFormatFinding),
    TargetEvidenceUnavailable,
}

/// Compatibility names retained for callers that used the pre-contract
/// classification. New code should inspect `CompatibilityClass` and
/// `SkillLoadability` directly.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SkillCompatibilityClass {
    Compatible,
    Warning,
    Blocked,
}

impl SkillCompatibility {
    /// Construct conservative blocked evidence when the canonical tree could
    /// not be validated far enough to inspect its frontmatter.
    pub fn blocked(target: HarnessId) -> Self {
        Self {
            target,
            class: CompatibilityClass::Incompatible,
            loadability: SkillLoadability::Blocked,
            findings: vec![SkillCompatibilityFinding::TargetEvidenceUnavailable],
        }
    }

    pub fn portable(target: HarnessId, validation: &AgentSkillValidation) -> Self {
        let (class, loadability) = if !validation.loadable_shape {
            (CompatibilityClass::Incompatible, SkillLoadability::Blocked)
        } else if validation.is_conforming() {
            (CompatibilityClass::Compatible, SkillLoadability::Loadable)
        } else {
            (CompatibilityClass::Unknown, SkillLoadability::Unknown)
        };
        Self {
            target,
            class,
            loadability,
            findings: validation
                .findings()
                .iter()
                .cloned()
                .map(SkillCompatibilityFinding::Format)
                .collect(),
        }
    }

    /// Existing non-project lifecycle callers do not have a directory name at
    /// this boundary. Use the declared name as the comparison name so this
    /// helper remains a parser/compatibility check, while project lifecycle
    /// callers use `validate_agent_skill` with the canonical directory name.
    pub fn evaluate(tree: &ValidatedSkillTree, targets: &HarnessSet) -> Vec<Self> {
        let directory_name = tree
            .declared_name()
            .and_then(|name| AgentSkillName::new(name.as_str()).ok())
            .unwrap_or_else(|| AgentSkillName::new("skill").expect("static skill name is valid"));
        let validation = validate_agent_skill(tree, &directory_name);
        targets
            .iter()
            .map(|target| Self::portable(target.clone(), &validation))
            .collect()
    }

    pub const fn target(&self) -> &HarnessId {
        &self.target
    }

    pub const fn class(&self) -> CompatibilityClass {
        self.class
    }

    pub const fn loadability(&self) -> SkillLoadability {
        self.loadability
    }

    pub const fn strict_agent_skills(&self) -> bool {
        matches!(self.class, CompatibilityClass::Compatible)
    }

    pub const fn loadable(&self) -> bool {
        matches!(self.loadability, SkillLoadability::Loadable)
    }

    pub const fn legacy_class(&self) -> SkillCompatibilityClass {
        match (self.class, self.loadability) {
            (CompatibilityClass::Compatible, SkillLoadability::Loadable) => {
                SkillCompatibilityClass::Compatible
            }
            (_, SkillLoadability::Blocked) => SkillCompatibilityClass::Blocked,
            _ => SkillCompatibilityClass::Warning,
        }
    }

    pub fn findings(&self) -> &[SkillCompatibilityFinding] {
        &self.findings
    }
}

impl fmt::Display for AgentSkillFormatFinding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExtensionField(field) => write!(formatter, "extension field `{field}`"),
            finding => write!(formatter, "{finding:?}"),
        }
    }
}

impl fmt::Display for SkillCompatibilityFinding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Format(finding) => finding.fmt(formatter),
            Self::TargetEvidenceUnavailable => {
                formatter.write_str("target loadability evidence is unavailable")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::RelativeArtifactPath,
        runtime::{ExternalTreeEntry, ExternalTreeLimits, ExternalTreeSnapshot},
        skill::ValidatedSkillTree,
    };

    fn tree(content: &[u8]) -> ValidatedSkillTree {
        let snapshot = ExternalTreeSnapshot::new(
            [ExternalTreeEntry::file(
                RelativeArtifactPath::new("SKILL.md").unwrap(),
                content.to_vec(),
                false,
            )],
            ExternalTreeLimits::new(8, 32, 1024, 4096, 1024).unwrap(),
        )
        .unwrap();
        ValidatedSkillTree::validate(&snapshot).unwrap()
    }

    #[test]
    fn validates_normative_metadata_and_preserves_extensions() {
        let validation = validate_agent_skill(
            &tree(
                b"---\nname: demo-skill\ndescription: A useful skill\nlicense: MIT\ncompatibility: Rust\nmetadata:\n  author: nathan\nallowed-tools: Read\nfuture-field: keep\n---\nbody\n",
            ),
            &AgentSkillName::new("demo-skill").unwrap(),
        );
        assert_eq!(
            validation.conformance(),
            AgentSkillConformance::Nonconforming
        );
        assert!(validation.loadable_shape());
        assert_eq!(validation.metadata().unwrap().metadata["author"], "nathan");
        assert!(
            validation
                .metadata()
                .unwrap()
                .extension_fields
                .contains("future-field")
        );
        assert!(matches!(
            SkillCompatibility::portable(HarnessId::new("claude").unwrap(), &validation).class(),
            CompatibilityClass::Unknown
        ));
    }

    #[test]
    fn malformed_required_metadata_is_blocked() {
        let validation = validate_agent_skill(
            &tree(b"---\nname: demo\n---\nbody\n"),
            &AgentSkillName::new("demo").unwrap(),
        );
        assert!(!validation.loadable_shape());
        assert!(
            validation
                .findings()
                .contains(&AgentSkillFormatFinding::MissingDescription)
        );
        let compatibility =
            SkillCompatibility::portable(HarnessId::new("codex").unwrap(), &validation);
        assert_eq!(compatibility.class(), CompatibilityClass::Incompatible);
        assert_eq!(compatibility.loadability(), SkillLoadability::Blocked);
    }

    #[test]
    fn name_rules_and_directory_matching_are_strict() {
        for invalid in ["", "-demo", "demo-", "demo--skill", "Demo", "demo_skill"] {
            assert!(AgentSkillName::new(invalid).is_err(), "{invalid}");
        }
        let validation = validate_agent_skill(
            &tree(b"---\nname: other\ndescription: okay\n---\n"),
            &AgentSkillName::new("demo").unwrap(),
        );
        assert!(
            validation
                .findings()
                .contains(&AgentSkillFormatFinding::DirectoryNameMismatch)
        );
    }

    #[test]
    fn frontmatter_without_yaml_or_utf8_is_not_loadable() {
        for bytes in [
            b"body\n".as_slice(),
            b"---\nname: demo\n".as_slice(),
            &[0xff],
        ] {
            let validation =
                validate_agent_skill(&tree(bytes), &AgentSkillName::new("demo").unwrap());
            assert!(!validation.loadable_shape());
        }
    }
}
