//! Pure contracts for bounded, read-only native observation.

use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::{OsStr, OsString},
    fmt,
    num::NonZeroU32,
    time::Duration,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::domain::{AbsolutePath, ConfiguredBinary, ExecutableIdentity, RelativeArtifactPath};

pub const MAX_PROCESS_DEADLINE_MILLISECONDS: u64 = 300_000;
pub const MAX_PROCESS_STREAM_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_PROCESS_COMBINED_BYTES: u64 = 128 * 1024 * 1024;
pub const MAX_JSON_BYTES: u64 = 64 * 1024 * 1024;
/// Highest container nesting accepted below serde_json's built-in recursion guard.
pub const MAX_JSON_DEPTH: u32 = 127;
pub const MAX_TREE_DEPTH: u32 = 64;
pub const MAX_TREE_ENTRIES: u64 = 100_000;
pub const MAX_TREE_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_TREE_TOTAL_BYTES: u64 = 1024 * 1024 * 1024;
pub const MAX_SYMLINK_TARGET_BYTES: u64 = 64 * 1024;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationLimitKind {
    ProcessDeadlineMilliseconds,
    StandardOutputBytes,
    StandardErrorBytes,
    CombinedOutputBytes,
    JsonBytes,
    JsonDepth,
    TreeDepth,
    TreeEntries,
    FileBytes,
    TotalTreeBytes,
    SymlinkTargetBytes,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LimitRelationship {
    CombinedOutputCoversStandardOutput,
    CombinedOutputCoversStandardError,
    TreeEntriesCoverDepth,
    TotalTreeCoversFile,
    TotalTreeCoversSymlinkTarget,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputStream {
    StandardOutput,
    StandardError,
    Combined,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationRuntimeError {
    ZeroLimit { limit: ObservationLimitKind },
    LimitExceedsMaximum { limit: ObservationLimitKind },
    InvalidLimitRelationship { relationship: LimitRelationship },
    InvalidSearchPath,
    ExecutableNotFound,
    ExecutableNotRegular,
    ExecutableNotRunnable,
    ExecutableInaccessible,
    ExecutableResolutionFailed,
    ExecutableChanged,
    ProcessSpawnFailed,
    ProcessIoFailed,
    ProcessWaitFailed,
    ProcessTerminationFailed,
    ProcessDrainFailed,
    ProcessDeadlineExceeded,
    ProcessOutputLimitExceeded { stream: OutputStream },
    JsonByteLimitExceeded,
    JsonInvalidUtf8,
    JsonInvalidSyntax,
    JsonDuplicateKey,
    JsonTrailingContent,
    JsonDepthLimitExceeded,
    TreeRootUnavailable,
    TreeEntryUnreadable,
    TreeEntryUnsupported,
    TreeEntryNonUtf8,
    TreeEntryChanged,
    TreeDepthLimitExceeded,
    TreeEntryLimitExceeded,
    TreeFileLimitExceeded,
    TreeTotalLimitExceeded,
    TreeSymlinkTargetLimitExceeded,
    DuplicateTreeEntry,
}

impl fmt::Display for ObservationRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ZeroLimit { .. } => "an observation limit must be non-zero",
            Self::LimitExceedsMaximum { .. } => "an observation limit exceeds its hard maximum",
            Self::InvalidLimitRelationship { .. } => {
                "observation limits have an invalid relationship"
            }
            Self::InvalidSearchPath => "the executable search path is invalid",
            Self::ExecutableNotFound => "the configured executable was not found",
            Self::ExecutableNotRegular => "the configured executable is not a regular file",
            Self::ExecutableNotRunnable => "the configured executable is not runnable",
            Self::ExecutableInaccessible => "the configured executable is inaccessible",
            Self::ExecutableResolutionFailed => "the configured executable could not be resolved",
            Self::ExecutableChanged => "the resolved executable changed before use",
            Self::ProcessSpawnFailed => "the native process could not be started",
            Self::ProcessIoFailed => "native process output could not be read",
            Self::ProcessWaitFailed => "the native process could not be reaped",
            Self::ProcessTerminationFailed => "the native process could not be terminated",
            Self::ProcessDrainFailed => {
                "native process output could not be drained after termination"
            }
            Self::ProcessDeadlineExceeded => "the native process exceeded its deadline",
            Self::ProcessOutputLimitExceeded { .. } => {
                "native process output exceeded a configured limit"
            }
            Self::JsonByteLimitExceeded => "the JSON input exceeded its byte limit",
            Self::JsonInvalidUtf8 => "the JSON input is not valid UTF-8",
            Self::JsonInvalidSyntax => "the JSON input is not valid JSON",
            Self::JsonDuplicateKey => "the JSON input contains a duplicate object key",
            Self::JsonTrailingContent => "the JSON input contains trailing content",
            Self::JsonDepthLimitExceeded => "the JSON input exceeded its depth limit",
            Self::TreeRootUnavailable => "the external tree root is unavailable",
            Self::TreeEntryUnreadable => "an external tree entry could not be read",
            Self::TreeEntryUnsupported => "an external tree entry has an unsupported type",
            Self::TreeEntryNonUtf8 => "an external tree entry name is not valid UTF-8",
            Self::TreeEntryChanged => "an external tree entry changed while it was observed",
            Self::TreeDepthLimitExceeded => "the external tree exceeded its depth limit",
            Self::TreeEntryLimitExceeded => "the external tree exceeded its entry limit",
            Self::TreeFileLimitExceeded => "an external tree file exceeded its byte limit",
            Self::TreeTotalLimitExceeded => "the external tree exceeded its total byte limit",
            Self::TreeSymlinkTargetLimitExceeded => {
                "an external tree link target exceeded its byte limit"
            }
            Self::DuplicateTreeEntry => "the external tree contains a duplicate entry",
        })
    }
}

impl std::error::Error for ObservationRuntimeError {}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct ProcessLimitsWire {
    deadline_milliseconds: u64,
    stdout_bytes: u64,
    stderr_bytes: u64,
    combined_output_bytes: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProcessLimits {
    deadline_milliseconds: u64,
    stdout_bytes: u64,
    stderr_bytes: u64,
    combined_output_bytes: u64,
}

impl ProcessLimits {
    pub fn new(
        deadline_milliseconds: u64,
        stdout_bytes: u64,
        stderr_bytes: u64,
        combined_output_bytes: u64,
    ) -> Result<Self, ObservationRuntimeError> {
        validate_limit(
            deadline_milliseconds,
            MAX_PROCESS_DEADLINE_MILLISECONDS,
            ObservationLimitKind::ProcessDeadlineMilliseconds,
        )?;
        validate_byte_limit(
            stdout_bytes,
            MAX_PROCESS_STREAM_BYTES,
            ObservationLimitKind::StandardOutputBytes,
        )?;
        validate_byte_limit(
            stderr_bytes,
            MAX_PROCESS_STREAM_BYTES,
            ObservationLimitKind::StandardErrorBytes,
        )?;
        validate_byte_limit(
            combined_output_bytes,
            MAX_PROCESS_COMBINED_BYTES,
            ObservationLimitKind::CombinedOutputBytes,
        )?;
        if combined_output_bytes < stdout_bytes {
            return Err(ObservationRuntimeError::InvalidLimitRelationship {
                relationship: LimitRelationship::CombinedOutputCoversStandardOutput,
            });
        }
        if combined_output_bytes < stderr_bytes {
            return Err(ObservationRuntimeError::InvalidLimitRelationship {
                relationship: LimitRelationship::CombinedOutputCoversStandardError,
            });
        }
        Ok(Self {
            deadline_milliseconds,
            stdout_bytes,
            stderr_bytes,
            combined_output_bytes,
        })
    }

    pub const fn deadline_milliseconds(self) -> u64 {
        self.deadline_milliseconds
    }
    pub const fn deadline(self) -> Duration {
        Duration::from_millis(self.deadline_milliseconds)
    }
    pub const fn stdout_bytes(self) -> u64 {
        self.stdout_bytes
    }
    pub const fn stderr_bytes(self) -> u64 {
        self.stderr_bytes
    }
    pub const fn combined_output_bytes(self) -> u64 {
        self.combined_output_bytes
    }
}

impl From<ProcessLimits> for ProcessLimitsWire {
    fn from(value: ProcessLimits) -> Self {
        Self {
            deadline_milliseconds: value.deadline_milliseconds,
            stdout_bytes: value.stdout_bytes,
            stderr_bytes: value.stderr_bytes,
            combined_output_bytes: value.combined_output_bytes,
        }
    }
}

impl TryFrom<ProcessLimitsWire> for ProcessLimits {
    type Error = ObservationRuntimeError;
    fn try_from(value: ProcessLimitsWire) -> Result<Self, Self::Error> {
        Self::new(
            value.deadline_milliseconds,
            value.stdout_bytes,
            value.stderr_bytes,
            value.combined_output_bytes,
        )
    }
}

impl Serialize for ProcessLimits {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        ProcessLimitsWire::from(*self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ProcessLimits {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        ProcessLimitsWire::deserialize(deserializer)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(into = "JsonLimitsWire")]
pub struct JsonLimits {
    bytes: u64,
    depth: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct JsonLimitsWire {
    bytes: u64,
    depth: u32,
}

impl JsonLimits {
    pub fn new(bytes: u64, depth: u32) -> Result<Self, ObservationRuntimeError> {
        validate_byte_limit(bytes, MAX_JSON_BYTES, ObservationLimitKind::JsonBytes)?;
        validate_limit(
            u64::from(depth),
            u64::from(MAX_JSON_DEPTH),
            ObservationLimitKind::JsonDepth,
        )?;
        Ok(Self { bytes, depth })
    }
    pub const fn bytes(self) -> u64 {
        self.bytes
    }
    pub const fn depth(self) -> u32 {
        self.depth
    }
}

impl From<JsonLimits> for JsonLimitsWire {
    fn from(value: JsonLimits) -> Self {
        Self {
            bytes: value.bytes,
            depth: value.depth,
        }
    }
}

impl TryFrom<JsonLimitsWire> for JsonLimits {
    type Error = ObservationRuntimeError;
    fn try_from(value: JsonLimitsWire) -> Result<Self, Self::Error> {
        Self::new(value.bytes, value.depth)
    }
}

impl<'de> Deserialize<'de> for JsonLimits {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        JsonLimitsWire::deserialize(deserializer)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(into = "ExternalTreeLimitsWire")]
pub struct ExternalTreeLimits {
    depth: u32,
    entries: u64,
    file_bytes: u64,
    total_bytes: u64,
    symlink_target_bytes: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ExternalTreeLimitsWire {
    depth: u32,
    entries: u64,
    file_bytes: u64,
    total_bytes: u64,
    symlink_target_bytes: u64,
}

impl ExternalTreeLimits {
    pub fn new(
        depth: u32,
        entries: u64,
        file_bytes: u64,
        total_bytes: u64,
        symlink_target_bytes: u64,
    ) -> Result<Self, ObservationRuntimeError> {
        validate_limit(
            u64::from(depth),
            u64::from(MAX_TREE_DEPTH),
            ObservationLimitKind::TreeDepth,
        )?;
        validate_limit(entries, MAX_TREE_ENTRIES, ObservationLimitKind::TreeEntries)?;
        validate_byte_limit(
            file_bytes,
            MAX_TREE_FILE_BYTES,
            ObservationLimitKind::FileBytes,
        )?;
        validate_byte_limit(
            total_bytes,
            MAX_TREE_TOTAL_BYTES,
            ObservationLimitKind::TotalTreeBytes,
        )?;
        validate_byte_limit(
            symlink_target_bytes,
            MAX_SYMLINK_TARGET_BYTES,
            ObservationLimitKind::SymlinkTargetBytes,
        )?;
        if entries < u64::from(depth) {
            return Err(ObservationRuntimeError::InvalidLimitRelationship {
                relationship: LimitRelationship::TreeEntriesCoverDepth,
            });
        }
        if total_bytes < file_bytes {
            return Err(ObservationRuntimeError::InvalidLimitRelationship {
                relationship: LimitRelationship::TotalTreeCoversFile,
            });
        }
        if total_bytes < symlink_target_bytes {
            return Err(ObservationRuntimeError::InvalidLimitRelationship {
                relationship: LimitRelationship::TotalTreeCoversSymlinkTarget,
            });
        }
        Ok(Self {
            depth,
            entries,
            file_bytes,
            total_bytes,
            symlink_target_bytes,
        })
    }
    pub const fn depth(self) -> u32 {
        self.depth
    }
    pub const fn entries(self) -> u64 {
        self.entries
    }
    pub const fn file_bytes(self) -> u64 {
        self.file_bytes
    }
    pub const fn total_bytes(self) -> u64 {
        self.total_bytes
    }
    pub const fn symlink_target_bytes(self) -> u64 {
        self.symlink_target_bytes
    }
}

impl From<ExternalTreeLimits> for ExternalTreeLimitsWire {
    fn from(value: ExternalTreeLimits) -> Self {
        Self {
            depth: value.depth,
            entries: value.entries,
            file_bytes: value.file_bytes,
            total_bytes: value.total_bytes,
            symlink_target_bytes: value.symlink_target_bytes,
        }
    }
}

impl TryFrom<ExternalTreeLimitsWire> for ExternalTreeLimits {
    type Error = ObservationRuntimeError;
    fn try_from(value: ExternalTreeLimitsWire) -> Result<Self, Self::Error> {
        Self::new(
            value.depth,
            value.entries,
            value.file_bytes,
            value.total_bytes,
            value.symlink_target_bytes,
        )
    }
}

impl<'de> Deserialize<'de> for ExternalTreeLimits {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        ExternalTreeLimitsWire::deserialize(deserializer)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}

fn validate_limit(
    value: u64,
    maximum: u64,
    kind: ObservationLimitKind,
) -> Result<(), ObservationRuntimeError> {
    if value == 0 {
        return Err(ObservationRuntimeError::ZeroLimit { limit: kind });
    }
    if value > maximum {
        return Err(ObservationRuntimeError::LimitExceedsMaximum { limit: kind });
    }
    Ok(())
}

fn validate_byte_limit(
    value: u64,
    maximum: u64,
    kind: ObservationLimitKind,
) -> Result<(), ObservationRuntimeError> {
    validate_limit(value, maximum, kind)?;
    usize::try_from(value)
        .map(|_| ())
        .map_err(|_| ObservationRuntimeError::LimitExceedsMaximum { limit: kind })
}

#[derive(Clone, Eq, PartialEq)]
pub struct ExecutableResolutionRequest {
    configured_binary: ConfiguredBinary,
    search_path: Option<OsString>,
}

impl ExecutableResolutionRequest {
    pub fn new(configured_binary: ConfiguredBinary, search_path: Option<OsString>) -> Self {
        Self {
            configured_binary,
            search_path,
        }
    }
    pub const fn configured_binary(&self) -> &ConfiguredBinary {
        &self.configured_binary
    }
    pub fn search_path(&self) -> Option<&OsStr> {
        self.search_path.as_deref()
    }
}

impl fmt::Debug for ExecutableResolutionRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ExecutableResolutionRequest")
            .field(
                "configured_binary_kind",
                &match self.configured_binary {
                    ConfiguredBinary::PathLookup(_) => "path_lookup",
                    ConfiguredBinary::Absolute(_) => "absolute",
                },
            )
            .field("search_path_supplied", &self.search_path.is_some())
            .finish()
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct NativeProcessRequest {
    executable: ExecutableIdentity,
    arguments: Vec<OsString>,
    environment: BTreeMap<OsString, OsString>,
    working_directory: Option<AbsolutePath>,
    limits: ProcessLimits,
}

impl NativeProcessRequest {
    pub fn new(
        executable: ExecutableIdentity,
        arguments: impl IntoIterator<Item = OsString>,
        environment: BTreeMap<OsString, OsString>,
        working_directory: Option<AbsolutePath>,
        limits: ProcessLimits,
    ) -> Self {
        Self {
            executable,
            arguments: arguments.into_iter().collect(),
            environment,
            working_directory,
            limits,
        }
    }
    pub const fn executable(&self) -> &ExecutableIdentity {
        &self.executable
    }
    pub fn arguments(&self) -> &[OsString] {
        &self.arguments
    }
    pub const fn environment(&self) -> &BTreeMap<OsString, OsString> {
        &self.environment
    }
    pub const fn working_directory(&self) -> Option<&AbsolutePath> {
        self.working_directory.as_ref()
    }
    pub const fn limits(&self) -> ProcessLimits {
        self.limits
    }
}

impl fmt::Debug for NativeProcessRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativeProcessRequest")
            .field("argument_count", &self.arguments.len())
            .field("environment_count", &self.environment.len())
            .field(
                "working_directory_supplied",
                &self.working_directory.is_some(),
            )
            .field("limits", &self.limits)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum NativeProcessStatus {
    Exited { code: u8 },
    Signaled { signal: NonZeroU32 },
}

impl NativeProcessStatus {
    pub const fn success(self) -> bool {
        matches!(self, Self::Exited { code: 0 })
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct NativeProcessOutput {
    status: NativeProcessStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    elapsed: Duration,
}

impl NativeProcessOutput {
    // Reserved for the bounded process adapter implemented by the next runtime story.
    #[allow(dead_code)]
    pub(crate) fn new(
        status: NativeProcessStatus,
        stdout: Vec<u8>,
        stderr: Vec<u8>,
        elapsed: Duration,
        limits: ProcessLimits,
    ) -> Result<Self, ObservationRuntimeError> {
        let stdout_bytes = u64::try_from(stdout.len()).map_err(|_| {
            ObservationRuntimeError::ProcessOutputLimitExceeded {
                stream: OutputStream::StandardOutput,
            }
        })?;
        if stdout_bytes > limits.stdout_bytes() {
            return Err(ObservationRuntimeError::ProcessOutputLimitExceeded {
                stream: OutputStream::StandardOutput,
            });
        }
        let stderr_bytes = u64::try_from(stderr.len()).map_err(|_| {
            ObservationRuntimeError::ProcessOutputLimitExceeded {
                stream: OutputStream::StandardError,
            }
        })?;
        if stderr_bytes > limits.stderr_bytes() {
            return Err(ObservationRuntimeError::ProcessOutputLimitExceeded {
                stream: OutputStream::StandardError,
            });
        }
        let combined = stdout_bytes.checked_add(stderr_bytes).ok_or(
            ObservationRuntimeError::ProcessOutputLimitExceeded {
                stream: OutputStream::Combined,
            },
        )?;
        if combined > limits.combined_output_bytes() {
            return Err(ObservationRuntimeError::ProcessOutputLimitExceeded {
                stream: OutputStream::Combined,
            });
        }
        if elapsed > limits.deadline() {
            return Err(ObservationRuntimeError::ProcessDeadlineExceeded);
        }
        Ok(Self {
            status,
            stdout,
            stderr,
            elapsed,
        })
    }
    pub const fn status(&self) -> NativeProcessStatus {
        self.status
    }
    pub fn stdout(&self) -> &[u8] {
        &self.stdout
    }
    pub fn stderr(&self) -> &[u8] {
        &self.stderr
    }
    pub const fn elapsed(&self) -> Duration {
        self.elapsed
    }
}

impl fmt::Debug for NativeProcessOutput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativeProcessOutput")
            .field("status", &self.status)
            .field("stdout_bytes", &self.stdout.len())
            .field("stderr_bytes", &self.stderr.len())
            .field("elapsed", &self.elapsed)
            .finish()
    }
}

#[derive(Clone, PartialEq)]
pub struct DecodedJson(serde_json::Value);

impl DecodedJson {
    // Reserved for the strict decoder after it validates the complete source document.
    #[allow(dead_code)]
    pub(crate) const fn new(value: serde_json::Value) -> Self {
        Self(value)
    }
    pub const fn value(&self) -> &serde_json::Value {
        &self.0
    }
    pub fn into_value(self) -> serde_json::Value {
        self.0
    }
}

impl fmt::Debug for DecodedJson {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DecodedJson")
            .field("kind", &json_kind(&self.0))
            .finish_non_exhaustive()
    }
}

fn json_kind(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalTreeEntryKind {
    Directory,
    File,
    Symlink,
}

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
#[allow(dead_code)]
enum ExternalTreePayload {
    Directory,
    File(Vec<u8>),
    Symlink(Vec<u8>),
}

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
pub struct ExternalTreeEntry {
    path: RelativeArtifactPath,
    payload: ExternalTreePayload,
}

impl ExternalTreeEntry {
    // Reserved for the descriptor-relative tree adapter; callers only receive entries.
    #[allow(dead_code)]
    pub(crate) const fn directory(path: RelativeArtifactPath) -> Self {
        Self {
            path,
            payload: ExternalTreePayload::Directory,
        }
    }
    #[allow(dead_code)]
    pub(crate) fn file(path: RelativeArtifactPath, bytes: Vec<u8>) -> Self {
        Self {
            path,
            payload: ExternalTreePayload::File(bytes),
        }
    }
    #[allow(dead_code)]
    pub(crate) fn symlink(path: RelativeArtifactPath, target: Vec<u8>) -> Self {
        Self {
            path,
            payload: ExternalTreePayload::Symlink(target),
        }
    }
    pub const fn path(&self) -> &RelativeArtifactPath {
        &self.path
    }
    pub const fn kind(&self) -> ExternalTreeEntryKind {
        match &self.payload {
            ExternalTreePayload::Directory => ExternalTreeEntryKind::Directory,
            ExternalTreePayload::File(_) => ExternalTreeEntryKind::File,
            ExternalTreePayload::Symlink(_) => ExternalTreeEntryKind::Symlink,
        }
    }
    pub fn file_bytes(&self) -> Option<&[u8]> {
        match &self.payload {
            ExternalTreePayload::File(bytes) => Some(bytes),
            _ => None,
        }
    }
    pub fn symlink_target(&self) -> Option<&[u8]> {
        match &self.payload {
            ExternalTreePayload::Symlink(target) => Some(target),
            _ => None,
        }
    }
}

impl fmt::Debug for ExternalTreeEntry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.payload {
            ExternalTreePayload::Directory => {
                formatter.debug_struct("Directory").finish_non_exhaustive()
            }
            ExternalTreePayload::File(bytes) => formatter
                .debug_struct("File")
                .field("byte_count", &bytes.len())
                .finish_non_exhaustive(),
            ExternalTreePayload::Symlink(target) => formatter
                .debug_struct("Symlink")
                .field("target_bytes", &target.len())
                .finish_non_exhaustive(),
        }
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct ExternalTreeSnapshot {
    entries: Vec<ExternalTreeEntry>,
}

impl ExternalTreeSnapshot {
    // Reserved for the descriptor-relative tree adapter implemented by a later story.
    #[allow(dead_code)]
    pub(crate) fn new(
        entries: impl IntoIterator<Item = ExternalTreeEntry>,
        limits: ExternalTreeLimits,
    ) -> Result<Self, ObservationRuntimeError> {
        let mut entries = entries.into_iter().collect::<Vec<_>>();
        let entry_count = u64::try_from(entries.len())
            .map_err(|_| ObservationRuntimeError::TreeEntryLimitExceeded)?;
        if entry_count > limits.entries() {
            return Err(ObservationRuntimeError::TreeEntryLimitExceeded);
        }
        entries.sort();
        let mut paths = BTreeSet::new();
        if entries.iter().any(|entry| !paths.insert(entry.path())) {
            return Err(ObservationRuntimeError::DuplicateTreeEntry);
        }
        let mut total_bytes = 0_u64;
        for entry in &entries {
            let depth = u32::try_from(
                std::path::Path::new(entry.path().as_str())
                    .components()
                    .count(),
            )
            .map_err(|_| ObservationRuntimeError::TreeDepthLimitExceeded)?;
            if depth > limits.depth() {
                return Err(ObservationRuntimeError::TreeDepthLimitExceeded);
            }
            let payload_bytes = match &entry.payload {
                ExternalTreePayload::Directory => 0,
                ExternalTreePayload::File(bytes) => {
                    let length = u64::try_from(bytes.len())
                        .map_err(|_| ObservationRuntimeError::TreeFileLimitExceeded)?;
                    if length > limits.file_bytes() {
                        return Err(ObservationRuntimeError::TreeFileLimitExceeded);
                    }
                    length
                }
                ExternalTreePayload::Symlink(target) => {
                    let length = u64::try_from(target.len())
                        .map_err(|_| ObservationRuntimeError::TreeSymlinkTargetLimitExceeded)?;
                    if length > limits.symlink_target_bytes() {
                        return Err(ObservationRuntimeError::TreeSymlinkTargetLimitExceeded);
                    }
                    length
                }
            };
            total_bytes = total_bytes
                .checked_add(payload_bytes)
                .ok_or(ObservationRuntimeError::TreeTotalLimitExceeded)?;
            if total_bytes > limits.total_bytes() {
                return Err(ObservationRuntimeError::TreeTotalLimitExceeded);
            }
        }
        Ok(Self { entries })
    }
    pub fn entries(&self) -> &[ExternalTreeEntry] {
        &self.entries
    }

    /// Return a bounded snapshot without one top-level metadata entry and its
    /// descendants. Source-control metadata is not part of a managed skill or
    /// other complete resource tree, so adapters can exclude it before
    /// fingerprinting and publication without weakening the filesystem
    /// observer's general-purpose behavior.
    pub fn without_top_level_directory(
        &self,
        name: &str,
        limits: ExternalTreeLimits,
    ) -> Result<Self, ObservationRuntimeError> {
        let prefix = format!("{name}/");
        Self::new(
            self.entries
                .iter()
                .filter(|entry| {
                    let path = entry.path().as_str();
                    path != name && !path.starts_with(&prefix)
                })
                .cloned(),
            limits,
        )
    }
}

impl fmt::Debug for ExternalTreeSnapshot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ExternalTreeSnapshot")
            .field("entry_count", &self.entries.len())
            .finish()
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct ExternalTreeRequest {
    root: AbsolutePath,
    limits: ExternalTreeLimits,
}

impl ExternalTreeRequest {
    pub const fn new(root: AbsolutePath, limits: ExternalTreeLimits) -> Self {
        Self { root, limits }
    }
    pub const fn root(&self) -> &AbsolutePath {
        &self.root
    }
    pub const fn limits(&self) -> ExternalTreeLimits {
        self.limits
    }
}

impl fmt::Debug for ExternalTreeRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ExternalTreeRequest")
            .field("limits", &self.limits)
            .finish_non_exhaustive()
    }
}

pub trait ExecutableResolver {
    fn resolve(
        &self,
        request: &ExecutableResolutionRequest,
    ) -> Result<ExecutableIdentity, ObservationRuntimeError>;

    fn revalidate(&self, executable: &ExecutableIdentity) -> Result<(), ObservationRuntimeError>;
}

pub trait NativeProcessRunner {
    fn run(
        &self,
        request: &NativeProcessRequest,
    ) -> Result<NativeProcessOutput, ObservationRuntimeError>;
}

pub trait StrictJsonDecoder {
    fn decode(
        &self,
        input: &[u8],
        limits: JsonLimits,
    ) -> Result<DecodedJson, ObservationRuntimeError>;
}

pub trait ExternalTreeObserver {
    fn observe(
        &self,
        request: &ExternalTreeRequest,
    ) -> Result<ExternalTreeSnapshot, ObservationRuntimeError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ExecutableFileIdentity, NativeId};

    fn executable(secret: &str) -> ExecutableIdentity {
        ExecutableIdentity::new(
            AbsolutePath::new(format!("/tmp/{secret}")).unwrap(),
            ExecutableFileIdentity::new(7, 11),
        )
    }

    fn process_limits() -> ProcessLimits {
        ProcessLimits::new(1_000, 1024, 2048, 4096).unwrap()
    }

    fn tree_limits() -> ExternalTreeLimits {
        ExternalTreeLimits::new(8, 32, 4096, 8192, 1024).unwrap()
    }

    #[test]
    fn process_limits_reject_zero_hard_max_plus_one_and_invalid_relationships() {
        for (values, expected) in [
            (
                (0, 1, 1, 1),
                ObservationRuntimeError::ZeroLimit {
                    limit: ObservationLimitKind::ProcessDeadlineMilliseconds,
                },
            ),
            (
                (MAX_PROCESS_DEADLINE_MILLISECONDS + 1, 1, 1, 1),
                ObservationRuntimeError::LimitExceedsMaximum {
                    limit: ObservationLimitKind::ProcessDeadlineMilliseconds,
                },
            ),
            (
                (1, 2, 1, 1),
                ObservationRuntimeError::InvalidLimitRelationship {
                    relationship: LimitRelationship::CombinedOutputCoversStandardOutput,
                },
            ),
        ] {
            assert_eq!(
                ProcessLimits::new(values.0, values.1, values.2, values.3),
                Err(expected)
            );
        }
        for deadline in [
            MAX_PROCESS_DEADLINE_MILLISECONDS - 1,
            MAX_PROCESS_DEADLINE_MILLISECONDS,
        ] {
            assert!(ProcessLimits::new(deadline, 1, 1, 1).is_ok());
        }
        for stdout in [MAX_PROCESS_STREAM_BYTES - 1, MAX_PROCESS_STREAM_BYTES] {
            assert!(ProcessLimits::new(1, stdout, 1, MAX_PROCESS_COMBINED_BYTES).is_ok());
        }
        for stderr in [MAX_PROCESS_STREAM_BYTES - 1, MAX_PROCESS_STREAM_BYTES] {
            assert!(ProcessLimits::new(1, 1, stderr, MAX_PROCESS_COMBINED_BYTES).is_ok());
        }
        for combined in [MAX_PROCESS_COMBINED_BYTES - 1, MAX_PROCESS_COMBINED_BYTES] {
            assert!(ProcessLimits::new(1, 1, 1, combined).is_ok());
        }
        for values in [(1, 0, 1, 1), (1, 1, 0, 1), (1, 1, 1, 0)] {
            assert!(ProcessLimits::new(values.0, values.1, values.2, values.3).is_err());
        }
        for values in [
            (
                1,
                MAX_PROCESS_STREAM_BYTES + 1,
                1,
                MAX_PROCESS_COMBINED_BYTES,
            ),
            (
                1,
                1,
                MAX_PROCESS_STREAM_BYTES + 1,
                MAX_PROCESS_COMBINED_BYTES,
            ),
            (1, 1, 1, MAX_PROCESS_COMBINED_BYTES + 1),
        ] {
            assert!(ProcessLimits::new(values.0, values.1, values.2, values.3).is_err());
        }
        assert_eq!(process_limits().deadline(), Duration::from_secs(1));
    }

    #[test]
    fn json_limits_enforce_stack_safe_depth_and_strict_serde() {
        for bytes in [MAX_JSON_BYTES - 1, MAX_JSON_BYTES] {
            assert!(JsonLimits::new(bytes, 1).is_ok());
        }
        for depth in [MAX_JSON_DEPTH - 1, MAX_JSON_DEPTH] {
            let limits = JsonLimits::new(MAX_JSON_BYTES, depth).unwrap();
            let encoded = serde_json::to_string(&limits).unwrap();
            assert_eq!(
                serde_json::from_str::<JsonLimits>(&encoded).unwrap(),
                limits
            );
        }
        assert!(matches!(
            JsonLimits::new(1, MAX_JSON_DEPTH + 1),
            Err(ObservationRuntimeError::LimitExceedsMaximum {
                limit: ObservationLimitKind::JsonDepth
            })
        ));
        assert!(JsonLimits::new(0, 1).is_err());
        assert!(JsonLimits::new(1, 0).is_err());
        assert!(JsonLimits::new(MAX_JSON_BYTES + 1, 1).is_err());
        assert!(serde_json::from_str::<JsonLimits>(r#"{"bytes":1,"depth":1,"extra":1}"#).is_err());
    }

    #[test]
    fn tree_limits_enforce_hard_maxima_and_cross_field_invariants() {
        for depth in [MAX_TREE_DEPTH - 1, MAX_TREE_DEPTH] {
            assert!(
                ExternalTreeLimits::new(
                    depth,
                    MAX_TREE_ENTRIES,
                    MAX_TREE_FILE_BYTES,
                    MAX_TREE_TOTAL_BYTES,
                    MAX_SYMLINK_TARGET_BYTES,
                )
                .is_ok()
            );
        }
        for entries in [MAX_TREE_ENTRIES - 1, MAX_TREE_ENTRIES] {
            assert!(ExternalTreeLimits::new(1, entries, 1, 1, 1).is_ok());
        }
        for file_bytes in [MAX_TREE_FILE_BYTES - 1, MAX_TREE_FILE_BYTES] {
            assert!(ExternalTreeLimits::new(1, 1, file_bytes, MAX_TREE_TOTAL_BYTES, 1).is_ok());
        }
        for total_bytes in [MAX_TREE_TOTAL_BYTES - 1, MAX_TREE_TOTAL_BYTES] {
            assert!(ExternalTreeLimits::new(1, 1, 1, total_bytes, 1).is_ok());
        }
        for target_bytes in [MAX_SYMLINK_TARGET_BYTES - 1, MAX_SYMLINK_TARGET_BYTES] {
            assert!(ExternalTreeLimits::new(1, 1, 1, MAX_TREE_TOTAL_BYTES, target_bytes).is_ok());
        }
        for values in [
            (0, 1, 1, 1, 1),
            (1, 0, 1, 1, 1),
            (1, 1, 0, 1, 1),
            (1, 1, 1, 0, 1),
            (1, 1, 1, 1, 0),
        ] {
            assert!(
                ExternalTreeLimits::new(values.0, values.1, values.2, values.3, values.4).is_err()
            );
        }
        for values in [
            (MAX_TREE_DEPTH + 1, MAX_TREE_ENTRIES, 1, 1, 1),
            (1, MAX_TREE_ENTRIES + 1, 1, 1, 1),
            (1, 1, MAX_TREE_FILE_BYTES + 1, MAX_TREE_TOTAL_BYTES, 1),
            (1, 1, 1, MAX_TREE_TOTAL_BYTES + 1, 1),
            (1, 1, 1, MAX_TREE_TOTAL_BYTES, MAX_SYMLINK_TARGET_BYTES + 1),
        ] {
            assert!(
                ExternalTreeLimits::new(values.0, values.1, values.2, values.3, values.4).is_err()
            );
        }
        assert!(matches!(
            ExternalTreeLimits::new(MAX_TREE_DEPTH + 1, MAX_TREE_ENTRIES, 1, 1, 1),
            Err(ObservationRuntimeError::LimitExceedsMaximum {
                limit: ObservationLimitKind::TreeDepth
            })
        ));
        assert!(matches!(
            ExternalTreeLimits::new(2, 1, 1, 1, 1),
            Err(ObservationRuntimeError::InvalidLimitRelationship {
                relationship: LimitRelationship::TreeEntriesCoverDepth
            })
        ));
        assert!(matches!(
            ExternalTreeLimits::new(1, 1, 2, 1, 1),
            Err(ObservationRuntimeError::InvalidLimitRelationship {
                relationship: LimitRelationship::TotalTreeCoversFile
            })
        ));
        assert!(matches!(
            ExternalTreeLimits::new(1, 1, 1, 1, 2),
            Err(ObservationRuntimeError::InvalidLimitRelationship {
                relationship: LimitRelationship::TotalTreeCoversSymlinkTarget
            })
        ));
        let encoded = serde_json::to_string(&tree_limits()).unwrap();
        assert_eq!(
            serde_json::from_str::<ExternalTreeLimits>(&encoded).unwrap(),
            tree_limits()
        );
    }

    #[test]
    fn sensitive_requests_results_and_errors_are_debug_and_display_safe() {
        const SECRET: &str = "secret-native-canary";
        let resolution = ExecutableResolutionRequest::new(
            ConfiguredBinary::path_lookup(NativeId::new(SECRET).unwrap()).unwrap(),
            Some(OsString::from(SECRET)),
        );
        let process = NativeProcessRequest::new(
            executable(SECRET),
            [OsString::from(SECRET)],
            BTreeMap::from([(OsString::from("TOKEN"), OsString::from(SECRET))]),
            Some(AbsolutePath::new(format!("/tmp/{SECRET}")).unwrap()),
            process_limits(),
        );
        let output = NativeProcessOutput::new(
            NativeProcessStatus::Exited { code: 17 },
            SECRET.as_bytes().to_vec(),
            SECRET.as_bytes().to_vec(),
            Duration::from_millis(4),
            ProcessLimits::new(100, 64, 64, 128).unwrap(),
        )
        .unwrap();
        let decoded = DecodedJson::new(serde_json::json!({"secret": SECRET}));
        let tree = ExternalTreeSnapshot::new(
            [
                ExternalTreeEntry::file(
                    RelativeArtifactPath::new("secret.txt").unwrap(),
                    SECRET.as_bytes().to_vec(),
                ),
                ExternalTreeEntry::symlink(
                    RelativeArtifactPath::new("link").unwrap(),
                    SECRET.as_bytes().to_vec(),
                ),
            ],
            tree_limits(),
        )
        .unwrap();
        let tree_request = ExternalTreeRequest::new(
            AbsolutePath::new(format!("/tmp/{SECRET}")).unwrap(),
            tree_limits(),
        );

        for rendered in [
            format!("{resolution:?}"),
            format!("{process:?}"),
            format!("{output:?}"),
            format!("{decoded:?}"),
            format!("{tree:?}"),
            format!("{tree_request:?}"),
        ] {
            assert!(
                !rendered.contains(SECRET),
                "unsafe debug output: {rendered}"
            );
        }
        for error in [
            ObservationRuntimeError::InvalidSearchPath,
            ObservationRuntimeError::ProcessOutputLimitExceeded {
                stream: OutputStream::StandardError,
            },
            ObservationRuntimeError::JsonInvalidSyntax,
            ObservationRuntimeError::TreeEntryUnreadable,
        ] {
            assert!(!format!("{error:?}").contains(SECRET));
            assert!(!error.to_string().contains(SECRET));
            assert!(!serde_json::to_string(&error).unwrap().contains(SECRET));
        }
    }

    #[test]
    fn tree_snapshots_sort_entries_reject_duplicate_paths_and_preserve_payloads() {
        let first = ExternalTreeEntry::file(
            RelativeArtifactPath::new("a/file").unwrap(),
            b"content".to_vec(),
        );
        let second = ExternalTreeEntry::directory(RelativeArtifactPath::new("a").unwrap());
        let snapshot = ExternalTreeSnapshot::new([first.clone(), second], tree_limits()).unwrap();
        assert_eq!(snapshot.entries()[0].path().as_str(), "a");
        assert_eq!(
            snapshot.entries()[0].kind(),
            ExternalTreeEntryKind::Directory
        );
        assert_eq!(
            snapshot.entries()[1].file_bytes(),
            Some(b"content".as_slice())
        );
        assert_eq!(
            ExternalTreeSnapshot::new([first.clone(), first], tree_limits()),
            Err(ObservationRuntimeError::DuplicateTreeEntry)
        );
        assert!(!NativeProcessStatus::Exited { code: 17 }.success());
        assert!(NativeProcessStatus::Exited { code: 0 }.success());
    }

    #[test]
    fn bounded_result_builders_reject_every_payload_bypass() {
        let process_limits = ProcessLimits::new(10, 2, 2, 3).unwrap();
        for (stdout, stderr, elapsed, expected) in [
            (
                vec![0; 3],
                vec![],
                Duration::from_millis(1),
                ObservationRuntimeError::ProcessOutputLimitExceeded {
                    stream: OutputStream::StandardOutput,
                },
            ),
            (
                vec![],
                vec![0; 3],
                Duration::from_millis(1),
                ObservationRuntimeError::ProcessOutputLimitExceeded {
                    stream: OutputStream::StandardError,
                },
            ),
            (
                vec![0; 2],
                vec![0; 2],
                Duration::from_millis(1),
                ObservationRuntimeError::ProcessOutputLimitExceeded {
                    stream: OutputStream::Combined,
                },
            ),
            (
                vec![],
                vec![],
                Duration::from_millis(11),
                ObservationRuntimeError::ProcessDeadlineExceeded,
            ),
        ] {
            assert_eq!(
                NativeProcessOutput::new(
                    NativeProcessStatus::Exited { code: 0 },
                    stdout,
                    stderr,
                    elapsed,
                    process_limits,
                ),
                Err(expected)
            );
        }

        let limits = ExternalTreeLimits::new(2, 2, 2, 3, 2).unwrap();
        let file = |path: &str, size| {
            ExternalTreeEntry::file(RelativeArtifactPath::new(path).unwrap(), vec![0; size])
        };
        let link = |path: &str, size| {
            ExternalTreeEntry::symlink(RelativeArtifactPath::new(path).unwrap(), vec![0; size])
        };
        assert_eq!(
            ExternalTreeSnapshot::new([file("a", 1), file("b", 1), file("c", 1)], limits),
            Err(ObservationRuntimeError::TreeEntryLimitExceeded)
        );
        assert_eq!(
            ExternalTreeSnapshot::new([file("a/b/c", 1)], limits),
            Err(ObservationRuntimeError::TreeDepthLimitExceeded)
        );
        assert_eq!(
            ExternalTreeSnapshot::new([file("a", 3)], limits),
            Err(ObservationRuntimeError::TreeFileLimitExceeded)
        );
        assert_eq!(
            ExternalTreeSnapshot::new([link("a", 3)], limits),
            Err(ObservationRuntimeError::TreeSymlinkTargetLimitExceeded)
        );
        assert_eq!(
            ExternalTreeSnapshot::new([file("a", 2), link("b", 2)], limits),
            Err(ObservationRuntimeError::TreeTotalLimitExceeded)
        );
    }

    struct FakePorts;

    impl ExecutableResolver for FakePorts {
        fn resolve(
            &self,
            _request: &ExecutableResolutionRequest,
        ) -> Result<ExecutableIdentity, ObservationRuntimeError> {
            Ok(executable("resolved"))
        }
        fn revalidate(
            &self,
            _executable: &ExecutableIdentity,
        ) -> Result<(), ObservationRuntimeError> {
            Ok(())
        }
    }

    impl NativeProcessRunner for FakePorts {
        fn run(
            &self,
            _request: &NativeProcessRequest,
        ) -> Result<NativeProcessOutput, ObservationRuntimeError> {
            NativeProcessOutput::new(
                NativeProcessStatus::Exited { code: 0 },
                b"{}".to_vec(),
                Vec::new(),
                Duration::from_millis(1),
                process_limits(),
            )
        }
    }

    impl StrictJsonDecoder for FakePorts {
        fn decode(
            &self,
            _input: &[u8],
            _limits: JsonLimits,
        ) -> Result<DecodedJson, ObservationRuntimeError> {
            Ok(DecodedJson::new(serde_json::json!({})))
        }
    }

    impl ExternalTreeObserver for FakePorts {
        fn observe(
            &self,
            _request: &ExternalTreeRequest,
        ) -> Result<ExternalTreeSnapshot, ObservationRuntimeError> {
            ExternalTreeSnapshot::new([], _request.limits())
        }
    }

    #[test]
    fn behavior_ports_are_composable_without_concrete_io() {
        let ports = FakePorts;
        let resolution = ExecutableResolutionRequest::new(
            ConfiguredBinary::path_lookup(NativeId::new("codex").unwrap()).unwrap(),
            Some(OsString::from("/usr/bin")),
        );
        let executable = ports.resolve(&resolution).unwrap();
        ports.revalidate(&executable).unwrap();
        let process = ports
            .run(&NativeProcessRequest::new(
                executable,
                [],
                BTreeMap::new(),
                None,
                process_limits(),
            ))
            .unwrap();
        let json = ports
            .decode(process.stdout(), JsonLimits::new(1024, 8).unwrap())
            .unwrap();
        assert!(json.value().is_object());
        assert!(
            ports
                .observe(&ExternalTreeRequest::new(
                    AbsolutePath::new("/tmp/tree").unwrap(),
                    tree_limits(),
                ))
                .unwrap()
                .entries()
                .is_empty()
        );
    }
}
