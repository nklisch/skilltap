use std::fmt;

use crate::{
    domain::{AbsolutePath, Fingerprint, RelativeArtifactPath, ResolvedRevision, Source},
    runtime::DirectoryIdentity,
    storage::{ArtifactTree, ManagedProjection},
};

/// A confined source checkout resolved by orchestration for managed projection.
///
/// Adapters read catalog and plugin content from `root` and use `source` as the
/// authoritative provenance identity. They do not resolve or acquire sources.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedSourceCheckout {
    root: AbsolutePath,
    source: Source,
    revision: Option<ResolvedRevision>,
}

impl ResolvedSourceCheckout {
    /// Constructs a checkout from components validated by source resolution.
    pub fn new(root: AbsolutePath, source: Source, revision: Option<ResolvedRevision>) -> Self {
        Self {
            root,
            source,
            revision,
        }
    }

    pub const fn root(&self) -> &AbsolutePath {
        &self.root
    }

    pub const fn source(&self) -> &Source {
        &self.source
    }

    pub const fn revision(&self) -> Option<&ResolvedRevision> {
        self.revision.as_ref()
    }
}

/// One complete managed skill-tree write.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedPluginWrite {
    pub root: AbsolutePath,
    pub destination: RelativeArtifactPath,
    pub desired_tree: Option<ArtifactTree>,
    pub expected_tree: Option<ArtifactTree>,
    pub expected_identity: Option<DirectoryIdentity>,
}

/// One adapter-encoded managed file write.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedFileWrite {
    pub root: AbsolutePath,
    pub destination: RelativeArtifactPath,
    pub expected: Option<Vec<u8>>,
    pub desired: Option<Vec<u8>>,
}

/// Pure target-bound writes plus complete projection evidence from an adapter.
///
/// Shared orchestration consumes the manifest and fingerprints directly rather
/// than reconstructing target-native semantics from encoded file writes.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ManagedProjectionPlan {
    pub trees: Vec<ManagedPluginWrite>,
    pub files: Vec<ManagedFileWrite>,
    /// Complete target-bound Skill, MCP, and acknowledged Omitted evidence.
    pub manifest: Vec<ManagedProjection>,
    /// Aggregate fingerprint of currently observed projected surfaces.
    pub current_fingerprint: Option<Fingerprint>,
    /// Aggregate fingerprint of desired projected surfaces.
    pub desired_fingerprint: Option<Fingerprint>,
}

/// Typed failures at the managed-projection adapter boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ManagedProjectionError {
    UnsupportedResourceKind,
    RequiredUnsupported,
    SourceMissing,
    SourceUnavailable,
    CatalogMissing,
    CatalogInvalid {
        detail: &'static str,
    },
    PluginMissing {
        detail: &'static str,
    },
    PluginSourceInvalid {
        detail: &'static str,
    },
    PluginUnreadable {
        detail: &'static str,
    },
    McpInvalid {
        detail: &'static str,
    },
    McpConflict,
    Drifted {
        detail: &'static str,
    },
    /// A failure code defined by one adapter, not an alias for a canonical
    /// variant's code.
    Other {
        code: &'static str,
        summary: &'static str,
    },
}

impl ManagedProjectionError {
    /// Stable diagnostic code surfaced unchanged by application orchestration.
    pub const fn code(&self) -> &'static str {
        match self {
            Self::UnsupportedResourceKind => "unsupported_resource_kind",
            Self::RequiredUnsupported => "required_unsupported",
            Self::SourceMissing => "managed_project_source_missing",
            Self::SourceUnavailable => "managed_project_source_unavailable",
            Self::CatalogMissing => "managed_project_catalog_missing",
            Self::CatalogInvalid { .. } => "managed_project_catalog_invalid",
            Self::PluginMissing { .. } => "managed_project_plugin_invalid",
            Self::PluginSourceInvalid { .. } => "managed_project_plugin_source_invalid",
            Self::PluginUnreadable { .. } => "managed_project_plugin_unreadable",
            Self::McpInvalid { .. } => "managed_project_mcp_invalid",
            Self::McpConflict => "managed_project_mcp_conflict",
            Self::Drifted { .. } => "managed_project_drifted",
            Self::Other { code, .. } => code,
        }
    }

    /// Stable, bounded summary suitable for human and structured output.
    pub const fn summary(&self) -> &'static str {
        match self {
            Self::UnsupportedResourceKind => {
                "The selected resource kind does not support managed projection."
            }
            Self::RequiredUnsupported => {
                "A required plugin component cannot be represented faithfully by the selected target."
            }
            Self::SourceMissing => "The managed project marketplace has no explicit source.",
            Self::SourceUnavailable => {
                "The Git marketplace source could not be cloned and checked out safely."
            }
            Self::CatalogMissing => "The selected source has no compatible marketplace document.",
            Self::CatalogInvalid { detail }
            | Self::PluginMissing { detail }
            | Self::PluginSourceInvalid { detail }
            | Self::PluginUnreadable { detail }
            | Self::McpInvalid { detail }
            | Self::Drifted { detail } => detail,
            Self::McpConflict => "The existing mcp_servers value is not a table.",
            Self::Other { summary, .. } => summary,
        }
    }
}

impl fmt::Display for ManagedProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.summary())
    }
}

impl std::error::Error for ManagedProjectionError {}

#[cfg(test)]
mod tests {
    use super::ManagedProjectionError;

    #[test]
    fn managed_projection_error_codes_are_stable() {
        let cases = [
            (
                ManagedProjectionError::UnsupportedResourceKind,
                "unsupported_resource_kind",
            ),
            (
                ManagedProjectionError::RequiredUnsupported,
                "required_unsupported",
            ),
            (
                ManagedProjectionError::SourceMissing,
                "managed_project_source_missing",
            ),
            (
                ManagedProjectionError::SourceUnavailable,
                "managed_project_source_unavailable",
            ),
            (
                ManagedProjectionError::CatalogMissing,
                "managed_project_catalog_missing",
            ),
            (
                ManagedProjectionError::CatalogInvalid {
                    detail: "Invalid catalog.",
                },
                "managed_project_catalog_invalid",
            ),
            (
                ManagedProjectionError::PluginMissing {
                    detail: "Invalid plugin.",
                },
                "managed_project_plugin_invalid",
            ),
            (
                ManagedProjectionError::PluginSourceInvalid {
                    detail: "Invalid plugin source.",
                },
                "managed_project_plugin_source_invalid",
            ),
            (
                ManagedProjectionError::PluginUnreadable {
                    detail: "Unreadable plugin.",
                },
                "managed_project_plugin_unreadable",
            ),
            (
                ManagedProjectionError::McpInvalid {
                    detail: "Invalid MCP declaration.",
                },
                "managed_project_mcp_invalid",
            ),
            (
                ManagedProjectionError::McpConflict,
                "managed_project_mcp_conflict",
            ),
            (
                ManagedProjectionError::Drifted {
                    detail: "Drifted projection.",
                },
                "managed_project_drifted",
            ),
            (
                ManagedProjectionError::Other {
                    code: "adapter_specific",
                    summary: "Adapter-specific failure.",
                },
                "adapter_specific",
            ),
        ];

        for (error, expected) in cases {
            assert_eq!(error.code(), expected);
        }
    }

    #[test]
    fn contextual_summaries_vary_without_changing_the_typed_code() {
        let invalid_json = ManagedProjectionError::McpInvalid {
            detail: "The plugin MCP declaration is invalid JSON.",
        };
        let missing_servers = ManagedProjectionError::McpInvalid {
            detail: "The plugin MCP declaration has no mcpServers object.",
        };

        assert_eq!(invalid_json.code(), "managed_project_mcp_invalid");
        assert_eq!(missing_servers.code(), "managed_project_mcp_invalid");
        assert_eq!(
            invalid_json.summary(),
            "The plugin MCP declaration is invalid JSON."
        );
        assert_eq!(
            missing_servers.summary(),
            "The plugin MCP declaration has no mcpServers object."
        );
    }
}
