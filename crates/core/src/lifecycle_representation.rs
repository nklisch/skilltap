//! Pure native-versus-managed lifecycle representation selection.
//!
//! Representation is deliberately derived from target-local evidence instead
//! of target identity or a persisted route flag. Existing applied evidence is
//! authoritative; only a fresh marketplace compares candidate component plans.

use crate::{
    domain::{Ownership, Provenance},
    materialization::MaterializationPlan,
    storage::TargetResourceState,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LifecycleRepresentation {
    Native,
    Managed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepresentationCandidate {
    pub representation: LifecycleRepresentation,
    pub plan: MaterializationPlan,
}

/// The fresh form intentionally carries complete candidate plans by value so
/// selection remains a pure, allocation-free operation at the call boundary.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RepresentationEvidence {
    Existing(LifecycleRepresentation),
    Marketplace(LifecycleRepresentation),
    Fresh {
        native: Option<RepresentationCandidate>,
        managed: Option<RepresentationCandidate>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LifecycleRepresentationError {
    ContradictoryAppliedState,
    MissingMarketplaceRepresentation,
    RequiredComponentsBlocked,
    IncomparablePartialRepresentations,
    NoSupportedRepresentation,
}

/// Select a representation from already-normalized lifecycle evidence.
///
/// A candidate with required omissions is not eligible. A faithful native
/// candidate wins over every managed candidate. When both candidates are
/// partial, the strict component superset wins; equal partial plans retain the
/// native preference, while incomparable plans fail closed.
pub fn select_lifecycle_representation(
    evidence: RepresentationEvidence,
) -> Result<LifecycleRepresentation, LifecycleRepresentationError> {
    match evidence {
        RepresentationEvidence::Existing(representation)
        | RepresentationEvidence::Marketplace(representation) => Ok(representation),
        RepresentationEvidence::Fresh { native, managed } => {
            select_fresh_representation(native, managed)
        }
    }
}

fn select_fresh_representation(
    native: Option<RepresentationCandidate>,
    managed: Option<RepresentationCandidate>,
) -> Result<LifecycleRepresentation, LifecycleRepresentationError> {
    let native_blocked = native
        .as_ref()
        .is_some_and(|candidate| candidate.plan.blocked());
    let managed_blocked = managed
        .as_ref()
        .is_some_and(|candidate| candidate.plan.blocked());
    let native = native.filter(|candidate| !candidate.plan.blocked());
    let managed = managed.filter(|candidate| !candidate.plan.blocked());

    match (native, managed) {
        (None, None) if native_blocked || managed_blocked => {
            Err(LifecycleRepresentationError::RequiredComponentsBlocked)
        }
        (None, None) => Err(LifecycleRepresentationError::NoSupportedRepresentation),
        (Some(native), None) => Ok(native.representation),
        (None, Some(managed)) => Ok(managed.representation),
        (Some(native), Some(managed)) => {
            let native_faithful = native.plan.omitted_optional.is_empty();
            let managed_faithful = managed.plan.omitted_optional.is_empty();
            if native_faithful {
                return Ok(LifecycleRepresentation::Native);
            }
            if managed_faithful {
                return Ok(LifecycleRepresentation::Managed);
            }

            if native.plan.included == managed.plan.included {
                return Ok(LifecycleRepresentation::Native);
            }
            if managed.plan.included.is_superset(&native.plan.included) {
                return Ok(LifecycleRepresentation::Managed);
            }
            if native.plan.included.is_superset(&managed.plan.included) {
                return Ok(LifecycleRepresentation::Native);
            }
            Err(LifecycleRepresentationError::IncomparablePartialRepresentations)
        }
    }
}

/// Recover the already-applied representation for one exact target binding.
///
/// Native/adopted harness-owned bindings and skilltap-owned materialized
/// bindings are intentionally distinct. A native id on a managed binding is
/// not sufficient native evidence: managed lifecycle state records the
/// resource's native-shaped selector for diagnostics and operation identity.
pub fn applied_lifecycle_representation(
    state: &TargetResourceState,
) -> Result<LifecycleRepresentation, LifecycleRepresentationError> {
    let native = matches!(
        (state.provenance(), state.ownership()),
        (
            Provenance::Native | Provenance::Adopted,
            Ownership::Harness | Ownership::Unmanaged,
        )
    );
    // A materialized marketplace may legitimately have an empty projection
    // manifest: its managed catalog has no Skill/MCP component entries. The
    // provenance/ownership pair is therefore the representation pin; the
    // manifest remains execution evidence rather than the route discriminator.
    let managed =
        state.provenance() == Provenance::Materialized && state.ownership() == Ownership::Skilltap;

    if native && !state.managed_projections().is_empty() {
        return Err(LifecycleRepresentationError::ContradictoryAppliedState);
    }

    match (native, managed) {
        (true, false) => Ok(LifecycleRepresentation::Native),
        (false, true) => Ok(LifecycleRepresentation::Managed),
        _ => Err(LifecycleRepresentationError::ContradictoryAppliedState),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        ComponentId, Fingerprint, FingerprintAlgorithm, HarnessId, NativeId, ResourceId,
        ResourceKey, Scope,
    };
    use crate::storage::{ResourceState, Timestamp};

    fn component(value: &str) -> ComponentId {
        ComponentId::new(value).unwrap()
    }

    fn plan(included: &[&str], omitted: &[&str], blocked: &[&str]) -> MaterializationPlan {
        MaterializationPlan {
            target: HarnessId::new("fixture").unwrap(),
            included: included.iter().map(|value| component(value)).collect(),
            omitted_optional: omitted.iter().map(|value| component(value)).collect(),
            blocked_required: blocked.iter().map(|value| component(value)).collect(),
        }
    }

    fn candidate(
        representation: LifecycleRepresentation,
        included: &[&str],
        omitted: &[&str],
        blocked: &[&str],
    ) -> RepresentationCandidate {
        RepresentationCandidate {
            representation,
            plan: plan(included, omitted, blocked),
        }
    }

    fn state(
        provenance: Provenance,
        ownership: Ownership,
        fingerprint: bool,
        managed: bool,
    ) -> TargetResourceState {
        target_state("fixture", provenance, ownership, fingerprint, managed)
    }

    fn target_state(
        harness: &str,
        provenance: Provenance,
        ownership: Ownership,
        fingerprint: bool,
        managed: bool,
    ) -> TargetResourceState {
        let fingerprint = fingerprint
            .then(|| Fingerprint::new(FingerprintAlgorithm::Sha256, "a".repeat(64)).unwrap());
        TargetResourceState::new(
            HarnessId::new(harness).unwrap(),
            Some(NativeId::new("fixture").unwrap()),
            provenance,
            ownership,
            None,
            None,
            fingerprint,
            None,
            None,
            Timestamp::new(1, 0).unwrap(),
            None,
        )
        .unwrap()
        .with_managed_projections(managed.then(|| {
            crate::storage::ManagedProjection::Skill {
                id: crate::domain::RelativeArtifactPath::new("fixture").unwrap(),
                fingerprint: Fingerprint::new(FingerprintAlgorithm::Sha256, "b".repeat(64))
                    .unwrap(),
            }
        }))
    }

    #[test]
    fn existing_and_marketplace_evidence_pin_the_representation() {
        assert_eq!(
            select_lifecycle_representation(RepresentationEvidence::Existing(
                LifecycleRepresentation::Managed,
            )),
            Ok(LifecycleRepresentation::Managed)
        );
        assert_eq!(
            select_lifecycle_representation(RepresentationEvidence::Marketplace(
                LifecycleRepresentation::Native,
            )),
            Ok(LifecycleRepresentation::Native)
        );
    }

    #[test]
    fn faithful_native_is_preferred_over_managed() {
        assert_eq!(
            select_lifecycle_representation(RepresentationEvidence::Fresh {
                native: Some(candidate(
                    LifecycleRepresentation::Native,
                    &["skill:a"],
                    &[],
                    &[],
                )),
                managed: Some(candidate(
                    LifecycleRepresentation::Managed,
                    &["skill:a", "mcp:b"],
                    &[],
                    &[],
                )),
            }),
            Ok(LifecycleRepresentation::Native)
        );
    }

    #[test]
    fn managed_strict_superset_wins_only_for_partial_candidates() {
        assert_eq!(
            select_lifecycle_representation(RepresentationEvidence::Fresh {
                native: Some(candidate(
                    LifecycleRepresentation::Native,
                    &["skill:a"],
                    &["mcp:b"],
                    &[],
                )),
                managed: Some(candidate(
                    LifecycleRepresentation::Managed,
                    &["skill:a", "mcp:b"],
                    &["hook:c"],
                    &[],
                )),
            }),
            Ok(LifecycleRepresentation::Managed)
        );
        assert_eq!(
            select_lifecycle_representation(RepresentationEvidence::Fresh {
                native: Some(candidate(
                    LifecycleRepresentation::Native,
                    &["skill:a", "mcp:b"],
                    &["hook:c"],
                    &[],
                )),
                managed: Some(candidate(
                    LifecycleRepresentation::Managed,
                    &["skill:a"],
                    &["mcp:b", "hook:c"],
                    &[],
                )),
            }),
            Ok(LifecycleRepresentation::Native)
        );
    }

    #[test]
    fn equal_partial_plans_prefer_native_and_incomparable_plans_block() {
        let equal = select_lifecycle_representation(RepresentationEvidence::Fresh {
            native: Some(candidate(
                LifecycleRepresentation::Native,
                &["skill:a"],
                &["mcp:b"],
                &[],
            )),
            managed: Some(candidate(
                LifecycleRepresentation::Managed,
                &["skill:a"],
                &["mcp:b"],
                &[],
            )),
        });
        assert_eq!(equal, Ok(LifecycleRepresentation::Native));

        let incomparable = select_lifecycle_representation(RepresentationEvidence::Fresh {
            native: Some(candidate(
                LifecycleRepresentation::Native,
                &["skill:a", "skill:b"],
                &["mcp:c"],
                &[],
            )),
            managed: Some(candidate(
                LifecycleRepresentation::Managed,
                &["skill:a", "hook:c"],
                &["mcp:c"],
                &[],
            )),
        });
        assert_eq!(
            incomparable,
            Err(LifecycleRepresentationError::IncomparablePartialRepresentations)
        );
    }

    #[test]
    fn required_blocks_and_absent_candidates_fail_or_select_the_only_valid_route() {
        assert_eq!(
            select_lifecycle_representation(RepresentationEvidence::Fresh {
                native: Some(candidate(
                    LifecycleRepresentation::Native,
                    &[],
                    &[],
                    &["agent:required"],
                )),
                managed: Some(candidate(
                    LifecycleRepresentation::Managed,
                    &["skill:a"],
                    &[],
                    &[],
                )),
            }),
            Ok(LifecycleRepresentation::Managed)
        );
        assert_eq!(
            select_lifecycle_representation(RepresentationEvidence::Fresh {
                native: None,
                managed: Some(candidate(
                    LifecycleRepresentation::Managed,
                    &[],
                    &[],
                    &["agent:required"],
                )),
            }),
            Err(LifecycleRepresentationError::RequiredComponentsBlocked)
        );
        assert_eq!(
            select_lifecycle_representation(RepresentationEvidence::Fresh {
                native: None,
                managed: None,
            }),
            Err(LifecycleRepresentationError::NoSupportedRepresentation)
        );
    }

    #[test]
    fn applied_state_pins_native_and_managed_and_rejects_contradictions() {
        assert_eq!(
            applied_lifecycle_representation(&state(
                Provenance::Native,
                Ownership::Harness,
                false,
                false,
            )),
            Ok(LifecycleRepresentation::Native)
        );
        assert_eq!(
            applied_lifecycle_representation(&state(
                Provenance::Adopted,
                Ownership::Harness,
                false,
                false,
            )),
            Ok(LifecycleRepresentation::Native)
        );
        assert_eq!(
            applied_lifecycle_representation(&state(
                Provenance::Materialized,
                Ownership::Skilltap,
                true,
                true,
            )),
            Ok(LifecycleRepresentation::Managed)
        );
        assert_eq!(
            applied_lifecycle_representation(&state(
                Provenance::Native,
                Ownership::Harness,
                true,
                true,
            )),
            Err(LifecycleRepresentationError::ContradictoryAppliedState)
        );
        assert_eq!(
            applied_lifecycle_representation(&state(
                Provenance::Direct,
                Ownership::Skilltap,
                false,
                false,
            )),
            Err(LifecycleRepresentationError::ContradictoryAppliedState)
        );
    }

    #[test]
    fn target_local_state_pinning_allows_mixed_targets_and_scopes() {
        let native = target_state(
            "codex",
            Provenance::Native,
            Ownership::Harness,
            false,
            false,
        );
        let managed = target_state(
            "managed",
            Provenance::Materialized,
            Ownership::Skilltap,
            true,
            true,
        );
        let global = ResourceState::new(
            ResourceKey::new(ResourceId::new("plugin:demo").unwrap(), Scope::Global),
            [native],
        )
        .unwrap();
        let project = ResourceState::new(
            ResourceKey::new(
                ResourceId::new("plugin:demo").unwrap(),
                Scope::Project(crate::domain::AbsolutePath::new("/project").unwrap()),
            ),
            [managed],
        )
        .unwrap();
        assert_eq!(
            applied_lifecycle_representation(
                global.target(&HarnessId::new("codex").unwrap()).unwrap()
            ),
            Ok(LifecycleRepresentation::Native)
        );
        assert_eq!(
            applied_lifecycle_representation(
                project.target(&HarnessId::new("managed").unwrap()).unwrap()
            ),
            Ok(LifecycleRepresentation::Managed)
        );
    }
}
