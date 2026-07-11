use std::{
    ffi::OsString,
    fmt,
    path::{Component, Path},
    str::FromStr,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{SCHEMA_VERSION, SchemaError};
use crate::domain::{AbsolutePath, NativeId};

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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HarnessPolicies {
    pub codex: HarnessPolicy,
    pub claude: HarnessPolicy,
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(into = "ConfigWire")]
pub struct ConfigDocument {
    harnesses: HarnessPolicies,
    instructions: InstructionPolicy,
    updates: UpdatePolicy,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ConfigWire {
    schema: u32,
    harnesses: HarnessPolicies,
    instructions: InstructionPolicy,
    updates: UpdatePolicy,
}

impl ConfigDocument {
    pub const fn schema(&self) -> u32 {
        SCHEMA_VERSION
    }

    pub fn new(
        schema: u32,
        harnesses: HarnessPolicies,
        instructions: InstructionPolicy,
        updates: UpdatePolicy,
    ) -> Result<Self, SchemaError> {
        if schema != SCHEMA_VERSION {
            return Err(SchemaError::UnsupportedVersion {
                document: "config",
                version: schema,
            });
        }
        Ok(Self {
            harnesses,
            instructions,
            updates,
        })
    }

    pub fn defaults() -> Self {
        Self {
            harnesses: HarnessPolicies {
                codex: HarnessPolicy {
                    enabled: true,
                    binary: HarnessBinary::new("codex").expect("known valid binary"),
                },
                claude: HarnessPolicy {
                    enabled: true,
                    binary: HarnessBinary::new("claude").expect("known valid binary"),
                },
            },
            instructions: InstructionPolicy {
                claude_mode: ClaudeInstructionMode::Symlink,
            },
            updates: UpdatePolicy {
                mode: UpdateMode::ApplySafe,
                interval: UpdateInterval::new(6, UpdateIntervalUnit::Hours)
                    .expect("known positive interval"),
            },
        }
    }

    pub const fn harnesses(&self) -> &HarnessPolicies {
        &self.harnesses
    }
    pub const fn instructions(&self) -> &InstructionPolicy {
        &self.instructions
    }
    pub const fn updates(&self) -> &UpdatePolicy {
        &self.updates
    }
}

impl From<ConfigDocument> for ConfigWire {
    fn from(value: ConfigDocument) -> Self {
        Self {
            schema: SCHEMA_VERSION,
            harnesses: value.harnesses,
            instructions: value.instructions,
            updates: value.updates,
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
