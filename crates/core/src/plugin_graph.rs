//! Target-independent source plugin component graphs.
//!
//! Harness adapters parse their documented native manifests and emit
//! [`ComponentDeclaration`] values. This module validates and normalizes those
//! declarations without browsing marketplaces or retaining native payloads.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use crate::domain::{
    ComponentGraph, ComponentGraphError, ComponentId, ComponentKind, ComponentRequiredness,
    NativeId, RelativeArtifactPath, ResourceComponent, Source, SourceKind,
};

/// A component declaration emitted by a native harness adapter for one
/// explicitly selected plugin source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentDeclaration {
    pub id: ComponentId,
    pub kind: ComponentKind,
    pub requiredness: ComponentRequiredness,
    pub dependencies: BTreeSet<ComponentId>,
    pub relative_path: RelativeArtifactPath,
    pub declared_name: Option<String>,
}

/// Bounded source information retained for later materialization explanations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentProvenance {
    relative_path: RelativeArtifactPath,
    declared_name: Option<String>,
}

impl ComponentProvenance {
    pub const fn relative_path(&self) -> &RelativeArtifactPath {
        &self.relative_path
    }

    pub fn declared_name(&self) -> Option<&str> {
        self.declared_name.as_deref()
    }
}

/// A normalized source graph. Component semantics are represented by the
/// existing [`ComponentGraph`]; provenance remains alongside it so target
/// adapters can explain the source path without leaking native documents.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceComponentGraph {
    source: Source,
    components: ComponentGraph,
    provenance: BTreeMap<ComponentId, ComponentProvenance>,
}

impl SourceComponentGraph {
    pub const fn source(&self) -> &Source {
        &self.source
    }

    pub const fn components(&self) -> &ComponentGraph {
        &self.components
    }

    pub fn provenance(&self, id: &ComponentId) -> Option<&ComponentProvenance> {
        self.provenance.get(id)
    }

    pub fn iter_provenance(
        &self,
    ) -> impl ExactSizeIterator<Item = (&ComponentId, &ComponentProvenance)> {
        self.provenance.iter()
    }
}

/// Errors emitted while reading an explicitly selected source. Native output
/// is intentionally not retained in this error type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PluginGraphReadError {
    SourceUnavailable,
    MalformedManifest,
    UnsupportedSourceKind(SourceKind),
}

impl fmt::Display for PluginGraphReadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SourceUnavailable => {
                formatter.write_str("the selected plugin source is unavailable")
            }
            Self::MalformedManifest => {
                formatter.write_str("the selected plugin manifest is malformed")
            }
            Self::UnsupportedSourceKind(kind) => {
                write!(
                    formatter,
                    "plugin graph reading does not support {kind} sources"
                )
            }
        }
    }
}

impl std::error::Error for PluginGraphReadError {}

/// Errors raised while normalizing adapter declarations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PluginGraphError {
    Graph(ComponentGraphError),
    InvalidDeclaredName { component: ComponentId },
}

impl fmt::Display for PluginGraphError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Graph(error) => error.fmt(formatter),
            Self::InvalidDeclaredName { component } => {
                write!(
                    formatter,
                    "component `{component}` has an invalid declared name"
                )
            }
        }
    }
}

impl std::error::Error for PluginGraphError {}

/// Core port implemented by a Codex or Claude adapter.
pub trait PluginGraphReader {
    fn read(&self, source: &Source) -> Result<Vec<ComponentDeclaration>, PluginGraphReadError>;
}

/// Validate and normalize declarations into a deterministic source graph.
pub fn normalize(
    source: Source,
    declarations: impl IntoIterator<Item = ComponentDeclaration>,
) -> Result<SourceComponentGraph, PluginGraphError> {
    let mut resources = Vec::new();
    let mut provenance = BTreeMap::new();
    for declaration in declarations {
        let id = declaration.id.clone();
        if declaration
            .declared_name
            .as_deref()
            .is_some_and(|name| NativeId::new(name).is_err())
        {
            return Err(PluginGraphError::InvalidDeclaredName { component: id });
        }
        resources.push(ResourceComponent {
            id: id.clone(),
            kind: declaration.kind,
            requiredness: declaration.requiredness,
            dependencies: declaration.dependencies,
        });
        if provenance
            .insert(
                id.clone(),
                ComponentProvenance {
                    relative_path: declaration.relative_path,
                    declared_name: declaration.declared_name,
                },
            )
            .is_some()
        {
            return Err(PluginGraphError::Graph(
                ComponentGraphError::DuplicateComponent { id },
            ));
        }
    }
    let components = ComponentGraph::new(resources).map_err(PluginGraphError::Graph)?;
    Ok(SourceComponentGraph {
        source,
        components,
        provenance,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{SourceKind, SourceLocator};

    fn id(value: &str) -> ComponentId {
        ComponentId::new(value).unwrap()
    }

    fn path(value: &str) -> RelativeArtifactPath {
        RelativeArtifactPath::new(value).unwrap()
    }

    fn source() -> Source {
        Source::new(
            SourceKind::Git,
            SourceLocator::new("https://example.test/plugin.git").unwrap(),
            None,
        )
        .unwrap()
    }

    fn declaration(id_value: &str, path_value: &str) -> ComponentDeclaration {
        ComponentDeclaration {
            id: id(id_value),
            kind: ComponentKind::Skill,
            requiredness: ComponentRequiredness::Required,
            dependencies: BTreeSet::new(),
            relative_path: path(path_value),
            declared_name: Some(id_value.to_owned()),
        }
    }

    #[test]
    fn normalization_is_deterministic_and_retains_bounded_provenance() {
        let graph = normalize(
            source(),
            [
                declaration("skill:z", "skills/z"),
                declaration("skill:a", "skills/a"),
            ],
        )
        .unwrap();
        let ids = graph
            .components()
            .iter()
            .map(|(id, _)| id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(ids, ["skill:a", "skill:z"]);
        assert_eq!(
            graph
                .provenance(&id("skill:a"))
                .unwrap()
                .relative_path()
                .as_str(),
            "skills/a"
        );
        assert_eq!(
            graph.provenance(&id("skill:z")).unwrap().declared_name(),
            Some("skill:z")
        );
    }

    #[test]
    fn graph_validation_rejects_invalid_relationships() {
        let mut dependent = declaration("skill:a", "skills/a");
        dependent.dependencies.insert(id("skill:missing"));
        assert!(matches!(
            normalize(source(), [dependent]),
            Err(PluginGraphError::Graph(
                ComponentGraphError::DanglingDependency { .. }
            ))
        ));
    }

    #[test]
    fn duplicate_components_and_invalid_names_fail_before_graph_creation() {
        assert!(matches!(
            normalize(
                source(),
                [
                    declaration("skill:a", "skills/a"),
                    declaration("skill:a", "skills/other")
                ]
            ),
            Err(PluginGraphError::Graph(
                ComponentGraphError::DuplicateComponent { .. }
            ))
        ));
        let mut invalid = declaration("skill:a", "skills/a");
        invalid.declared_name = Some(" Not A Native Name".to_owned());
        assert!(matches!(
            normalize(source(), [invalid]),
            Err(PluginGraphError::InvalidDeclaredName { .. })
        ));
    }
}
