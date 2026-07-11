//! Pure cross-harness component materialization planning.

use std::collections::BTreeSet;

use crate::{
    domain::{ComponentGraph, ComponentId, ComponentRequiredness, HarnessId, Source},
    plugin_graph::{
        PluginGraphError, PluginGraphReadError, PluginGraphReader, SourceComponentGraph, normalize,
    },
};

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
}
