//! Pure adoption planning over ephemeral normalized observations.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use crate::{
    domain::{
        ComponentChoice, DesiredOrigin, DesiredResource, Fingerprint, HarnessId, HarnessSet,
        ObservationKey, ObservationLayer, ObservationTarget, ObservedEnvironment, ObservedResource,
        ResourceKey, Scope, UpdateIntent,
    },
    storage::InventoryDocument,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdoptionSelection {
    targets: BTreeSet<ObservationTarget>,
}

impl AdoptionSelection {
    pub fn new(targets: impl IntoIterator<Item = ObservationTarget>) -> Self {
        Self {
            targets: targets.into_iter().collect(),
        }
    }

    pub fn contains(&self, target: &ObservationTarget) -> bool {
        self.targets.is_empty() || self.targets.contains(target)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdoptionIdentity {
    pub target: ObservationTarget,
    pub observation: ObservationKey,
    pub native_identity: crate::domain::NativeId,
    pub fingerprint: Option<Fingerprint>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdoptionCandidate {
    pub desired: DesiredResource,
    pub identity: AdoptionIdentity,
    pub source_harnesses: HarnessSet,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdoptionConflictCode {
    ExistingDifferentResource,
    CandidateSemanticsDiffer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdoptionUnadoptableCode {
    DeclaredOnly,
    UnsupportedObservation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdoptionDecision {
    Adopted(Box<AdoptionCandidate>),
    Coalesced(Box<AdoptionCandidate>),
    AlreadyManaged {
        key: ResourceKey,
    },
    Conflict {
        key: ResourceKey,
        code: AdoptionConflictCode,
    },
    Unadoptable {
        key: ResourceKey,
        code: AdoptionUnadoptableCode,
    },
    Unchanged {
        key: ResourceKey,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdoptionPlan {
    pub decisions: Vec<AdoptionDecision>,
    pub additions: Vec<DesiredResource>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdoptionError {
    InvalidCandidate,
    DuplicateCandidate,
    ConflictingInventory,
}

impl std::fmt::Display for AdoptionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidCandidate => "an observed resource could not become a desired resource",
            Self::DuplicateCandidate => "duplicate adoption candidates were observed",
            Self::ConflictingInventory => "adoption additions contain conflicting resources",
        })
    }
}

impl std::error::Error for AdoptionError {}

/// Plans adoption without reading or writing any external state.
pub fn plan_adoption(
    inventory: Option<&InventoryDocument>,
    environment: &ObservedEnvironment,
    selection: &AdoptionSelection,
) -> Result<AdoptionPlan, AdoptionError> {
    let existing = inventory.map(InventoryDocument::resources);
    let mut seen = HashSet::new();
    let mut candidates = BTreeMap::<ResourceKey, Vec<AdoptionCandidate>>::new();

    for (target, outcome) in environment.iter() {
        if !selection.contains(target) {
            continue;
        }
        let crate::domain::HarnessObservationOutcome::Observed { observation } = outcome else {
            continue;
        };
        for resource in observation.resources().values() {
            if resource.key().layer() != ObservationLayer::Effective {
                continue;
            }
            if !seen.insert((target.clone(), resource.key().clone())) {
                return Err(AdoptionError::DuplicateCandidate);
            }
            let desired = desired_from_observed(resource, target.harness())?;
            let key = desired.key().clone();
            let identity = AdoptionIdentity {
                target: target.clone(),
                observation: resource.key().clone(),
                native_identity: resource.native_identity().clone(),
                fingerprint: resource.fingerprint().cloned(),
            };
            let candidate = AdoptionCandidate {
                desired: desired.clone(),
                identity,
                source_harnesses: HarnessSet::new([target.harness().clone()])
                    .map_err(|_| AdoptionError::InvalidCandidate)?,
            };
            candidates.entry(key).or_default().push(candidate);
        }
    }

    let mut decisions = Vec::new();
    let mut additions = Vec::new();
    for (key, candidates) in candidates {
        let Some(first) = candidates.first() else {
            continue;
        };
        if candidates
            .iter()
            .skip(1)
            .any(|candidate| !equivalent_candidates(first, candidate))
        {
            decisions.extend(candidates.into_iter().map(|_| AdoptionDecision::Conflict {
                key: key.clone(),
                code: AdoptionConflictCode::CandidateSemanticsDiffer,
            }));
            continue;
        }

        let coalesced = coalesce_candidates(candidates)?;
        if let Some(value) = existing.and_then(|resources| resources.get(&key)) {
            if desired_semantically_equivalent(value, &coalesced.desired) {
                decisions.push(AdoptionDecision::AlreadyManaged { key });
            } else {
                decisions.push(AdoptionDecision::Conflict {
                    key,
                    code: AdoptionConflictCode::ExistingDifferentResource,
                });
            }
            continue;
        }

        let decision = if coalesced.source_harnesses.iter().count() > 1 {
            AdoptionDecision::Coalesced(Box::new(coalesced.clone()))
        } else {
            AdoptionDecision::Adopted(Box::new(coalesced.clone()))
        };
        additions.push(coalesced.desired);
        decisions.push(decision);
    }
    Ok(AdoptionPlan {
        decisions,
        additions,
    })
}

/// Returns whether two fresh observations describe the same logical resource.
///
/// Native identities and fingerprints intentionally do not participate: they are
/// evidence for the locked revalidation step, not proof of semantic equivalence.
pub fn equivalent_candidates(left: &AdoptionCandidate, right: &AdoptionCandidate) -> bool {
    left.desired.key() == right.desired.key()
        && left.desired.kind() == right.desired.kind()
        && left.desired.source() == right.desired.source()
        && left.desired.components() == right.desired.components()
        && left.desired.dependencies() == right.desired.dependencies()
}

/// Merge planned additions into an inventory while preserving existing policy.
///
/// Existing entries are never rewritten, even when an equivalent addition has a
/// different target set or adopted origin. Equivalent new additions are merged
/// into one target set; a semantic disagreement at one key aborts the merge.
pub fn merge_inventory(
    inventory: &InventoryDocument,
    additions: impl IntoIterator<Item = DesiredResource>,
) -> Result<InventoryDocument, AdoptionError> {
    let mut projects = inventory.projects().clone();
    let mut resources = inventory.resources().clone();
    let mut pending = BTreeMap::new();

    for addition in additions {
        if let Scope::Project(path) = addition.scope() {
            projects.insert(path.clone());
        }
        let key = addition.key().clone();
        if let Some(existing) = resources.get(&key) {
            if !desired_semantically_equivalent(existing, &addition) {
                return Err(AdoptionError::ConflictingInventory);
            }
            continue;
        }
        if let Some(existing) = pending.get(&key) {
            if !desired_semantically_equivalent(existing, &addition) {
                return Err(AdoptionError::ConflictingInventory);
            }
            let merged = merge_desired_additions(existing, &addition)?;
            pending.insert(key, merged);
        } else {
            pending.insert(key, addition);
        }
    }

    // Inventory entries already under management are intentionally immutable;
    // only fresh additions can be coalesced across harnesses.
    for (key, addition) in pending {
        if let Some(existing) = resources.get_mut(&key) {
            if !desired_semantically_equivalent(existing, &addition) {
                return Err(AdoptionError::ConflictingInventory);
            }
            continue;
        }
        resources.insert(key, addition);
    }

    InventoryDocument::new(inventory.schema(), projects, resources.into_values())
        .map_err(|_| AdoptionError::InvalidCandidate)
}

fn coalesce_candidates(
    candidates: Vec<AdoptionCandidate>,
) -> Result<AdoptionCandidate, AdoptionError> {
    let mut iter = candidates.into_iter();
    let mut merged = iter.next().ok_or(AdoptionError::InvalidCandidate)?;
    for candidate in iter {
        let mut harnesses = merged
            .source_harnesses
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        harnesses.extend(candidate.source_harnesses.iter().cloned());
        let source_harness = harnesses
            .iter()
            .next()
            .cloned()
            .ok_or(AdoptionError::InvalidCandidate)?;
        merged.source_harnesses =
            HarnessSet::new(harnesses).map_err(|_| AdoptionError::InvalidCandidate)?;
        merged.desired = rebuild_desired(
            &merged.desired,
            merged.source_harnesses.clone(),
            DesiredOrigin::Adopted(source_harness),
        )?;
    }
    Ok(merged)
}

fn desired_semantically_equivalent(left: &DesiredResource, right: &DesiredResource) -> bool {
    left.key() == right.key()
        && left.kind() == right.kind()
        && left.source() == right.source()
        && left.components() == right.components()
        && left.dependencies() == right.dependencies()
}

fn merge_desired_additions(
    left: &DesiredResource,
    right: &DesiredResource,
) -> Result<DesiredResource, AdoptionError> {
    let mut targets = left.targets().iter().cloned().collect::<BTreeSet<_>>();
    targets.extend(right.targets().iter().cloned());
    let origin = match (left.origin(), right.origin()) {
        (DesiredOrigin::Adopted(left), DesiredOrigin::Adopted(right)) => {
            DesiredOrigin::Adopted(left.min(right).clone())
        }
        (origin, _) => origin.clone(),
    };
    rebuild_desired(
        left,
        HarnessSet::new(targets).map_err(|_| AdoptionError::InvalidCandidate)?,
        origin,
    )
}

fn rebuild_desired(
    resource: &DesiredResource,
    targets: HarnessSet,
    origin: DesiredOrigin,
) -> Result<DesiredResource, AdoptionError> {
    DesiredResource::new(
        resource.key().clone(),
        resource.kind(),
        targets,
        origin,
        resource.source().cloned(),
        resource.update(),
        resource.components().clone(),
        resource.component_choices().clone(),
        resource.accepted_consequences().clone(),
        resource.dependencies().clone(),
    )
    .map_err(|_| AdoptionError::InvalidCandidate)
}

fn desired_from_observed(
    resource: &ObservedResource,
    harness: &HarnessId,
) -> Result<DesiredResource, AdoptionError> {
    let component_choices = resource
        .components()
        .iter()
        .map(|(id, _)| (id.clone(), ComponentChoice::Default))
        .collect();
    let dependencies = resource
        .dependencies()
        .iter()
        .filter_map(|dependency| match dependency {
            crate::domain::ObservedDependency::Resolved { resource } => Some(resource.clone()),
            crate::domain::ObservedDependency::Unresolved { .. } => None,
        })
        .collect();
    DesiredResource::new(
        ResourceKey::new(
            resource.key().resource().id().clone(),
            scope_clone(resource.scope()),
        ),
        resource.kind(),
        HarnessSet::new([harness.clone()]).map_err(|_| AdoptionError::InvalidCandidate)?,
        DesiredOrigin::Adopted(harness.clone()),
        resource.source().cloned(),
        UpdateIntent::Track,
        resource.components().clone(),
        component_choices,
        Default::default(),
        dependencies,
    )
    .map_err(|_| AdoptionError::InvalidCandidate)
}

fn scope_clone(scope: &Scope) -> Scope {
    scope.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        AbsolutePath, CapabilityProfileId, CapabilityProfileSelection, CapabilitySet,
        ConfiguredBinary, ExecutableFileIdentity, ExecutableIdentity, HarnessInstallation,
        HarnessReachability, NativeVersion, ObservationBatch, ObservationEvidence,
        ObservationRequest, ObservedDependency, ResourceHealth, ResourceId, ResourceKind,
        ScopedCapabilitySets,
    };

    fn environment() -> ObservedEnvironment {
        environment_with_harness("codex")
    }

    fn environment_with_harness(name: &str) -> ObservedEnvironment {
        let harness = HarnessId::new(name).unwrap();
        let installation = HarnessInstallation::new(
            harness.clone(),
            ConfiguredBinary::absolute(AbsolutePath::new("/opt/codex").unwrap()),
            HarnessReachability::Reachable {
                executable: ExecutableIdentity::new(
                    AbsolutePath::new("/opt/codex").unwrap(),
                    ExecutableFileIdentity::new(1, 2),
                ),
                native_version: NativeVersion::new("3.0.0").unwrap(),
            },
        );
        let profile = CapabilityProfileSelection::verified(
            CapabilityProfileId::new("codex-v3").unwrap(),
            ScopedCapabilitySets::new(CapabilitySet::default(), CapabilitySet::default()),
        );
        let evidence = ObservationEvidence::new(&installation, profile).unwrap();
        let request = ObservationRequest::new(Scope::Global, evidence);
        let resource = ObservedResource::new(
            ObservationKey::new(
                ResourceKey::new(ResourceId::new("plugin:demo").unwrap(), Scope::Global),
                harness,
                ObservationLayer::Effective,
            ),
            ResourceKind::Plugin,
            crate::domain::Provenance::Native,
            crate::domain::Ownership::Unmanaged,
            ResourceHealth::Healthy,
            None,
            crate::domain::ComponentGraph::default(),
            [ObservedDependency::Unresolved {
                native_identity: crate::domain::NativeId::new("native-missing").unwrap(),
            }]
            .into_iter()
            .collect(),
            crate::domain::NativeId::new("demo@native").unwrap(),
            None,
            None,
        );
        let observation =
            crate::domain::HarnessObservation::new(request.clone(), [resource], []).unwrap();
        let batch = ObservationBatch::new([request]).unwrap();
        crate::domain::ObservedEnvironment::new(
            batch,
            [crate::domain::HarnessObservationOutcome::observed(
                observation,
            )],
        )
        .unwrap()
    }

    #[test]
    fn effective_resource_becomes_adopted_candidate_without_io() {
        let plan = plan_adoption(None, &environment(), &AdoptionSelection::new([])).unwrap();
        assert_eq!(plan.additions.len(), 1);
        assert!(matches!(plan.decisions[0], AdoptionDecision::Adopted(_)));
        assert!(matches!(
            plan.additions[0].origin(),
            DesiredOrigin::Adopted(harness) if harness.as_str() == "codex"
        ));
    }

    #[test]
    fn equivalent_existing_inventory_is_already_managed() {
        let first = plan_adoption(None, &environment(), &AdoptionSelection::new([]))
            .unwrap()
            .additions
            .remove(0);
        let inventory = InventoryDocument::new(1, [], [first]).unwrap();
        let plan = plan_adoption(
            Some(&inventory),
            &environment(),
            &AdoptionSelection::new([]),
        )
        .unwrap();
        assert!(matches!(
            plan.decisions[0],
            AdoptionDecision::AlreadyManaged { .. }
        ));
        assert!(plan.additions.is_empty());
    }

    #[test]
    fn equivalent_candidates_ignore_native_identity_and_fingerprint() {
        let plan = plan_adoption(None, &environment(), &AdoptionSelection::new([])).unwrap();
        let AdoptionDecision::Adopted(candidate) = &plan.decisions[0] else {
            panic!("expected an adopted candidate");
        };
        let mut other = (**candidate).clone();
        other.identity.native_identity = crate::domain::NativeId::new("different").unwrap();
        assert!(equivalent_candidates(candidate, &other));
    }

    #[test]
    fn equivalent_candidates_coalesce_targets_and_use_stable_origin() {
        let plan = plan_adoption(None, &environment(), &AdoptionSelection::new([])).unwrap();
        let AdoptionDecision::Adopted(first) = &plan.decisions[0] else {
            panic!("expected an adopted candidate");
        };
        let mut second = (**first).clone();
        let claude = HarnessId::new("claude").unwrap();
        second.source_harnesses = HarnessSet::new([claude.clone()]).unwrap();
        let merged = coalesce_candidates(vec![(**first).clone(), second]).unwrap();
        assert_eq!(merged.source_harnesses.iter().count(), 2);
        assert!(merged.desired.targets().contains(&claude));
        assert!(matches!(
            merged.desired.origin(),
            DesiredOrigin::Adopted(harness) if harness.as_str() == "claude"
        ));
    }

    #[test]
    fn merge_inventory_preserves_existing_entries_and_records_project_scope() {
        let first = plan_adoption(None, &environment(), &AdoptionSelection::new([]))
            .unwrap()
            .additions
            .remove(0);
        let inventory = InventoryDocument::new(1, [], []).unwrap();
        let merged = merge_inventory(&inventory, [first.clone()]).unwrap();
        assert_eq!(merged.resources().len(), 1);
        assert!(merged.projects().is_empty());
        let unchanged = merge_inventory(&merged, [first]).unwrap();
        assert_eq!(merged, unchanged);
    }

    #[test]
    fn merge_inventory_rejects_conflicting_same_key_additions() {
        let first = plan_adoption(None, &environment(), &AdoptionSelection::new([]))
            .unwrap()
            .additions
            .remove(0);
        let different = DesiredResource::new(
            first.key().clone(),
            ResourceKind::StandaloneSkill,
            first.targets().clone(),
            DesiredOrigin::Direct,
            first.source().cloned(),
            first.update(),
            first.components().clone(),
            first.component_choices().clone(),
            first.accepted_consequences().clone(),
            first.dependencies().clone(),
        )
        .unwrap();
        let inventory = InventoryDocument::new(1, [], []).unwrap();
        assert_eq!(
            merge_inventory(&inventory, [first, different]).unwrap_err(),
            AdoptionError::ConflictingInventory
        );
    }
}
