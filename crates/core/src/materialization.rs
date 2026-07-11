//! Pure cross-harness component materialization planning.

use std::collections::BTreeSet;

use crate::{
    domain::{ComponentGraph, ComponentId, ComponentRequiredness, HarnessId, Source},
    plugin_graph::{
        PluginGraphError, PluginGraphReadError, PluginGraphReader, SourceComponentGraph, normalize,
    },
};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ProjectionRoot {
    CanonicalAgentsSkills,
    CodexSkills,
    ClaudeSkills,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentProjection {
    pub component: ComponentId,
    pub target: HarnessId,
    pub root: ProjectionRoot,
    pub source_path: crate::domain::RelativeArtifactPath,
    pub destination: crate::domain::RelativeArtifactPath,
    pub complete_tree: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProjectionError {
    MissingProvenance {
        component: ComponentId,
    },
    InvalidSkillPath {
        component: ComponentId,
    },
    UnsupportedTarget {
        target: HarnessId,
    },
    ComponentNotFound {
        component: ComponentId,
    },
    ComponentKindMismatch {
        component: ComponentId,
    },
    UnsupportedMcp {
        component: ComponentId,
        reason: &'static str,
    },
}

impl std::fmt::Display for ProjectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingProvenance { component } => {
                write!(
                    formatter,
                    "component `{component}` has no source provenance"
                )
            }
            Self::InvalidSkillPath { component } => {
                write!(
                    formatter,
                    "skill `{component}` does not name a complete skill directory"
                )
            }
            Self::UnsupportedTarget { target } => {
                write!(
                    formatter,
                    "target `{target}` has no documented skill projection root"
                )
            }
            Self::ComponentNotFound { component } => {
                write!(
                    formatter,
                    "materialization references unknown component `{component}`"
                )
            }
            Self::ComponentKindMismatch { component } => {
                write!(formatter, "component `{component}` is not a skill")
            }
            Self::UnsupportedMcp { component, reason } => {
                write!(
                    formatter,
                    "MCP component `{component}` cannot be projected: {reason}"
                )
            }
        }
    }
}

impl std::error::Error for ProjectionError {}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum McpTransport {
    Stdio,
    Http,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct McpProjection {
    pub component: ComponentId,
    pub target: HarnessId,
    pub destination: crate::domain::RelativeArtifactPath,
    pub transport: McpTransport,
    pub credential_references: BTreeSet<String>,
}

pub trait McpProjectionMapper {
    fn map(
        &self,
        component: &crate::domain::ResourceComponent,
        provenance: &crate::plugin_graph::ComponentProvenance,
        target: &HarnessId,
    ) -> Result<McpProjection, ProjectionError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionPlan {
    pub skills: Vec<ComponentProjection>,
    pub mcp: Vec<McpProjection>,
}

/// Compose skill and MCP projections from an already classified inclusion set.
/// This function remains pure; managed publication is a downstream operation.
pub fn plan_component_projections(
    graph: &SourceComponentGraph,
    materialization: &MaterializationPlan,
    target: &HarnessId,
    mcp: &impl McpProjectionMapper,
) -> Result<ProjectionPlan, ProjectionError> {
    let skills = plan_skill_projections(graph, materialization, target)?;
    let mut mcp_projections = Vec::new();
    for component_id in &materialization.included {
        let Some(component) = graph.components().get(component_id) else {
            return Err(ProjectionError::ComponentNotFound {
                component: component_id.clone(),
            });
        };
        if component.kind != crate::domain::ComponentKind::McpServer {
            continue;
        }
        let Some(provenance) = graph.provenance(component_id) else {
            return Err(ProjectionError::MissingProvenance {
                component: component_id.clone(),
            });
        };
        mcp_projections.push(mcp.map(component, provenance, target)?);
    }
    mcp_projections.sort_by(|left, right| {
        left.component
            .cmp(&right.component)
            .then(left.destination.cmp(&right.destination))
    });
    Ok(ProjectionPlan {
        skills,
        mcp: mcp_projections,
    })
}

/// Plan complete portable skill directories for one target. Publication is a
/// later transaction and is intentionally absent from this function.
pub fn plan_skill_projections(
    graph: &SourceComponentGraph,
    materialization: &MaterializationPlan,
    target: &HarnessId,
) -> Result<Vec<ComponentProjection>, ProjectionError> {
    let mut projections = Vec::new();
    for component_id in &materialization.included {
        let Some(component) = graph.components().get(component_id) else {
            return Err(ProjectionError::ComponentNotFound {
                component: component_id.clone(),
            });
        };
        if component.kind != crate::domain::ComponentKind::Skill {
            continue;
        }
        let Some(provenance) = graph.provenance(component_id) else {
            return Err(ProjectionError::MissingProvenance {
                component: component_id.clone(),
            });
        };
        let source_path = provenance.relative_path().clone();
        let Some(name) = component_id.as_str().strip_prefix("skill:") else {
            return Err(ProjectionError::InvalidSkillPath {
                component: component_id.clone(),
            });
        };
        if name.is_empty()
            || !source_path.as_str().starts_with("skills/")
            || source_path.as_str().ends_with("/SKILL.md")
        {
            return Err(ProjectionError::InvalidSkillPath {
                component: component_id.clone(),
            });
        }
        let destination = crate::domain::RelativeArtifactPath::new(format!("skills/{name}"))
            .map_err(|_| ProjectionError::InvalidSkillPath {
                component: component_id.clone(),
            })?;
        let roots = match target.as_str() {
            "codex" => vec![ProjectionRoot::CanonicalAgentsSkills],
            "claude" => vec![
                ProjectionRoot::CanonicalAgentsSkills,
                ProjectionRoot::ClaudeSkills,
            ],
            _ => {
                return Err(ProjectionError::UnsupportedTarget {
                    target: target.clone(),
                });
            }
        };
        for root in roots {
            projections.push(ComponentProjection {
                component: component_id.clone(),
                target: target.clone(),
                root,
                source_path: source_path.clone(),
                destination: destination.clone(),
                complete_tree: true,
            });
        }
    }
    projections.sort_by(|left, right| {
        left.component
            .cmp(&right.component)
            .then(left.root.cmp(&right.root))
            .then(left.destination.cmp(&right.destination))
    });
    Ok(projections)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GraphPlanningError {
    Read(PluginGraphReadError),
    Normalize(PluginGraphError),
}

impl std::fmt::Display for GraphPlanningError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read(error) => error.fmt(formatter),
            Self::Normalize(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for GraphPlanningError {}

/// Read and normalize one explicitly selected source. This pure handoff does
/// not choose a target and performs no inventory, state, or artifact writes.
pub fn read_and_plan_graph<R: PluginGraphReader>(
    reader: &R,
    source: Source,
) -> Result<SourceComponentGraph, GraphPlanningError> {
    let declarations = reader.read(&source).map_err(GraphPlanningError::Read)?;
    normalize(source, declarations).map_err(GraphPlanningError::Normalize)
}

/// Apply the existing target-bound planner to a normalized source graph.
pub fn plan_source_materialization(
    graph: &SourceComponentGraph,
    support: &MaterializationSupport,
) -> MaterializationPlan {
    plan_materialization(graph.components(), support)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaterializationSupport {
    pub target: HarnessId,
    pub supported: BTreeSet<ComponentId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaterializationPlan {
    pub target: HarnessId,
    pub included: BTreeSet<ComponentId>,
    pub omitted_optional: BTreeSet<ComponentId>,
    pub blocked_required: BTreeSet<ComponentId>,
}

impl MaterializationPlan {
    pub fn blocked(&self) -> bool {
        !self.blocked_required.is_empty()
    }
}

/// Classify every source component for one target. Dependencies of an
/// unsupported required component remain visible and no optional omission is
/// silently promoted to faithful transfer.
pub fn plan_materialization(
    graph: &ComponentGraph,
    support: &MaterializationSupport,
) -> MaterializationPlan {
    let mut included = BTreeSet::new();
    let mut omitted_optional = BTreeSet::new();
    let mut blocked_required = BTreeSet::new();
    for (id, component) in graph.iter() {
        if support.supported.contains(id) {
            included.insert(id.clone());
        } else if component.requiredness == ComponentRequiredness::Required {
            blocked_required.insert(id.clone());
        } else {
            omitted_optional.insert(id.clone());
        }
    }
    for id in included.clone() {
        let component = graph.get(&id).expect("included component belongs to graph");
        if component
            .dependencies
            .iter()
            .any(|dependency| !included.contains(dependency))
        {
            included.remove(&id);
            match component.requiredness {
                ComponentRequiredness::Required => {
                    blocked_required.insert(id);
                }
                ComponentRequiredness::Optional => {
                    omitted_optional.insert(id);
                }
            }
        }
    }
    MaterializationPlan {
        target: support.target.clone(),
        included,
        omitted_optional,
        blocked_required,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{ComponentGraph, ComponentKind, ResourceComponent, SourceKind, SourceLocator},
        plugin_graph::ComponentDeclaration,
    };

    fn id(value: &str) -> ComponentId {
        ComponentId::new(value).unwrap()
    }

    #[test]
    fn required_components_block_and_optional_components_are_visible() {
        let graph = ComponentGraph::new([
            ResourceComponent {
                id: id("skill"),
                kind: ComponentKind::Skill,
                requiredness: ComponentRequiredness::Required,
                dependencies: BTreeSet::new(),
            },
            ResourceComponent {
                id: id("hook"),
                kind: ComponentKind::Hook,
                requiredness: ComponentRequiredness::Optional,
                dependencies: BTreeSet::new(),
            },
        ])
        .unwrap();
        let support = MaterializationSupport {
            target: HarnessId::new("codex").unwrap(),
            supported: [id("hook")].into_iter().collect(),
        };
        let plan = plan_materialization(&graph, &support);
        assert!(plan.blocked());
        assert!(plan.blocked_required.contains(&id("skill")));
        assert!(plan.included.contains(&id("hook")));
    }

    struct FixtureReader;

    impl PluginGraphReader for FixtureReader {
        fn read(
            &self,
            _source: &Source,
        ) -> Result<Vec<ComponentDeclaration>, PluginGraphReadError> {
            Ok(vec![ComponentDeclaration {
                id: id("skill"),
                kind: ComponentKind::Skill,
                requiredness: ComponentRequiredness::Required,
                dependencies: BTreeSet::new(),
                relative_path: crate::domain::RelativeArtifactPath::new("skills/skill").unwrap(),
                declared_name: Some("skill".to_owned()),
            }])
        }
    }

    #[test]
    fn graph_handoff_is_repeatable_and_target_planning_stays_pure() {
        let source = Source::new(
            SourceKind::Git,
            SourceLocator::new("https://example.test/plugin.git").unwrap(),
            None,
        )
        .unwrap();
        let graph = read_and_plan_graph(&FixtureReader, source.clone()).unwrap();
        let support = MaterializationSupport {
            target: HarnessId::new("claude").unwrap(),
            supported: [id("skill")].into_iter().collect(),
        };
        let first = plan_source_materialization(&graph, &support);
        let second = plan_source_materialization(
            &read_and_plan_graph(&FixtureReader, source).unwrap(),
            &support,
        );
        assert_eq!(first, second);
        assert!(first.included.contains(&id("skill")));
        assert!(!first.blocked());
    }

    struct FailingReader;

    impl PluginGraphReader for FailingReader {
        fn read(
            &self,
            _source: &Source,
        ) -> Result<Vec<ComponentDeclaration>, PluginGraphReadError> {
            Err(PluginGraphReadError::SourceUnavailable)
        }
    }

    #[test]
    fn reader_failure_is_returned_before_any_planning_side_effect() {
        let source = Source::new(
            SourceKind::Git,
            SourceLocator::new("https://example.test/plugin.git").unwrap(),
            None,
        )
        .unwrap();
        assert_eq!(
            read_and_plan_graph(&FailingReader, source),
            Err(GraphPlanningError::Read(
                PluginGraphReadError::SourceUnavailable
            ))
        );
    }

    #[test]
    fn skill_projection_keeps_complete_tree_and_canonical_claude_bridge() {
        let graph = normalize(
            Source::new(
                SourceKind::Git,
                SourceLocator::new("https://example.test/plugin.git").unwrap(),
                None,
            )
            .unwrap(),
            [ComponentDeclaration {
                id: id("skill:demo"),
                kind: ComponentKind::Skill,
                requiredness: ComponentRequiredness::Required,
                dependencies: BTreeSet::new(),
                relative_path: crate::domain::RelativeArtifactPath::new("skills/demo").unwrap(),
                declared_name: Some("demo".to_owned()),
            }],
        )
        .unwrap();
        let support = MaterializationSupport {
            target: HarnessId::new("claude").unwrap(),
            supported: [id("skill:demo")].into_iter().collect(),
        };
        let materialization = plan_materialization(graph.components(), &support);
        let target = HarnessId::new("claude").unwrap();
        let projections = plan_skill_projections(&graph, &materialization, &target).unwrap();
        assert_eq!(projections.len(), 2);
        assert!(
            projections
                .iter()
                .all(|projection| projection.complete_tree)
        );
        assert!(
            projections
                .iter()
                .any(|projection| projection.root == ProjectionRoot::CanonicalAgentsSkills)
        );
        assert!(
            projections
                .iter()
                .any(|projection| projection.root == ProjectionRoot::ClaudeSkills)
        );
    }

    #[test]
    fn excluded_components_do_not_reappear_in_skill_projection() {
        let graph = normalize(
            Source::new(
                SourceKind::Git,
                SourceLocator::new("https://example.test/plugin.git").unwrap(),
                None,
            )
            .unwrap(),
            [ComponentDeclaration {
                id: id("skill:demo"),
                kind: ComponentKind::Skill,
                requiredness: ComponentRequiredness::Optional,
                dependencies: BTreeSet::new(),
                relative_path: crate::domain::RelativeArtifactPath::new("skills/demo").unwrap(),
                declared_name: Some("demo".to_owned()),
            }],
        )
        .unwrap();
        let support = MaterializationSupport {
            target: HarnessId::new("codex").unwrap(),
            supported: BTreeSet::new(),
        };
        let materialization = plan_materialization(graph.components(), &support);
        let projections =
            plan_skill_projections(&graph, &materialization, &HarnessId::new("codex").unwrap())
                .unwrap();
        assert!(projections.is_empty());
    }

    struct FixtureMcpMapper;

    impl McpProjectionMapper for FixtureMcpMapper {
        fn map(
            &self,
            component: &crate::domain::ResourceComponent,
            _provenance: &crate::plugin_graph::ComponentProvenance,
            target: &HarnessId,
        ) -> Result<McpProjection, ProjectionError> {
            Ok(McpProjection {
                component: component.id.clone(),
                target: target.clone(),
                destination: crate::domain::RelativeArtifactPath::new(".mcp.json").unwrap(),
                transport: McpTransport::Http,
                credential_references: BTreeSet::new(),
            })
        }
    }

    #[test]
    fn component_projection_plan_consumes_only_included_skills_and_mcp() {
        let graph = normalize(
            Source::new(
                SourceKind::Git,
                SourceLocator::new("https://example.test/plugin.git").unwrap(),
                None,
            )
            .unwrap(),
            [
                ComponentDeclaration {
                    id: id("skill:demo"),
                    kind: ComponentKind::Skill,
                    requiredness: ComponentRequiredness::Required,
                    dependencies: BTreeSet::new(),
                    relative_path: crate::domain::RelativeArtifactPath::new("skills/demo").unwrap(),
                    declared_name: Some("demo".to_owned()),
                },
                ComponentDeclaration {
                    id: id("mcp:docs"),
                    kind: ComponentKind::McpServer,
                    requiredness: ComponentRequiredness::Optional,
                    dependencies: BTreeSet::new(),
                    relative_path: crate::domain::RelativeArtifactPath::new(".mcp.json").unwrap(),
                    declared_name: Some("docs".to_owned()),
                },
            ],
        )
        .unwrap();
        let support = MaterializationSupport {
            target: HarnessId::new("claude").unwrap(),
            supported: [id("skill:demo"), id("mcp:docs")].into_iter().collect(),
        };
        let materialization = plan_materialization(graph.components(), &support);
        let target = HarnessId::new("claude").unwrap();
        let plan = plan_component_projections(&graph, &materialization, &target, &FixtureMcpMapper)
            .unwrap();
        assert_eq!(plan.skills.len(), 2);
        assert_eq!(plan.mcp.len(), 1);
        assert_eq!(plan.mcp[0].component, id("mcp:docs"));
    }
}
