use std::{collections::BTreeSet, fmt, marker::PhantomData, path::PathBuf};

use serde::{
    Deserialize, Deserializer, Serialize,
    de::{DeserializeOwned, IgnoredAny, MapAccess, Visitor},
};

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

enum CodecFailure {
    Malformed,
    Invalid,
    UnsupportedSchema { version: u32 },
    Encode,
}

trait DocumentCodec<T> {
    fn decode(&self, contents: &[u8]) -> Result<T, CodecFailure>;
    fn encode(&self, value: &T) -> Result<Vec<u8>, CodecFailure>;
}

#[derive(Clone, Copy)]
struct TomlCodec {
    expected_schema: u32,
}

impl TomlCodec {
    const fn new(expected_schema: u32) -> Self {
        Self { expected_schema }
    }
}

impl<T> DocumentCodec<T> for TomlCodec
where
    T: DeserializeOwned + Serialize,
{
    fn decode(&self, contents: &[u8]) -> Result<T, CodecFailure> {
        let contents = std::str::from_utf8(contents).map_err(|_| CodecFailure::Malformed)?;
        let table = toml::from_str::<toml::Table>(contents).map_err(|_| CodecFailure::Malformed)?;
        validate_toml_schema(&table, self.expected_schema)?;
        toml::from_str(contents).map_err(|_| CodecFailure::Invalid)
    }

    fn encode(&self, value: &T) -> Result<Vec<u8>, CodecFailure> {
        toml::to_string_pretty(value)
            .map(String::into_bytes)
            .map_err(|_| CodecFailure::Encode)
    }
}

#[derive(Clone, Copy)]
struct JsonCodec {
    expected_schema: u32,
}

impl JsonCodec {
    const fn new(expected_schema: u32) -> Self {
        Self { expected_schema }
    }
}

impl<T> DocumentCodec<T> for JsonCodec
where
    T: DeserializeOwned + Serialize,
{
    fn decode(&self, contents: &[u8]) -> Result<T, CodecFailure> {
        serde_json::from_slice::<serde_json::Value>(contents)
            .map_err(|_| CodecFailure::Malformed)?;
        let probe = serde_json::from_slice::<JsonSchemaProbe>(contents)
            .map_err(|_| CodecFailure::Invalid)?;
        if let Some(version) = probe.schema
            && version != self.expected_schema
        {
            return Err(CodecFailure::UnsupportedSchema { version });
        }
        serde_json::from_slice(contents).map_err(|_| CodecFailure::Invalid)
    }

    fn encode(&self, value: &T) -> Result<Vec<u8>, CodecFailure> {
        let mut bytes = serde_json::to_vec_pretty(value).map_err(|_| CodecFailure::Encode)?;
        bytes.push(b'\n');
        Ok(bytes)
    }
}

fn validate_toml_schema(table: &toml::Table, expected_schema: u32) -> Result<(), CodecFailure> {
    if let Some(version) = table.get("schema").and_then(toml::Value::as_integer)
        && version >= 0
        && version as u64 <= u32::MAX as u64
        && version as u32 != expected_schema
    {
        return Err(CodecFailure::UnsupportedSchema {
            version: version as u32,
        });
    }
    Ok(())
}

struct JsonSchemaProbe {
    schema: Option<u32>,
}

impl<'de> Deserialize<'de> for JsonSchemaProbe {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ProbeVisitor;

        impl<'de> Visitor<'de> for ProbeVisitor {
            type Value = JsonSchemaProbe;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a JSON object with unique top-level fields")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut seen = BTreeSet::new();
                let mut schema = None;
                while let Some(key) = map.next_key::<String>()? {
                    if !seen.insert(key.clone()) {
                        return Err(serde::de::Error::custom("duplicate top-level field"));
                    }
                    if key == "schema" {
                        schema = Some(map.next_value::<u32>()?);
                    } else {
                        map.next_value::<IgnoredAny>()?;
                    }
                }
                Ok(JsonSchemaProbe { schema })
            }
        }

        deserializer.deserialize_map(ProbeVisitor)
    }
}

#[cfg(test)]
mod tests;
