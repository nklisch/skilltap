use std::{
    collections::BTreeMap,
    ffi::OsString,
    fmt,
    path::{Component, Path},
    str::FromStr,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer, ser::SerializeMap};

use super::{CONFIG_SCHEMA_VERSION, SchemaError};
use crate::domain::{AbsolutePath, HarnessId, NativeId};

use crate::bootstrap::BootstrapUpdateMode;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HarnessBinary(String);

impl HarnessBinary {
    pub fn new(value: impl Into<String>) -> Result<Self, SchemaError> {
        let value = value.into();
        NativeId::new(value.clone())?;
        let path = Path::new(&value);
        if path.is_absolute() {
            AbsolutePath::new(value.clone())?;
            return Ok(Self(value));
        }

        let mut components = path.components();
        let Some(Component::Normal(component)) = components.next() else {
            return Err(SchemaError::InvalidHarnessBinary);
        };
        if components.next().is_some() || component.to_str() != Some(value.as_str()) {
            return Err(SchemaError::InvalidHarnessBinary);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<OsString> for HarnessBinary {
    type Error = SchemaError;

    fn try_from(value: OsString) -> Result<Self, Self::Error> {
        Self::new(
            value
                .into_string()
                .map_err(|_| SchemaError::NonUtf8HarnessBinary)?,
        )
    }
}

impl fmt::Display for HarnessBinary {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Serialize for HarnessBinary {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for HarnessBinary {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ClaudeInstructionMode {
    Symlink,
    Import,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum UpdateMode {
    Off,
    Check,
    ApplySafe,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UpdateIntervalUnit {
    Seconds,
    Minutes,
    Hours,
    Days,
}

impl UpdateIntervalUnit {
    const fn suffix(self) -> char {
        match self {
            Self::Seconds => 's',
            Self::Minutes => 'm',
            Self::Hours => 'h',
            Self::Days => 'd',
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UpdateInterval {
    value: u64,
    unit: UpdateIntervalUnit,
}

impl UpdateInterval {
    pub fn new(value: u64, unit: UpdateIntervalUnit) -> Result<Self, SchemaError> {
        if value == 0 {
            return Err(SchemaError::InvalidInterval);
        }
        Ok(Self { value, unit })
    }

    pub const fn value(self) -> u64 {
        self.value
    }

    pub const fn unit(self) -> UpdateIntervalUnit {
        self.unit
    }
}

impl fmt::Display for UpdateInterval {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}{}", self.value, self.unit.suffix())
    }
}

impl FromStr for UpdateInterval {
    type Err = SchemaError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (digits, unit) = [
            ('s', UpdateIntervalUnit::Seconds),
            ('m', UpdateIntervalUnit::Minutes),
            ('h', UpdateIntervalUnit::Hours),
            ('d', UpdateIntervalUnit::Days),
        ]
        .into_iter()
        .find_map(|(suffix, unit)| value.strip_suffix(suffix).map(|digits| (digits, unit)))
        .ok_or(SchemaError::InvalidInterval)?;
        let number = digits
            .parse::<u64>()
            .map_err(|_| SchemaError::InvalidInterval)?;
        if number.to_string() != digits {
            return Err(SchemaError::InvalidInterval);
        }
        Self::new(number, unit)
    }
}

impl Serialize for UpdateInterval {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for UpdateInterval {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HarnessPolicy {
    pub enabled: bool,
    pub binary: HarnessBinary,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct HarnessPolicyMap(BTreeMap<HarnessId, HarnessPolicy>);

impl HarnessPolicyMap {
    pub fn get(&self, id: &HarnessId) -> Option<&HarnessPolicy> {
        self.0.get(id)
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = (&HarnessId, &HarnessPolicy)> {
        self.0.iter()
    }

    /// Returns enabled harnesses in the established policy order. Keeping the
    /// original pair first preserves existing CLI output; additional ids are
    /// deterministic because the backing map is ordered.
    pub fn enabled(&self) -> impl Iterator<Item = &HarnessId> {
        self.stable_iter()
            .filter(|(_, policy)| policy.enabled)
            .map(|(id, _)| id)
    }

    pub fn with_policy(
        &self,
        id: HarnessId,
        enabled: bool,
        binary: Option<&HarnessBinary>,
    ) -> Self {
        let mut policies = self.0.clone();
        let binary = binary.cloned().unwrap_or_else(|| {
            policies
                .get(&id)
                .map(|policy| policy.binary.clone())
                .unwrap_or_else(|| {
                    // Harness identifiers are valid single PATH components, so
                    // they are also valid first-use PATH-lookup binary names.
                    HarnessBinary::new(id.as_str()).expect("validated harness id is a PATH name")
                })
        });
        policies.insert(id, HarnessPolicy { enabled, binary });
        Self(policies)
    }

    fn stable_iter(&self) -> impl Iterator<Item = (&HarnessId, &HarnessPolicy)> {
        ["codex", "claude"]
            .into_iter()
            .filter_map(|id| {
                self.0
                    .iter()
                    .find(|(candidate, _)| candidate.as_str() == id)
            })
            .chain(
                self.0
                    .iter()
                    .filter(|(id, _)| !matches!(id.as_str(), "codex" | "claude")),
            )
    }
}

impl Serialize for HarnessPolicyMap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        // TOML table order is not semantic, but retaining the established pair
        // first keeps schema-1 documents byte-stable for existing defaults.
        for (id, policy) in self.stable_iter() {
            map.serialize_entry(id, policy)?;
        }
        map.end()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InstructionPolicy {
    pub claude_mode: ClaudeInstructionMode,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UpdatePolicy {
    pub mode: UpdateMode,
    pub interval: UpdateInterval,
}

/// Policy for skilltap's own binary update lifecycle.  It is kept separate
/// from resource update policy because major-version acknowledgement applies
/// only to the self-hosted executable.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BinaryUpdatePolicy {
    pub mode: BootstrapUpdateMode,
    pub allow_major: bool,
}

impl Default for BinaryUpdatePolicy {
    fn default() -> Self {
        Self {
            mode: BootstrapUpdateMode::ApplySafe,
            allow_major: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(into = "ConfigWire")]
pub struct ConfigDocument {
    harnesses: HarnessPolicyMap,
    instructions: InstructionPolicy,
    updates: UpdatePolicy,
    #[serde(default)]
    bootstrap: BinaryUpdatePolicy,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ConfigWire {
    schema: u32,
    harnesses: HarnessPolicyMap,
    instructions: InstructionPolicy,
    updates: UpdatePolicy,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_default_binary_policy")]
    bootstrap: BinaryUpdatePolicy,
}

fn is_default_binary_policy(policy: &BinaryUpdatePolicy) -> bool {
    policy == &BinaryUpdatePolicy::default()
}

impl ConfigDocument {
    pub const fn schema(&self) -> u32 {
        CONFIG_SCHEMA_VERSION
    }

    pub fn new(
        schema: u32,
        harnesses: HarnessPolicyMap,
        instructions: InstructionPolicy,
        updates: UpdatePolicy,
    ) -> Result<Self, SchemaError> {
        if schema != CONFIG_SCHEMA_VERSION {
            return Err(SchemaError::UnsupportedVersion {
                document: "config",
                version: schema,
            });
        }
        Ok(Self {
            harnesses,
            instructions,
            updates,
            bootstrap: BinaryUpdatePolicy::default(),
        })
    }

    pub fn defaults() -> Self {
        Self {
            harnesses: HarnessPolicyMap(BTreeMap::from([
                (
                    HarnessId::new("codex").expect("known valid harness id"),
                    HarnessPolicy {
                        enabled: false,
                        binary: HarnessBinary::new("codex").expect("known valid binary"),
                    },
                ),
                (
                    HarnessId::new("claude").expect("known valid harness id"),
                    HarnessPolicy {
                        enabled: false,
                        binary: HarnessBinary::new("claude").expect("known valid binary"),
                    },
                ),
            ])),
            instructions: InstructionPolicy {
                claude_mode: ClaudeInstructionMode::Symlink,
            },
            updates: UpdatePolicy {
                mode: UpdateMode::ApplySafe,
                interval: UpdateInterval::new(6, UpdateIntervalUnit::Hours)
                    .expect("known positive interval"),
            },
            bootstrap: BinaryUpdatePolicy::default(),
        }
    }

    pub const fn harnesses(&self) -> &HarnessPolicyMap {
        &self.harnesses
    }
    pub const fn instructions(&self) -> &InstructionPolicy {
        &self.instructions
    }
    pub const fn updates(&self) -> &UpdatePolicy {
        &self.updates
    }

    pub const fn bootstrap(&self) -> &BinaryUpdatePolicy {
        &self.bootstrap
    }

    pub fn with_bootstrap_policy(&self, policy: BinaryUpdatePolicy) -> Self {
        let mut next = self.clone();
        next.bootstrap = policy;
        next
    }

    /// Returns a policy copy with one structurally valid harness entry updated.
    /// Registry membership is enforced by the CLI composition boundary.
    pub fn with_harness_policy(
        &self,
        harness: &HarnessId,
        enabled: bool,
        binary: Option<&HarnessBinary>,
    ) -> Result<Self, SchemaError> {
        let mut next = self.clone();
        next.harnesses = self.harnesses.with_policy(harness.clone(), enabled, binary);
        Ok(next)
    }

    /// Returns a policy copy with only one harness enabled/disabled.
    pub fn with_harness_enabled(
        &self,
        harness: &HarnessId,
        enabled: bool,
    ) -> Result<Self, SchemaError> {
        self.with_harness_policy(harness, enabled, None)
    }
}

impl From<ConfigDocument> for ConfigWire {
    fn from(value: ConfigDocument) -> Self {
        Self {
            schema: CONFIG_SCHEMA_VERSION,
            harnesses: value.harnesses,
            instructions: value.instructions,
            updates: value.updates,
            bootstrap: value.bootstrap,
        }
    }
}

impl TryFrom<ConfigWire> for ConfigDocument {
    type Error = SchemaError;
    fn try_from(value: ConfigWire) -> Result<Self, Self::Error> {
        Self::new(
            value.schema,
            value.harnesses,
            value.instructions,
            value.updates,
        )
        .map(|mut config| {
            config.bootstrap = value.bootstrap;
            config
        })
    }
}

impl<'de> Deserialize<'de> for ConfigDocument {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        ConfigWire::deserialize(deserializer)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}
