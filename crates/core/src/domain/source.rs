use std::{fmt, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{NativeId, ValidationError, validate_text};

macro_rules! opaque_text_type {
    ($name:ident, $kind:literal, $max:expr) => {
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
                let value = value.into();
                validate_text(&value, $kind, $max)?;
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

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

opaque_text_type!(SourceLocator, "source locator", 4096);
opaque_text_type!(RequestedRevision, "requested revision", 512);

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Git,
    Local,
    Native,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Source {
    pub kind: SourceKind,
    pub locator: SourceLocator,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requested_revision: Option<RequestedRevision>,
}

impl Source {
    pub fn new(
        kind: SourceKind,
        locator: SourceLocator,
        requested_revision: Option<RequestedRevision>,
    ) -> Self {
        Self {
            kind,
            locator,
            requested_revision,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GitCommit(String);

impl GitCommit {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        if !matches!(value.len(), 40 | 64) {
            return Err(ValidationError::GitCommitLength {
                actual: value.len(),
            });
        }
        if !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(ValidationError::NonHexadecimal { kind: "Git commit" });
        }
        Ok(Self(value.to_ascii_lowercase()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for GitCommit {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Serialize for GitCommit {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for GitCommit {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ResolvedRevision {
    GitCommit(GitCommit),
    Native(NativeId),
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FingerprintAlgorithm {
    Sha256,
    Sha512,
}

impl FingerprintAlgorithm {
    const fn digest_length(self) -> usize {
        match self {
            Self::Sha256 => 64,
            Self::Sha512 => 128,
        }
    }
}

impl fmt::Display for FingerprintAlgorithm {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Sha256 => "sha256",
            Self::Sha512 => "sha512",
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(try_from = "FingerprintWire", into = "FingerprintWire")]
pub struct Fingerprint {
    algorithm: FingerprintAlgorithm,
    digest: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct FingerprintWire {
    algorithm: FingerprintAlgorithm,
    digest: String,
}

impl Fingerprint {
    pub fn new(
        algorithm: FingerprintAlgorithm,
        digest: impl Into<String>,
    ) -> Result<Self, ValidationError> {
        let digest = digest.into();
        let expected = algorithm.digest_length();
        if digest.len() != expected {
            return Err(ValidationError::FingerprintDigestLength {
                algorithm,
                expected,
                actual: digest.len(),
            });
        }
        if !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(ValidationError::NonHexadecimal {
                kind: "fingerprint digest",
            });
        }
        Ok(Self {
            algorithm,
            digest: digest.to_ascii_lowercase(),
        })
    }

    pub const fn algorithm(&self) -> FingerprintAlgorithm {
        self.algorithm
    }

    pub fn digest(&self) -> &str {
        &self.digest
    }
}

impl From<Fingerprint> for FingerprintWire {
    fn from(value: Fingerprint) -> Self {
        Self {
            algorithm: value.algorithm,
            digest: value.digest,
        }
    }
}

impl TryFrom<FingerprintWire> for Fingerprint {
    type Error = ValidationError;

    fn try_from(value: FingerprintWire) -> Result<Self, Self::Error> {
        Self::new(value.algorithm, value.digest)
    }
}

impl fmt::Display for Fingerprint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:{}", self.algorithm, self.digest)
    }
}

impl FromStr for Fingerprint {
    type Err = ValidationError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (algorithm, digest) = value
            .split_once(':')
            .ok_or(ValidationError::InvalidFormat {
                kind: "fingerprint",
                expected: "use `<algorithm>:<hex-digest>` format",
            })?;
        let algorithm = match algorithm {
            "sha256" => FingerprintAlgorithm::Sha256,
            "sha512" => FingerprintAlgorithm::Sha512,
            _ => {
                return Err(ValidationError::InvalidFormat {
                    kind: "fingerprint algorithm",
                    expected: "be `sha256` or `sha512`",
                });
            }
        };
        Self::new(algorithm, digest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_and_revision_forms_are_stable_and_round_trip() {
        let source = Source::new(
            SourceKind::Git,
            SourceLocator::new("https://github.com/nklisch/skilltap.git").unwrap(),
            Some(RequestedRevision::new("main").unwrap()),
        );
        let json = serde_json::to_string(&source).unwrap();
        assert_eq!(
            json,
            r#"{"kind":"git","locator":"https://github.com/nklisch/skilltap.git","requested_revision":"main"}"#
        );
        assert_eq!(serde_json::from_str::<Source>(&json).unwrap(), source);

        let revision = ResolvedRevision::GitCommit(
            GitCommit::new("A".repeat(40)).expect("valid SHA-1 object id"),
        );
        let json = serde_json::to_string(&revision).unwrap();
        assert_eq!(
            json,
            r#"{"kind":"git_commit","value":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}"#
        );
        assert_eq!(
            serde_json::from_str::<ResolvedRevision>(&json).unwrap(),
            revision
        );
    }

    #[test]
    fn git_commits_accept_sha1_and_sha256_only() {
        assert_eq!(GitCommit::new("a".repeat(40)).unwrap().as_str().len(), 40);
        assert_eq!(
            GitCommit::new("B".repeat(64)).unwrap().as_str(),
            "b".repeat(64)
        );
        assert!(matches!(
            GitCommit::new("a".repeat(39)),
            Err(ValidationError::GitCommitLength { actual: 39 })
        ));
        let non_hex = format!("{}z", "a".repeat(39));
        let expected = GitCommit::new(non_hex.clone()).unwrap_err();
        assert_eq!(
            expected,
            ValidationError::NonHexadecimal { kind: "Git commit" }
        );
        assert!(
            serde_json::from_str::<GitCommit>(&format!(r#""{non_hex}""#))
                .unwrap_err()
                .to_string()
                .contains(&expected.to_string())
        );
    }

    #[test]
    fn fingerprints_validate_parse_normalize_and_round_trip() {
        let fingerprint = Fingerprint::new(FingerprintAlgorithm::Sha256, "A".repeat(64)).unwrap();
        assert_eq!(
            fingerprint.to_string(),
            format!("sha256:{}", "a".repeat(64))
        );
        assert_eq!(
            fingerprint.to_string().parse::<Fingerprint>().unwrap(),
            fingerprint
        );
        let json = serde_json::to_string(&fingerprint).unwrap();
        assert_eq!(
            json,
            format!(r#"{{"algorithm":"sha256","digest":"{}"}}"#, "a".repeat(64))
        );
        assert_eq!(
            serde_json::from_str::<Fingerprint>(&json).unwrap(),
            fingerprint
        );

        assert!(matches!(
            Fingerprint::new(FingerprintAlgorithm::Sha256, "a".repeat(63)),
            Err(ValidationError::FingerprintDigestLength { .. })
        ));
        let invalid_json = format!(r#"{{"algorithm":"sha256","digest":"{}z"}}"#, "a".repeat(63));
        assert!(
            serde_json::from_str::<Fingerprint>(&invalid_json)
                .unwrap_err()
                .to_string()
                .contains("fingerprint digest must contain only hexadecimal characters")
        );
        assert!("md5:abcd".parse::<Fingerprint>().is_err());
    }
}
