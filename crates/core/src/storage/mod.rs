//! Strict versioned schemas for skilltap-owned machine state.

mod config;
mod inventory;
mod managed_artifact;
mod managed_record;
mod repository;
mod state;

pub use config::{
    ClaudeInstructionMode, ConfigDocument, HarnessBinary, HarnessPolicies, HarnessPolicy,
    InstructionPolicy, UpdateInterval, UpdateIntervalUnit, UpdateMode, UpdatePolicy,
};
pub use inventory::InventoryDocument;
pub use managed_artifact::{
    ArtifactPublication, ArtifactTree, ArtifactTreeError, FileManagedArtifactRepository,
    LoadedArtifact, ManagedArtifactAction, ManagedArtifactError, ManagedArtifactFailure,
    ManagedArtifactHandle, ManagedArtifactRepository, ManagedArtifactResidual,
    ManagedRemovalResidual,
};
pub use managed_record::{ArtifactRole, ManagedArtifactRecord};
pub use repository::{
    ConfigRepository, DocumentAction, DocumentKind, DocumentState, FileConfigRepository,
    FileInventoryRepository, FileStateRepository, InventoryRepository, StateRepository,
    StorageError, StorageFailure,
};
pub use state::{ApplyRecord, HarnessState, ResourceState, StateDocument, Timestamp};

use std::fmt;

use crate::domain::{AbsolutePath, OperationId, ResourceGraphError, ResourceKey, ValidationError};

pub const CONFIG_SCHEMA_VERSION: u32 = 1;
pub const INVENTORY_SCHEMA_VERSION: u32 = 1;
pub const STATE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug)]
pub enum SchemaError {
    UnsupportedVersion {
        document: &'static str,
        version: u32,
    },
    InvalidInterval,
    InvalidHarnessBinary,
    NonUtf8HarnessBinary,
    TimestampBeforeEpoch,
    InvalidNanoseconds {
        nanoseconds: u32,
    },
    TimestampOutOfRange,
    DuplicateProject {
        path: AbsolutePath,
    },
    UndeclaredProject {
        resource: ResourceKey,
        path: AbsolutePath,
    },
    ResourceGraph(ResourceGraphError),
    DuplicateHarness {
        harness: crate::domain::HarnessId,
    },
    DuplicateStateResource {
        resource: ResourceKey,
    },
    StateResourceNotFound {
        resource: ResourceKey,
    },
    DuplicateOperation {
        operation: OperationId,
    },
    ManagedOwnerMismatch {
        resource: ResourceKey,
        owner: ResourceKey,
    },
    InvalidManagedArtifactRecord {
        owner: ResourceKey,
    },
    InvalidOwnership {
        resource: ResourceKey,
    },
    InvalidArtifactRole {
        resource: ResourceKey,
    },
    DuplicateManagedPath {
        path: crate::domain::RelativeArtifactPath,
    },
    Validation(ValidationError),
}

impl fmt::Display for SchemaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedVersion { document, version } => {
                write!(formatter, "unsupported {document} schema version {version}")
            }
            Self::InvalidInterval => formatter.write_str(
                "update interval must be a canonical positive integer followed by `s`, `m`, `h`, or `d`",
            ),
            Self::InvalidHarnessBinary => formatter.write_str(
                "harness binary must be one PATH executable name or a normalized absolute path",
            ),
            Self::NonUtf8HarnessBinary => {
                formatter.write_str("harness binary is not valid UTF-8")
            }
            Self::TimestampBeforeEpoch => {
                formatter.write_str("timestamp must not precede the Unix epoch")
            }
            Self::InvalidNanoseconds { nanoseconds } => write!(
                formatter,
                "timestamp nanoseconds must be less than 1000000000, got {nanoseconds}"
            ),
            Self::TimestampOutOfRange => {
                formatter.write_str("timestamp is outside the platform SystemTime range")
            }
            Self::DuplicateProject { path } => write!(formatter, "duplicate project root `{path}`"),
            Self::UndeclaredProject { resource, path } => write!(
                formatter,
                "resource `{resource}` uses undeclared project root `{path}`"
            ),
            Self::ResourceGraph(source) => source.fmt(formatter),
            Self::DuplicateHarness { harness } => {
                write!(formatter, "duplicate harness state `{harness}`")
            }
            Self::DuplicateStateResource { resource } => {
                write!(formatter, "duplicate state resource `{resource}`")
            }
            Self::StateResourceNotFound { resource } => {
                write!(formatter, "state resource `{resource}` was not found")
            }
            Self::DuplicateOperation { operation } => {
                write!(formatter, "duplicate apply operation `{operation}`")
            }
            Self::ManagedOwnerMismatch { resource, owner } => write!(
                formatter,
                "managed artifact owner `{owner:?}` does not match resource `{resource:?}`"
            ),
            Self::InvalidManagedArtifactRecord { owner } => {
                write!(formatter, "managed artifact record for `{owner:?}` is invalid")
            }
            Self::InvalidOwnership { resource } => write!(
                formatter,
                "resource `{resource}` has inconsistent provenance and ownership"
            ),
            Self::InvalidArtifactRole { resource } => write!(
                formatter,
                "resource `{resource}` has an artifact role inconsistent with its provenance"
            ),
            Self::DuplicateManagedPath { path } => {
                write!(formatter, "duplicate managed artifact path `{path}`")
            }
            Self::Validation(source) => source.fmt(formatter),
        }
    }
}

impl std::error::Error for SchemaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ResourceGraph(source) => Some(source),
            Self::Validation(source) => Some(source),
            _ => None,
        }
    }
}

impl From<ResourceGraphError> for SchemaError {
    fn from(value: ResourceGraphError) -> Self {
        Self::ResourceGraph(value)
    }
}

impl From<ValidationError> for SchemaError {
    fn from(value: ValidationError) -> Self {
        Self::Validation(value)
    }
}

#[cfg(test)]
mod tests;
