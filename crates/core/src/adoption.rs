//! Pure adoption planning over ephemeral normalized observations.

use std::collections::{BTreeSet, HashSet};

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
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdoptionUnadoptableCode {
    DeclaredOnly,
    UnsupportedObservation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdoptionDecision {
    Adopted(Box<AdoptionCandidate>),
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
}

impl std::fmt::Display for AdoptionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidCandidate => "an observed resource could not become a desired resource",
            Self::DuplicateCandidate => "duplicate adoption candidates were observed",
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
    let mut decisions = Vec::new();
    let mut additions = Vec::new();

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
            match existing.and_then(|resources| resources.get(&key)) {
                Some(value) if value == &desired => {
                    decisions.push(AdoptionDecision::AlreadyManaged { key });
                }
                Some(_) => decisions.push(AdoptionDecision::Conflict {
                    key,
                    code: AdoptionConflictCode::ExistingDifferentResource,
                }),
                None => {
                    additions.push(desired);
                    decisions.push(AdoptionDecision::Adopted(Box::new(candidate)));
                }
            }
        }
    }
    Ok(AdoptionPlan {
        decisions,
        additions,
    })
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
        let harness = HarnessId::new("codex").unwrap();
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
}
