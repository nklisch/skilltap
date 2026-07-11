//! Pure foreground update planning shared by CLI and daemon entry points.

use std::collections::BTreeSet;

use crate::{
    domain::{DesiredResource, OperationSelector, ResourceKey},
    storage::UpdateMode,
    updates::{UpdateCandidate, UpdateDecision, UpdateSafety, classify_update_with_mode},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForegroundUpdateEntry {
    resource: ResourceKey,
    current_revision: Option<crate::domain::ResolvedRevision>,
    available_revision: Option<crate::domain::ResolvedRevision>,
    decision: UpdateDecision,
    acknowledgment_selectors: BTreeSet<OperationSelector>,
}

impl ForegroundUpdateEntry {
    pub fn resource(&self) -> &ResourceKey {
        &self.resource
    }

    pub fn current_revision(&self) -> Option<&crate::domain::ResolvedRevision> {
        self.current_revision.as_ref()
    }

    pub fn available_revision(&self) -> Option<&crate::domain::ResolvedRevision> {
        self.available_revision.as_ref()
    }

    pub const fn decision(&self) -> UpdateDecision {
        self.decision
    }

    pub const fn is_safe(&self) -> bool {
        matches!(self.decision.safety, UpdateSafety::Safe)
    }

    pub fn acknowledgment_selectors(&self) -> &BTreeSet<OperationSelector> {
        &self.acknowledgment_selectors
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForegroundUpdatePlan {
    entries: Vec<ForegroundUpdateEntry>,
}

impl ForegroundUpdatePlan {
    pub fn entries(&self) -> &[ForegroundUpdateEntry] {
        &self.entries
    }

    pub fn safe_entries(&self) -> impl Iterator<Item = &ForegroundUpdateEntry> {
        self.entries.iter().filter(|entry| entry.is_safe())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ForegroundUpdatePlanError {
    DuplicateResource { resource: ResourceKey },
    MissingCandidate { resource: ResourceKey },
    DuplicateCandidate { resource: ResourceKey },
    UnexpectedCandidate { resource: ResourceKey },
}

impl std::fmt::Display for ForegroundUpdatePlanError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateResource { resource } => {
                write!(
                    formatter,
                    "foreground update resources contain `{resource}` twice"
                )
            }
            Self::MissingCandidate { resource } => {
                write!(
                    formatter,
                    "foreground update has no candidate for `{resource}`"
                )
            }
            Self::DuplicateCandidate { resource } => {
                write!(
                    formatter,
                    "foreground update has multiple candidates for `{resource}`"
                )
            }
            Self::UnexpectedCandidate { resource } => {
                write!(
                    formatter,
                    "foreground update candidate `{resource}` is not requested"
                )
            }
        }
    }
}

impl std::error::Error for ForegroundUpdatePlanError {}

pub struct ForegroundUpdateRequest<'a> {
    pub resources: &'a [DesiredResource],
    pub candidates: &'a [UpdateCandidate],
    pub mode: UpdateMode,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForegroundUpdateSelection {
    entries: Vec<ForegroundUpdateEntry>,
}

impl ForegroundUpdateSelection {
    pub fn entries(&self) -> &[ForegroundUpdateEntry] {
        &self.entries
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ForegroundUpdateSelectionError {
    Blocked {
        resource: ResourceKey,
    },
    DecisionRequired {
        resource: ResourceKey,
    },
    MissingAcknowledgment {
        resource: ResourceKey,
        selectors: BTreeSet<OperationSelector>,
    },
    UnexpectedAcknowledgment {
        selector: OperationSelector,
    },
}

impl std::fmt::Display for ForegroundUpdateSelectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Blocked { resource } => {
                write!(formatter, "foreground update `{resource}` is blocked")
            }
            Self::DecisionRequired { resource } => {
                write!(
                    formatter,
                    "foreground update `{resource}` needs a user decision"
                )
            }
            Self::MissingAcknowledgment { resource, .. } => write!(
                formatter,
                "foreground update `{resource}` is missing exact consequence acknowledgment"
            ),
            Self::UnexpectedAcknowledgment { selector } => write!(
                formatter,
                "acknowledgment selector `{selector:?}` does not belong to the update plan"
            ),
        }
    }
}

impl std::error::Error for ForegroundUpdateSelectionError {}

/// Pair exact desired resources with resolved candidates and classify each
/// candidate without performing any mutation or state I/O.
pub fn plan_foreground_updates(
    request: ForegroundUpdateRequest<'_>,
) -> Result<ForegroundUpdatePlan, ForegroundUpdatePlanError> {
    let mut requested = BTreeSet::new();
    let mut entries = Vec::with_capacity(request.resources.len());
    for resource in request.resources {
        let key = resource.key().clone();
        if !requested.insert(key.clone()) {
            return Err(ForegroundUpdatePlanError::DuplicateResource { resource: key });
        }
        let mut matches = request
            .candidates
            .iter()
            .filter(|candidate| candidate.resource == key);
        let Some(candidate) = matches.next() else {
            return Err(ForegroundUpdatePlanError::MissingCandidate { resource: key });
        };
        if matches.next().is_some() {
            return Err(ForegroundUpdatePlanError::DuplicateCandidate { resource: key });
        }
        entries.push(ForegroundUpdateEntry {
            resource: key,
            current_revision: candidate.current_revision.clone(),
            available_revision: candidate.available_revision.clone(),
            decision: classify_update_with_mode(candidate, request.mode),
            acknowledgment_selectors: candidate.acknowledgment_selectors.clone(),
        });
    }
    if let Some(candidate) = request
        .candidates
        .iter()
        .find(|candidate| !requested.contains(&candidate.resource))
    {
        return Err(ForegroundUpdatePlanError::UnexpectedCandidate {
            resource: candidate.resource.clone(),
        });
    }
    entries.sort_by(|left, right| left.resource.cmp(&right.resource));
    Ok(ForegroundUpdatePlan { entries })
}

/// Select safe and explicitly acknowledged entries. Exact selector equality
/// is required; there is no generic bypass for a partial consequence.
pub fn select_foreground_updates(
    plan: &ForegroundUpdatePlan,
    acknowledgments: &BTreeSet<OperationSelector>,
) -> Result<ForegroundUpdateSelection, ForegroundUpdateSelectionError> {
    let expected = plan
        .entries
        .iter()
        .flat_map(|entry| entry.acknowledgment_selectors.iter().cloned())
        .collect::<BTreeSet<_>>();
    if let Some(selector) = acknowledgments.difference(&expected).next() {
        return Err(ForegroundUpdateSelectionError::UnexpectedAcknowledgment {
            selector: selector.clone(),
        });
    }
    let mut selected = Vec::new();
    for entry in &plan.entries {
        match entry.decision.safety {
            UpdateSafety::NoUpdate => {}
            UpdateSafety::Safe => selected.push(entry.clone()),
            UpdateSafety::Blocked => {
                return Err(ForegroundUpdateSelectionError::Blocked {
                    resource: entry.resource.clone(),
                });
            }
            UpdateSafety::NeedsDecision => {
                if entry.acknowledgment_selectors.is_empty() {
                    return Err(ForegroundUpdateSelectionError::DecisionRequired {
                        resource: entry.resource.clone(),
                    });
                }
                if !entry.acknowledgment_selectors.is_subset(acknowledgments) {
                    return Err(ForegroundUpdateSelectionError::MissingAcknowledgment {
                        resource: entry.resource.clone(),
                        selectors: entry.acknowledgment_selectors.clone(),
                    });
                }
                selected.push(entry.clone());
            }
        }
    }
    Ok(ForegroundUpdateSelection { entries: selected })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    use crate::domain::{
        ComponentGraph, DesiredOrigin, GitCommit, HarnessId, HarnessSet, ResourceId, ResourceKind,
        Scope, UpdateIntent,
    };

    fn resource(name: &str) -> DesiredResource {
        let key = ResourceKey::new(
            ResourceId::new(format!("skill:{name}")).unwrap(),
            Scope::Global,
        );
        DesiredResource::new(
            key,
            ResourceKind::StandaloneSkill,
            HarnessSet::new([HarnessId::new("codex").unwrap()]).unwrap(),
            DesiredOrigin::Direct,
            None,
            UpdateIntent::Track,
            ComponentGraph::new([]).unwrap(),
            BTreeMap::new(),
            BTreeMap::new(),
            BTreeSet::new(),
        )
        .unwrap()
    }

    fn revision(value: char) -> crate::domain::ResolvedRevision {
        crate::domain::ResolvedRevision::GitCommit(
            GitCommit::new(value.to_string().repeat(40)).unwrap(),
        )
    }

    fn candidate(resource: &DesiredResource, current: char, available: char) -> UpdateCandidate {
        UpdateCandidate {
            resource: resource.key().clone(),
            current_revision: Some(revision(current)),
            available_revision: Some(revision(available)),
            resolution_error: None,
            pinned: false,
            drifted: false,
            compatibility_changed: false,
            requires_acknowledgment: false,
            intent: UpdateIntent::Track,
            acknowledgment_selectors: BTreeSet::new(),
        }
    }

    #[test]
    fn clean_candidates_produce_deterministic_safe_entries() {
        let first = resource("zeta");
        let second = resource("alpha");
        let candidates = [candidate(&first, 'a', 'b'), candidate(&second, 'a', 'c')];
        let plan = plan_foreground_updates(ForegroundUpdateRequest {
            resources: &[first, second],
            candidates: &candidates,
            mode: UpdateMode::ApplySafe,
        })
        .unwrap();
        assert_eq!(plan.entries().len(), 2);
        assert_eq!(plan.entries()[0].resource().id().as_str(), "skill:alpha");
        assert_eq!(plan.safe_entries().count(), 2);
    }

    #[test]
    fn candidate_pairing_fails_closed_for_missing_duplicate_and_unexpected() {
        let first = resource("alpha");
        let second = resource("beta");
        let missing = plan_foreground_updates(ForegroundUpdateRequest {
            resources: &[first.clone(), second.clone()],
            candidates: &[candidate(&first, 'a', 'b')],
            mode: UpdateMode::ApplySafe,
        });
        assert!(matches!(
            missing,
            Err(ForegroundUpdatePlanError::MissingCandidate { .. })
        ));
        let duplicate = plan_foreground_updates(ForegroundUpdateRequest {
            resources: std::slice::from_ref(&first),
            candidates: &[candidate(&first, 'a', 'b'), candidate(&first, 'a', 'c')],
            mode: UpdateMode::ApplySafe,
        });
        assert!(matches!(
            duplicate,
            Err(ForegroundUpdatePlanError::DuplicateCandidate { .. })
        ));
        let unexpected = plan_foreground_updates(ForegroundUpdateRequest {
            resources: std::slice::from_ref(&first),
            candidates: &[candidate(&first, 'a', 'b'), candidate(&second, 'a', 'c')],
            mode: UpdateMode::ApplySafe,
        });
        assert!(matches!(
            unexpected,
            Err(ForegroundUpdatePlanError::UnexpectedCandidate { .. })
        ));
    }

    #[test]
    fn partial_selection_requires_exact_acknowledgment_selectors() {
        let selected = resource("alpha");
        let selector = OperationSelector::Resource {
            resource: selected.key().clone(),
        };
        let mut partial = candidate(&selected, 'a', 'b');
        partial.requires_acknowledgment = true;
        partial.acknowledgment_selectors = [selector.clone()].into_iter().collect();
        let plan = plan_foreground_updates(ForegroundUpdateRequest {
            resources: std::slice::from_ref(&selected),
            candidates: &[partial],
            mode: UpdateMode::ApplySafe,
        })
        .unwrap();
        assert!(matches!(
            select_foreground_updates(&plan, &BTreeSet::new()),
            Err(ForegroundUpdateSelectionError::MissingAcknowledgment { .. })
        ));
        let acknowledgments = [selector.clone()].into_iter().collect();
        let selection = select_foreground_updates(&plan, &acknowledgments).unwrap();
        assert_eq!(selection.entries().len(), 1);
        let unexpected = [OperationSelector::Resource {
            resource: ResourceKey::new(ResourceId::new("skill:other").unwrap(), Scope::Global),
        }]
        .into_iter()
        .collect();
        assert!(matches!(
            select_foreground_updates(&plan, &unexpected),
            Err(ForegroundUpdateSelectionError::UnexpectedAcknowledgment { .. })
        ));
    }
}
