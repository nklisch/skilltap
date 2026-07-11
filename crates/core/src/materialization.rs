//! Pure cross-harness component materialization planning.

use std::collections::BTreeSet;

use crate::domain::{ComponentGraph, ComponentId, ComponentRequiredness, HarnessId};

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
    use crate::domain::{ComponentGraph, ComponentKind, ResourceComponent};

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
}
