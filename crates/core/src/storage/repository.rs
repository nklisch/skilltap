use std::{fmt, marker::PhantomData, path::PathBuf};

use serde::{Serialize, de::DeserializeOwned};

mod codec;

use codec::{CodecFailure, DocumentCodec, JsonCodec, TomlCodec};

use super::{
    CONFIG_SCHEMA_VERSION, ConfigDocument, INVENTORY_SCHEMA_VERSION, InventoryDocument,
    STATE_SCHEMA_VERSION, StateDocument,
};
use crate::{
    domain::AbsolutePath,
    runtime::{FileSystem, RuntimeError},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocumentKind {
    Config,
    Inventory,
    State,
}

impl DocumentKind {
    const fn file_name(self) -> &'static str {
        match self {
            Self::Config => "config.toml",
            Self::Inventory => "inventory.toml",
            Self::State => "state.json",
        }
    }
}

impl fmt::Display for DocumentKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Config => "config",
            Self::Inventory => "inventory",
            Self::State => "state",
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocumentAction {
    Read,
    Decode,
    Validate,
    Encode,
    Write,
}

impl fmt::Display for DocumentAction {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Read => "read",
            Self::Decode => "decode",
            Self::Validate => "validate",
            Self::Encode => "encode",
            Self::Write => "write",
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageFailure {
    Runtime,
    Malformed,
    Invalid,
    UnsupportedSchema { version: u32 },
}

enum StorageCause {
    Runtime,
    Malformed,
    Invalid,
    UnsupportedSchema { version: u32 },
}

pub struct StorageError {
    document: DocumentKind,
    action: DocumentAction,
    path: AbsolutePath,
    cause: StorageCause,
}

impl StorageError {
    pub const fn document(&self) -> DocumentKind {
        self.document
    }

    pub const fn action(&self) -> DocumentAction {
        self.action
    }

    pub const fn path(&self) -> &AbsolutePath {
        &self.path
    }

    pub const fn failure(&self) -> StorageFailure {
        match self.cause {
            StorageCause::Runtime => StorageFailure::Runtime,
            StorageCause::Malformed => StorageFailure::Malformed,
            StorageCause::Invalid => StorageFailure::Invalid,
            StorageCause::UnsupportedSchema { version } => {
                StorageFailure::UnsupportedSchema { version }
            }
        }
    }

    fn runtime(
        document: DocumentKind,
        action: DocumentAction,
        path: &AbsolutePath,
        _source: RuntimeError,
    ) -> Self {
        Self {
            document,
            action,
            path: path.clone(),
            cause: StorageCause::Runtime,
        }
    }

    fn codec(document: DocumentKind, path: &AbsolutePath, failure: CodecFailure) -> Self {
        let (action, cause) = match failure {
            CodecFailure::Malformed => (DocumentAction::Decode, StorageCause::Malformed),
            CodecFailure::Invalid => (DocumentAction::Validate, StorageCause::Invalid),
            CodecFailure::UnsupportedSchema { version } => (
                DocumentAction::Validate,
                StorageCause::UnsupportedSchema { version },
            ),
            CodecFailure::Encode => (DocumentAction::Encode, StorageCause::Invalid),
        };
        Self {
            document,
            action,
            path: path.clone(),
            cause,
        }
    }
}

impl fmt::Debug for StorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StorageError")
            .field("document", &self.document)
            .field("action", &self.action)
            .field("path", &self.path)
            .field("failure", &self.failure())
            .finish()
    }
}

impl fmt::Display for StorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} {} document `{}` failed: ",
            self.action, self.document, self.path
        )?;
        match self.failure() {
            StorageFailure::Runtime => formatter.write_str("runtime filesystem error"),
            StorageFailure::Malformed => formatter.write_str("malformed document"),
            StorageFailure::Invalid => formatter.write_str("invalid document"),
            StorageFailure::UnsupportedSchema { version } => {
                write!(formatter, "unsupported schema version {version}")
            }
        }
    }
}

impl std::error::Error for StorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DocumentState<T> {
    Missing,
    Present(T),
}

pub trait ConfigRepository {
    fn load(&self) -> Result<DocumentState<ConfigDocument>, StorageError>;
    fn replace(&self, value: &ConfigDocument) -> Result<(), StorageError>;
}

pub trait InventoryRepository {
    fn load(&self) -> Result<DocumentState<InventoryDocument>, StorageError>;
    fn replace(&self, value: &InventoryDocument) -> Result<(), StorageError>;
}

pub trait StateRepository {
    fn load(&self) -> Result<DocumentState<StateDocument>, StorageError>;
    fn replace(&self, value: &StateDocument) -> Result<(), StorageError>;
}

pub struct FileConfigRepository<'a>(DocumentEngine<'a, ConfigDocument, TomlCodec>);
pub struct FileInventoryRepository<'a>(DocumentEngine<'a, InventoryDocument, TomlCodec>);
pub struct FileStateRepository<'a>(DocumentEngine<'a, StateDocument, JsonCodec>);

macro_rules! repository_adapter {
    ($repository:ident, $port:ident, $document:ty, $kind:expr, $codec:expr) => {
        impl<'a> $repository<'a> {
            pub fn new(
                filesystem: &'a dyn FileSystem,
                config_root: AbsolutePath,
            ) -> Result<Self, StorageError> {
                DocumentEngine::new(filesystem, config_root, $kind, $codec).map(Self)
            }
        }

        impl $port for $repository<'_> {
            fn load(&self) -> Result<DocumentState<$document>, StorageError> {
                self.0.load()
            }

            fn replace(&self, value: &$document) -> Result<(), StorageError> {
                self.0.replace(value)
            }
        }
    };
}

repository_adapter!(
    FileConfigRepository,
    ConfigRepository,
    ConfigDocument,
    DocumentKind::Config,
    TomlCodec::new(CONFIG_SCHEMA_VERSION)
);
repository_adapter!(
    FileInventoryRepository,
    InventoryRepository,
    InventoryDocument,
    DocumentKind::Inventory,
    TomlCodec::new(INVENTORY_SCHEMA_VERSION)
);
repository_adapter!(
    FileStateRepository,
    StateRepository,
    StateDocument,
    DocumentKind::State,
    JsonCodec::new(STATE_SCHEMA_VERSION)
);

struct DocumentEngine<'a, T, C> {
    filesystem: &'a dyn FileSystem,
    config_root: AbsolutePath,
    path: AbsolutePath,
    kind: DocumentKind,
    codec: C,
    marker: PhantomData<T>,
}

impl<'a, T, C> DocumentEngine<'a, T, C>
where
    T: DeserializeOwned + Serialize,
    C: DocumentCodec<T>,
{
    fn new(
        filesystem: &'a dyn FileSystem,
        config_root: AbsolutePath,
        kind: DocumentKind,
        codec: C,
    ) -> Result<Self, StorageError> {
        let path = PathBuf::from(config_root.as_str()).join(kind.file_name());
        let path = path
            .to_str()
            .and_then(|value| AbsolutePath::new(value).ok())
            .ok_or_else(|| StorageError {
                document: kind,
                action: DocumentAction::Validate,
                path: config_root.clone(),
                cause: StorageCause::Invalid,
            })?;
        Ok(Self {
            filesystem,
            config_root,
            path,
            kind,
            codec,
            marker: PhantomData,
        })
    }

    fn load(&self) -> Result<DocumentState<T>, StorageError> {
        let Some(contents) =
            self.filesystem
                .read_regular_no_follow(&self.path)
                .map_err(|source| {
                    StorageError::runtime(self.kind, DocumentAction::Read, &self.path, source)
                })?
        else {
            return Ok(DocumentState::Missing);
        };
        self.codec
            .decode(&contents)
            .map(DocumentState::Present)
            .map_err(|failure| StorageError::codec(self.kind, &self.path, failure))
    }

    fn replace(&self, value: &T) -> Result<(), StorageError> {
        let contents = self
            .codec
            .encode(value)
            .map_err(|failure| StorageError::codec(self.kind, &self.path, failure))?;
        self.codec
            .decode(&contents)
            .map_err(|failure| StorageError::codec(self.kind, &self.path, failure))?;
        self.filesystem
            .create_directory_all(&self.config_root)
            .map_err(|source| {
                StorageError::runtime(self.kind, DocumentAction::Write, &self.path, source)
            })?;
        self.filesystem
            .atomic_write(&self.path, &contents)
            .map_err(|source| {
                StorageError::runtime(self.kind, DocumentAction::Write, &self.path, source)
            })
    }
}

#[cfg(test)]
mod tests;
