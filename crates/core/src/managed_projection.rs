use std::fmt;

use crate::{
    domain::{
        AbsolutePath, ComponentId, EvidenceCode, Fingerprint, RelativeArtifactPath,
        ResolvedRevision, Source,
    },
    plugin_graph::ComponentDeclaration,
    runtime::DirectoryIdentity,
    storage::ArtifactTree,
};

/// A component omitted because the target cannot represent it faithfully.
///
/// Required unsupported components are errors and never enter a projection
/// plan. This type records only acknowledged optional loss.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OmittedComponent {
    pub id: ComponentId,
    pub consequence: EvidenceCode,
}

/// Source content acquired by a target adapter for managed projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AcquiredProjection {
    /// A complete plugin tree and its normalized component declarations.
    Plugin {
        tree: ArtifactTree,
        fingerprint: Fingerprint,
        declarations: Vec<ComponentDeclaration>,
        source: Source,
        installed_revision: Option<ResolvedRevision>,
    },
    /// A target-native catalog or document that is projected verbatim.
    MarketplaceCatalog {
        bytes: Vec<u8>,
        fingerprint: Fingerprint,
        source: Source,
        installed_revision: Option<ResolvedRevision>,
    },
}

impl AcquiredProjection {
    pub const fn fingerprint(&self) -> &Fingerprint {
        match self {
            Self::Plugin { fingerprint, .. } | Self::MarketplaceCatalog { fingerprint, .. } => {
                fingerprint
            }
        }
    }

    pub const fn source(&self) -> &Source {
        match self {
            Self::Plugin { source, .. } | Self::MarketplaceCatalog { source, .. } => source,
        }
    }

    pub const fn installed_revision(&self) -> Option<&ResolvedRevision> {
        match self {
            Self::Plugin {
                installed_revision, ..
            }
            | Self::MarketplaceCatalog {
                installed_revision, ..
            } => installed_revision.as_ref(),
        }
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

/// Pure target-bound writes and acknowledged omissions produced by an adapter.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ManagedProjectionPlan {
    pub trees: Vec<ManagedPluginWrite>,
    pub files: Vec<ManagedFileWrite>,
    pub omitted: Vec<OmittedComponent>,
}

/// Typed failures at the managed-projection adapter boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ManagedProjectionError {
    UnsupportedResourceKind,
    RequiredUnsupported,
    SourceMissing,
    SourceUnavailable,
    CatalogMissing,
    CatalogInvalid,
    PluginMissing,
    PluginSourceInvalid,
    PluginUnreadable,
    McpInvalid,
    McpConflict,
    Drifted,
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
            Self::CatalogInvalid => "managed_project_catalog_invalid",
            Self::PluginMissing => "managed_project_plugin_invalid",
            Self::PluginSourceInvalid => "managed_project_plugin_source_invalid",
            Self::PluginUnreadable => "managed_project_plugin_unreadable",
            Self::McpInvalid => "managed_project_mcp_invalid",
            Self::McpConflict => "managed_project_mcp_conflict",
            Self::Drifted => "managed_project_drifted",
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
            Self::CatalogMissing => {
                "The selected source has no Codex-compatible marketplace document."
            }
            Self::CatalogInvalid => "The selected marketplace document is invalid.",
            Self::PluginMissing => {
                "The selected plugin does not contain a valid Codex manifest and complete component graph."
            }
            Self::PluginSourceInvalid => {
                "The selected plugin source is not a contained local marketplace entry."
            }
            Self::PluginUnreadable => "The selected plugin tree could not be read safely.",
            Self::McpInvalid => "The plugin MCP declaration is invalid JSON.",
            Self::McpConflict => "The existing mcp_servers value is not a table.",
            Self::Drifted => "The managed project destination drifted; no files were changed.",
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
                ManagedProjectionError::CatalogInvalid,
                "managed_project_catalog_invalid",
            ),
            (
                ManagedProjectionError::PluginMissing,
                "managed_project_plugin_invalid",
            ),
            (
                ManagedProjectionError::PluginSourceInvalid,
                "managed_project_plugin_source_invalid",
            ),
            (
                ManagedProjectionError::PluginUnreadable,
                "managed_project_plugin_unreadable",
            ),
            (
                ManagedProjectionError::McpInvalid,
                "managed_project_mcp_invalid",
            ),
            (
                ManagedProjectionError::McpConflict,
                "managed_project_mcp_conflict",
            ),
            (ManagedProjectionError::Drifted, "managed_project_drifted"),
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
}
