pub mod identity;
pub mod scope;
pub mod source;

pub use identity::{HarnessId, NativeId, OperationId, ResourceId};
pub use scope::{
    AbsolutePath, HarnessSet, RelativeArtifactPath, Scope, ScopeSelection, TargetSelection,
};
pub use source::{
    Fingerprint, FingerprintAlgorithm, GitCommit, RequestedRevision, ResolvedRevision, Source,
    SourceKind, SourceLocator,
};

use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValidationError {
    Empty {
        kind: &'static str,
    },
    SurroundingWhitespace {
        kind: &'static str,
    },
    ControlCharacter {
        kind: &'static str,
        index: usize,
    },
    TooLong {
        kind: &'static str,
        max: usize,
        actual: usize,
    },
    InvalidFormat {
        kind: &'static str,
        expected: &'static str,
    },
    PathNotAbsolute,
    InvalidAbsolutePathComponent,
    PathNotRelative,
    InvalidRelativePathComponent,
    EmptyHarnessSet,
    HarnessNotEnabled {
        harness: HarnessId,
    },
    GitCommitLength {
        actual: usize,
    },
    NonHexadecimal {
        kind: &'static str,
    },
    FingerprintDigestLength {
        algorithm: FingerprintAlgorithm,
        expected: usize,
        actual: usize,
    },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty { kind } => write!(formatter, "{kind} must not be empty"),
            Self::SurroundingWhitespace { kind } => {
                write!(formatter, "{kind} must not contain surrounding whitespace")
            }
            Self::ControlCharacter { kind, index } => {
                write!(
                    formatter,
                    "{kind} contains a control character at byte {index}"
                )
            }
            Self::TooLong { kind, max, actual } => {
                write!(
                    formatter,
                    "{kind} must be at most {max} bytes, got {actual}"
                )
            }
            Self::InvalidFormat { kind, expected } => {
                write!(formatter, "{kind} must {expected}")
            }
            Self::PathNotAbsolute => write!(formatter, "absolute path must be absolute"),
            Self::InvalidAbsolutePathComponent => {
                write!(formatter, "absolute path must be lexically normalized")
            }
            Self::PathNotRelative => write!(formatter, "artifact path must be relative"),
            Self::InvalidRelativePathComponent => write!(
                formatter,
                "artifact path must contain only normal relative components"
            ),
            Self::EmptyHarnessSet => write!(formatter, "harness set must not be empty"),
            Self::HarnessNotEnabled { harness } => {
                write!(formatter, "target harness `{harness}` is not enabled")
            }
            Self::GitCommitLength { actual } => write!(
                formatter,
                "Git commit must contain 40 or 64 hexadecimal characters, got {actual}"
            ),
            Self::NonHexadecimal { kind } => {
                write!(formatter, "{kind} must contain only hexadecimal characters")
            }
            Self::FingerprintDigestLength {
                algorithm,
                expected,
                actual,
            } => write!(
                formatter,
                "{algorithm} fingerprint digest must contain {expected} hexadecimal characters, got {actual}"
            ),
        }
    }
}

impl std::error::Error for ValidationError {}

fn validate_text(value: &str, kind: &'static str, max: usize) -> Result<(), ValidationError> {
    if value.is_empty() {
        return Err(ValidationError::Empty { kind });
    }
    if value.trim() != value {
        return Err(ValidationError::SurroundingWhitespace { kind });
    }
    if let Some((index, _)) = value
        .char_indices()
        .find(|(_, character)| character.is_control())
    {
        return Err(ValidationError::ControlCharacter { kind, index });
    }
    if value.len() > max {
        return Err(ValidationError::TooLong {
            kind,
            max,
            actual: value.len(),
        });
    }
    Ok(())
}

fn validate_identifier(value: &str, kind: &'static str, max: usize) -> Result<(), ValidationError> {
    validate_text(value, kind, max)?;
    let mut characters = value.chars();
    let starts_correctly = characters
        .next()
        .is_some_and(|character| character.is_ascii_lowercase() || character.is_ascii_digit());
    if !starts_correctly
        || !characters.all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || matches!(character, '-' | '_' | '.' | ':')
        })
    {
        return Err(ValidationError::InvalidFormat {
            kind,
            expected: "start with a lowercase ASCII letter or digit and contain only lowercase ASCII letters, digits, `.`, `-`, `_`, or `:`",
        });
    }
    Ok(())
}
