//! Pure dependency, selector, and acknowledgment handling for reconciliation plans.

use std::collections::{BTreeMap, BTreeSet};

use crate::domain::{
    AcknowledgmentRequirement, OperationClass, OperationId, OperationSelector, Plan, ResourceKey,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationGraph {
    pub plan: Plan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationWave {
    pub operations: Vec<OperationId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GraphFinding {
    Excluded {
        operation: OperationId,
    },
    DependencyExcluded {
        operation: OperationId,
        dependency: OperationId,
    },
}

#[derive(Debug)]
pub enum GraphError {
    UnknownOperation {
        operation: OperationId,
    },
    DependencyExcluded {
        operation: OperationId,
        dependency: OperationId,
    },
    DependencyCycle {
        operations: BTreeSet<OperationId>,
    },
    InvalidAcknowledgment,
    Operation(crate::domain::OperationContractError),
}

impl std::fmt::Display for GraphError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownOperation { operation } => {
                write!(formatter, "unknown operation `{operation}`")
            }
            Self::DependencyExcluded {
                operation,
                dependency,
            } => write!(
                formatter,
                "operation `{operation}` requires excluded dependency `{dependency}`"
            ),
            Self::DependencyCycle { operations } => write!(
                formatter,
                "operation dependency cycle includes {}",
                operations
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::InvalidAcknowledgment => {
                formatter.write_str("the accepted acknowledgment does not exactly match the plan")
            }
            Self::Operation(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for GraphError {}

impl From<crate::domain::OperationContractError> for GraphError {
    fn from(error: crate::domain::OperationContractError) -> Self {
        Self::Operation(error)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OperationSelection {
    pub include: BTreeSet<OperationSelector>,
    pub exclude: BTreeSet<OperationSelector>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectionResult {
    pub plan: Plan,
    pub excluded: BTreeSet<OperationId>,
    pub findings: Vec<GraphFinding>,
}

pub fn dependency_waves(plan: &Plan) -> Result<Vec<OperationWave>, GraphError> {
    let mut remaining = BTreeMap::new();
    let mut dependents: BTreeMap<OperationId, BTreeSet<OperationId>> = BTreeMap::new();
    for (id, operation) in plan.iter() {
        remaining.insert(id.clone(), operation.dependencies().len());
        for dependency in operation.dependencies() {
            if plan.get(dependency.operation_id()).is_none() {
                return Err(GraphError::UnknownOperation {
                    operation: dependency.operation_id().clone(),
                });
            }
            dependents
                .entry(dependency.operation_id().clone())
                .or_default()
                .insert(id.clone());
        }
    }
    let mut ready = remaining
        .iter()
        .filter_map(|(id, count)| (*count == 0).then_some(id.clone()))
        .collect::<BTreeSet<_>>();
    let mut waves = Vec::new();
    let mut visited = 0;
    while !ready.is_empty() {
        let current = ready.iter().cloned().collect::<Vec<_>>();
        ready.clear();
        visited += current.len();
        for id in &current {
            if let Some(children) = dependents.get(id) {
                for child in children {
                    let count = remaining
                        .get_mut(child)
                        .expect("validated operation dependency belongs to the plan");
                    *count -= 1;
                    if *count == 0 {
                        ready.insert(child.clone());
                    }
                }
            }
        }
        waves.push(OperationWave {
            operations: current,
        });
    }
    if visited != plan.iter().len() {
        return Err(GraphError::DependencyCycle {
            operations: remaining
                .into_iter()
                .filter_map(|(id, count)| (count > 0).then_some(id))
                .collect(),
        });
    }
    Ok(waves)
}

pub fn dependency_closure(
    plan: &Plan,
    selected: &BTreeSet<OperationId>,
) -> Result<BTreeSet<OperationId>, GraphError> {
    let mut closure = BTreeSet::new();
    let mut pending = selected.iter().cloned().collect::<Vec<_>>();
    while let Some(id) = pending.pop() {
        if !closure.insert(id.clone()) {
            continue;
        }
        let operation = plan.get(&id).ok_or_else(|| GraphError::UnknownOperation {
            operation: id.clone(),
        })?;
        pending.extend(
            operation
                .dependencies()
                .iter()
                .map(|dependency| dependency.operation_id().clone()),
        );
    }
    Ok(closure)
}

pub fn select_operations(
    plan: &Plan,
    selection: &OperationSelection,
) -> Result<SelectionResult, GraphError> {
    let mut selected = BTreeSet::new();
    let mut excluded = BTreeSet::new();
    let mut findings = Vec::new();
    for (id, operation) in plan.iter() {
        let included = selection.include.is_empty()
            || selection
                .include
                .iter()
                .any(|selector| selector_matches(selector, operation.selector()));
        let is_excluded = selection
            .exclude
            .iter()
            .any(|selector| selector_matches(selector, operation.selector()));
        if included && !is_excluded {
            selected.insert(id.clone());
        } else if is_excluded {
            excluded.insert(id.clone());
            findings.push(GraphFinding::Excluded {
                operation: id.clone(),
            });
        }
    }
    let closure = dependency_closure(plan, &selected)?;
    for id in &selected {
        let operation = plan
            .get(id)
            .expect("selected operation belongs to the validated plan");
        for dependency in operation.dependencies() {
            if excluded.contains(dependency.operation_id()) {
                return Err(GraphError::DependencyExcluded {
                    operation: id.clone(),
                    dependency: dependency.operation_id().clone(),
                });
            }
        }
    }
    let operations = closure
        .iter()
        .map(|id| {
            plan.get(id)
                .cloned()
                .ok_or_else(|| GraphError::UnknownOperation {
                    operation: id.clone(),
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SelectionResult {
        plan: Plan::new(operations)?,
        excluded,
        findings,
    })
}

pub fn validate_acknowledgment(
    plan: &Plan,
    accepted: &AcknowledgmentRequirement,
) -> Result<(), GraphError> {
    let Some(_) = accepted.selectors() else {
        return Err(GraphError::InvalidAcknowledgment);
    };
    for (_, operation) in plan
        .iter()
        .filter(|(_, operation)| operation.class() == OperationClass::Partial)
    {
        if operation.acknowledgment() != accepted {
            return Err(GraphError::InvalidAcknowledgment);
        }
    }
    Ok(())
}

fn selector_matches(selected: &OperationSelector, operation: &OperationSelector) -> bool {
    match (selected, operation) {
        (
            OperationSelector::Resource { resource },
            OperationSelector::Resource {
                resource: candidate,
            }
            | OperationSelector::Component {
                resource: candidate,
                ..
            },
        ) => resource == candidate,
        (
            OperationSelector::Component {
                resource,
                component_id,
            },
            OperationSelector::Component {
                resource: candidate,
                component_id: candidate_component,
            },
        ) => resource == candidate && component_id == candidate_component,
        (OperationSelector::Component { .. }, OperationSelector::Resource { .. }) => false,
    }
}

#[allow(dead_code)]
fn _resource_scope(resource: &ResourceKey) -> &crate::domain::Scope {
    resource.scope()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_plan_has_no_waves_or_selection_findings() {
        let plan = Plan::new([]).unwrap();
        assert!(dependency_waves(&plan).unwrap().is_empty());
        let selected = select_operations(&plan, &OperationSelection::default()).unwrap();
        assert!(selected.plan.is_empty());
        assert!(selected.findings.is_empty());
    }

    #[test]
    fn dependency_closure_rejects_unknown_operation() {
        let plan = Plan::new([]).unwrap();
        let id = OperationId::new("missing").unwrap();
        assert!(matches!(
            dependency_closure(&plan, &BTreeSet::from([id.clone()])),
            Err(GraphError::UnknownOperation { operation }) if operation == id
        ));
    }
}
