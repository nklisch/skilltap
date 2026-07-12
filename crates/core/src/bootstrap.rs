//! Pure release and binary bootstrap policy contracts.
//!
//! This module deliberately contains no filesystem, network, process, or
//! harness concerns.  Runtime adapters consume the validated values here.

use std::{fmt, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    domain::{Fingerprint, FingerprintAlgorithm, SourceLocator, ValidationError},
    runtime::{RuntimeError, SupportedPlatform},
    storage::UpdateMode,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BootstrapTarget {
    Codex,
    Claude,
    All,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ReleaseVersion {
    major: u64,
    minor: u64,
    patch: u64,
}

impl ReleaseVersion {
    pub const fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    pub const fn major(self) -> u64 {
        self.major
    }
    pub const fn minor(self) -> u64 {
        self.minor
    }
    pub const fn patch(self) -> u64 {
        self.patch
    }
}

impl fmt::Display for ReleaseVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl FromStr for ReleaseVersion {
    type Err = ValidationError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let parts = value.split('.').collect::<Vec<_>>();
        if parts.len() != 3 || parts.iter().any(|part| part.is_empty()) {
            return Err(ValidationError::InvalidFormat {
                kind: "release version",
                expected: "be numeric `major.minor.patch`",
            });
        }
        let parse = |part: &str| {
            if part != "0" && part.starts_with('0') {
                return Err(ValidationError::InvalidFormat {
                    kind: "release version",
                    expected: "use canonical numeric components",
                });
            }
            part.parse::<u64>()
                .map_err(|_| ValidationError::InvalidFormat {
                    kind: "release version",
                    expected: "contain numeric components",
                })
        };
        Ok(Self::new(
            parse(parts[0])?,
            parse(parts[1])?,
            parse(parts[2])?,
        ))
    }
}

impl Serialize for ReleaseVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for ReleaseVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArtifactArch {
    X86_64,
    Aarch64,
}

impl ArtifactArch {
    pub const fn asset_name(self) -> &'static str {
        match self {
            Self::X86_64 => "x86_64",
            Self::Aarch64 => "aarch64",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct ArtifactKey {
    pub platform: SupportedPlatform,
    pub arch: ArtifactArch,
}

impl ArtifactKey {
    pub fn current() -> Result<Self, RuntimeError> {
        let platform = SupportedPlatform::current()?;
        let arch = match std::env::consts::ARCH {
            "x86_64" => ArtifactArch::X86_64,
            "aarch64" => ArtifactArch::Aarch64,
            value => {
                return Err(RuntimeError::UnsupportedPlatform {
                    platform: value.to_owned(),
                });
            }
        };
        Ok(Self { platform, arch })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(into = "ReleaseArtifactWire")]
pub struct ReleaseArtifact {
    version: ReleaseVersion,
    key: ArtifactKey,
    asset_name: String,
    sha256: String,
    download_url: SourceLocator,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ReleaseArtifactWire {
    version: ReleaseVersion,
    key: ArtifactKey,
    asset_name: String,
    sha256: String,
    download_url: SourceLocator,
}

impl ReleaseArtifact {
    pub fn new(
        version: ReleaseVersion,
        key: ArtifactKey,
        asset_name: impl Into<String>,
        sha256: impl Into<String>,
        download_url: SourceLocator,
    ) -> Result<Self, ValidationError> {
        let asset_name = asset_name.into();
        if asset_name.is_empty()
            || asset_name == "."
            || asset_name == ".."
            || asset_name.contains(['/', '\\'])
            || asset_name.starts_with('-')
        {
            return Err(ValidationError::InvalidFormat {
                kind: "release asset name",
                expected: "be a non-empty filename without path traversal, separators, or option prefixes",
            });
        }
        if let Some(index) = asset_name.bytes().position(|byte| byte.is_ascii_control()) {
            return Err(ValidationError::ControlCharacter {
                kind: "release asset name",
                index,
            });
        }
        let fingerprint = Fingerprint::new(FingerprintAlgorithm::Sha256, sha256.into())?;
        let sha256 = fingerprint.digest().to_owned();
        if download_url.as_str().starts_with('-') {
            return Err(ValidationError::InvalidFormat {
                kind: "release download URL",
                expected: "not begin with `-`",
            });
        }
        Ok(Self {
            version,
            key,
            asset_name,
            sha256,
            download_url,
        })
    }

    pub const fn version(&self) -> ReleaseVersion {
        self.version
    }

    pub const fn key(&self) -> &ArtifactKey {
        &self.key
    }

    pub fn asset_name(&self) -> &str {
        &self.asset_name
    }

    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    pub const fn download_url(&self) -> &SourceLocator {
        &self.download_url
    }

    pub fn fingerprint(&self) -> Fingerprint {
        Fingerprint::new(FingerprintAlgorithm::Sha256, self.sha256.clone())
            .expect("ReleaseArtifact validates its checksum")
    }
}

impl From<ReleaseArtifact> for ReleaseArtifactWire {
    fn from(value: ReleaseArtifact) -> Self {
        Self {
            version: value.version,
            key: value.key,
            asset_name: value.asset_name,
            sha256: value.sha256,
            download_url: value.download_url,
        }
    }
}

impl TryFrom<ReleaseArtifactWire> for ReleaseArtifact {
    type Error = ValidationError;

    fn try_from(value: ReleaseArtifactWire) -> Result<Self, Self::Error> {
        Self::new(
            value.version,
            value.key,
            value.asset_name,
            value.sha256,
            value.download_url,
        )
    }
}

impl<'de> Deserialize<'de> for ReleaseArtifact {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        ReleaseArtifactWire::deserialize(deserializer)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BinaryDecision {
    Install,
    Update,
    Noop,
    MajorUpgradeBlocked,
}

pub fn choose_binary_decision(
    installed: Option<&ReleaseVersion>,
    available: &ReleaseVersion,
    allow_major: bool,
) -> BinaryDecision {
    let Some(installed) = installed else {
        return BinaryDecision::Install;
    };
    if installed == available {
        return BinaryDecision::Noop;
    }
    if available.major > installed.major && !allow_major {
        return BinaryDecision::MajorUpgradeBlocked;
    }
    if available > installed {
        BinaryDecision::Update
    } else {
        BinaryDecision::Noop
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BootstrapUpdateMode {
    Off,
    Check,
    ApplySafe,
}

impl Default for BootstrapUpdateMode {
    fn default() -> Self {
        Self::ApplySafe
    }
}

impl From<UpdateMode> for BootstrapUpdateMode {
    fn from(value: UpdateMode) -> Self {
        match value {
            UpdateMode::Off => Self::Off,
            UpdateMode::Check => Self::Check,
            UpdateMode::ApplySafe => Self::ApplySafe,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BootstrapPolicy {
    #[serde(default)]
    pub mode: BootstrapUpdateMode,
    #[serde(default)]
    pub allow_major: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::ConfigDocument;

    fn version(value: &str) -> ReleaseVersion {
        value.parse().unwrap()
    }

    #[test]
    fn release_versions_are_numeric_and_canonical() {
        assert_eq!(version("3.10.2").major(), 3);
        assert!("3.01.2".parse::<ReleaseVersion>().is_err());
        assert!("3.2".parse::<ReleaseVersion>().is_err());
        assert!("3.2.x".parse::<ReleaseVersion>().is_err());
    }

    #[test]
    fn binary_decisions_are_major_safe_and_idempotent() {
        let available = version("3.2.0");
        assert_eq!(
            choose_binary_decision(None, &available, false),
            BinaryDecision::Install
        );
        assert_eq!(
            choose_binary_decision(Some(&version("3.1.0")), &available, false),
            BinaryDecision::Update
        );
        assert_eq!(
            choose_binary_decision(Some(&available), &available, false),
            BinaryDecision::Noop
        );
        assert_eq!(
            choose_binary_decision(Some(&version("2.9.0")), &available, false),
            BinaryDecision::MajorUpgradeBlocked
        );
        assert_eq!(
            choose_binary_decision(Some(&version("2.9.0")), &available, true),
            BinaryDecision::Update
        );
        assert_eq!(
            choose_binary_decision(Some(&version("4.0.0")), &available, true),
            BinaryDecision::Noop
        );
    }

    #[test]
    fn artifact_values_reject_unsafe_locators_and_checksums() {
        let key = ArtifactKey {
            platform: SupportedPlatform::Linux,
            arch: ArtifactArch::X86_64,
        };
        let source = SourceLocator::new(
            "https://github.com/nklisch/skilltap/releases/download/v3.0.0/skilltap",
        )
        .unwrap();
        assert!(
            ReleaseArtifact::new(
                version("3.0.0"),
                key,
                "skilltap",
                "a".repeat(64),
                source.clone()
            )
            .is_ok()
        );
        assert!(
            ReleaseArtifact::new(
                version("3.0.0"),
                key,
                "../skilltap",
                "a".repeat(64),
                source.clone()
            )
            .is_err()
        );
        assert!(
            ReleaseArtifact::new(
                version("3.0.0"),
                key,
                "skilltap",
                "a".repeat(63),
                source.clone()
            )
            .is_err()
        );
        assert!(
            ReleaseArtifact::new(
                version("3.0.0"),
                key,
                "skilltap",
                "a".repeat(64),
                SourceLocator::new("--bad").unwrap()
            )
            .is_err()
        );

        for asset_name in [".", "..", "skilltap/path", "skilltap\\path", "skilltap\n"] {
            assert!(
                ReleaseArtifact::new(
                    version("3.0.0"),
                    key,
                    asset_name,
                    "a".repeat(64),
                    source.clone()
                )
                .is_err(),
                "unsafe asset name accepted: {asset_name:?}"
            );
        }
    }

    #[test]
    fn release_artifact_deserialization_revalidates_wire_values() {
        let key = ArtifactKey {
            platform: SupportedPlatform::Linux,
            arch: ArtifactArch::X86_64,
        };
        let source = SourceLocator::new(
            "https://github.com/nklisch/skilltap/releases/download/v3.0.0/skilltap",
        )
        .unwrap();
        let artifact =
            ReleaseArtifact::new(version("3.0.0"), key, "skilltap", "A".repeat(64), source)
                .unwrap();
        let encoded = serde_json::to_value(&artifact).unwrap();
        let decoded: ReleaseArtifact = serde_json::from_value(encoded).unwrap();
        assert_eq!(decoded.sha256(), "a".repeat(64));
        assert_eq!(decoded.fingerprint().digest(), "a".repeat(64));

        for (field, value) in [
            ("asset_name", serde_json::Value::String("..".into())),
            ("asset_name", serde_json::Value::String("skilltap\n".into())),
            ("sha256", serde_json::Value::String("not-a-checksum".into())),
        ] {
            let mut invalid = serde_json::to_value(&artifact).unwrap();
            invalid[field] = value;
            assert!(
                serde_json::from_value::<ReleaseArtifact>(invalid).is_err(),
                "invalid wire field accepted: {field}"
            );
        }
    }

    #[test]
    fn bootstrap_policy_is_optional_in_legacy_config_and_round_trips_when_set() {
        let config = ConfigDocument::defaults();
        let encoded = toml::to_string_pretty(&config).unwrap();
        assert!(!encoded.contains("[bootstrap]"));
        let custom = config.with_bootstrap_policy(crate::storage::BinaryUpdatePolicy {
            mode: BootstrapUpdateMode::Check,
            allow_major: true,
        });
        let encoded = toml::to_string_pretty(&custom).unwrap();
        assert!(encoded.contains("[bootstrap]"));
        let decoded: ConfigDocument = toml::from_str(&encoded).unwrap();
        assert_eq!(decoded.bootstrap(), custom.bootstrap());
    }
}
