//! Pure, adapter-neutral reconciliation planning.

use std::collections::BTreeSet;

use crate::{
    compatibility::{
        CompatibilityAnalysis, CompatibilityAnalysisError, CompatibilityRequest, analyze,
    },
    domain::{
        Fingerprint, NativeId, ObservationKey, Operation, OperationClass, OperationContractError,
        Ownership, Plan, ResourceHealth, ResourceKey, Scope,
    },
    hook_mapping::{HookContract, HookMappingError, HookTargetContract, analyze_hook},
    storage::ResourceState,
};

/// Expose target-bound compatibility through the reconciliation boundary
/// without importing adapter implementations or performing external I/O.
pub fn compatibility_for_target(
    request: CompatibilityRequest<'_>,
) -> Result<CompatibilityAnalysis, CompatibilityAnalysisError> {
    analyze(request)
}

/// Expose normalized hook equivalence through the reconciliation boundary.
/// Adapters provide the source and target contracts; this layer remains pure
/// and does not materialize files or invoke native harness commands.
pub fn hook_compatibility_for_target(
    source: &HookContract,
    target: &HookTargetContract,
    requiredness: crate::domain::ComponentRequiredness,
    target_harness: &crate::domain::HarnessId,
    resource: &ResourceKey,
) -> Result<crate::domain::CompatibilityResult, HookMappingError> {
    analyze_hook(source, target, requiredness, target_harness, resource)
}

#[derive(Clone, Debug)]
pub struct ReconciliationCandidate {
    pub operation: Operation,
    pub resource: ResourceKey,
    pub expected_identity: Option<NativeId>,
    pub expected_fingerprint: Option<Fingerprint>,
    pub observed: Option<crate::domain::ObservedResource>,
    pub prior_state: Option<ResourceState>,
}

#[derive(Clone, Debug, Default)]
pub struct ReconciliationRequest {
    pub candidates: Vec<ReconciliationCandidate>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ReconciliationFinding {
    Drift { resource: ResourceKey },
    OwnershipConflict { resource: ResourceKey },
    MissingEvidence { resource: ResourceKey },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReconciliationPlan {
    pub plan: Plan,
    pub findings: Vec<ReconciliationFinding>,
}

impl ReconciliationPlan {
    pub fn requires_attention(&self) -> bool {
        !self.findings.is_empty()
            || self.plan.iter().any(|(_, operation)| {
                matches!(
                    operation.class(),
                    OperationClass::Partial
                        | OperationClass::Unsupported
                        | OperationClass::Conflict
                )
            })
    }
}

#[derive(Debug)]
pub enum ReconciliationError {
    DuplicateResource { resource: ResourceKey },
    CandidateScopeMismatch { resource: ResourceKey },
    CandidateSelectorMismatch { resource: ResourceKey },
    Operation(OperationContractError),
}

impl std::fmt::Display for ReconciliationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateResource { resource } => {
                write!(formatter, "duplicate reconciliation resource `{resource}`")
            }
            Self::CandidateScopeMismatch { resource } => {
                write!(
                    formatter,
                    "reconciliation candidate scope mismatches `{resource}`"
                )
            }
            Self::CandidateSelectorMismatch { resource } => {
                write!(formatter, "reconciliation selector mismatches `{resource}`")
            }
            Self::Operation(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for ReconciliationError {}

impl From<OperationContractError> for ReconciliationError {
    fn from(error: OperationContractError) -> Self {
        Self::Operation(error)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReconciliationDisposition {
    Keep(Operation),
    NoOp(Operation),
    Attention {
        operation: Operation,
        finding: ReconciliationFinding,
    },
}

/// Classify a validated adapter candidate without external I/O.
pub fn classify_candidate(
    candidate: &ReconciliationCandidate,
) -> Result<ReconciliationDisposition, ReconciliationError> {
    if candidate.resource != *candidate.operation.selector().resource() {
        return Err(ReconciliationError::CandidateSelectorMismatch {
            resource: candidate.resource.clone(),
        });
    }
    if candidate.resource.scope() != candidate.operation.scope() {
        return Err(ReconciliationError::CandidateScopeMismatch {
            resource: candidate.resource.clone(),
        });
    }
    if let Some(observed) = &candidate.observed {
        if observed.key().resource() != &candidate.resource {
            return Err(ReconciliationError::CandidateScopeMismatch {
                resource: candidate.resource.clone(),
            });
        }
        if let Some(identity) = &candidate.expected_identity
            && observed.native_identity() != identity
        {
            return Ok(ReconciliationDisposition::Attention {
                operation: candidate.operation.clone(),
                finding: ReconciliationFinding::Drift {
                    resource: candidate.resource.clone(),
                },
            });
        }
        if let Some(fingerprint) = &candidate.expected_fingerprint
            && observed.fingerprint() != Some(fingerprint)
        {
            return Ok(ReconciliationDisposition::Attention {
                operation: candidate.operation.clone(),
                finding: ReconciliationFinding::Drift {
                    resource: candidate.resource.clone(),
                },
            });
        }
        if let Some(prior) = &candidate.prior_state
            && let Some(fingerprint) = prior.fingerprint()
            && observed.fingerprint() != Some(fingerprint)
        {
            return Ok(ReconciliationDisposition::Attention {
                operation: candidate.operation.clone(),
                finding: ReconciliationFinding::Drift {
                    resource: candidate.resource.clone(),
                },
            });
        }
        if observed.ownership() == Ownership::Unmanaged
            && matches!(
                candidate.operation.class(),
                OperationClass::SafeNative
                    | OperationClass::SafeFaithfulEquivalent
                    | OperationClass::SafeMaterialization
            )
        {
            return Ok(ReconciliationDisposition::Attention {
                operation: candidate.operation.clone(),
                finding: ReconciliationFinding::OwnershipConflict {
                    resource: candidate.resource.clone(),
                },
            });
        }
        if matches!(
            observed.health(),
            ResourceHealth::Drifted | ResourceHealth::Degraded
        ) {
            return Ok(ReconciliationDisposition::Attention {
                operation: candidate.operation.clone(),
                finding: ReconciliationFinding::Drift {
                    resource: candidate.resource.clone(),
                },
            });
        }
        if observed.health() == ResourceHealth::Unknown {
            return Ok(ReconciliationDisposition::Attention {
                operation: candidate.operation.clone(),
                finding: ReconciliationFinding::MissingEvidence {
                    resource: candidate.resource.clone(),
                },
            });
        }
    } else if candidate.operation.class() != OperationClass::NoOp {
        return Ok(ReconciliationDisposition::Attention {
            operation: candidate.operation.clone(),
            finding: ReconciliationFinding::MissingEvidence {
                resource: candidate.resource.clone(),
            },
        });
    }

    if candidate.operation.class() == OperationClass::NoOp {
        Ok(ReconciliationDisposition::NoOp(candidate.operation.clone()))
    } else {
        Ok(ReconciliationDisposition::Keep(candidate.operation.clone()))
    }
}

/// Assemble a deterministic validated plan from already selected candidates.
pub fn plan_reconciliation(
    request: ReconciliationRequest,
) -> Result<ReconciliationPlan, ReconciliationError> {
    let mut resources = BTreeSet::new();
    let mut operations = Vec::new();
    let mut findings = Vec::new();
    for candidate in request.candidates {
        if !resources.insert(candidate.resource.clone()) {
            return Err(ReconciliationError::DuplicateResource {
                resource: candidate.resource,
            });
        }
        match classify_candidate(&candidate)? {
            ReconciliationDisposition::Keep(operation)
            | ReconciliationDisposition::NoOp(operation) => operations.push(operation),
            ReconciliationDisposition::Attention { operation, finding } => {
                operations.push(operation);
                findings.push(finding);
            }
        }
    }
    findings.sort();
    Ok(ReconciliationPlan {
        plan: Plan::new(operations)?,
        findings,
    })
}

#[allow(dead_code)]
fn _scope_of_observation(key: &ObservationKey) -> &Scope {
    key.resource().scope()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        AffectedSurface, CompatibilityClass, CompatibilityResult, ComponentGraph, ComponentId,
        ComponentRequiredness, EvidenceCode, EvidenceDetail, HarnessId, ObservationLayer,
        OperationAction, OperationId, OperationReason, OperationSelector, OperationSemantics,
        Provenance, ResourceId, ResourceKind, TransferFidelity,
    };

    fn resource() -> ResourceKey {
        ResourceKey::new(
            ResourceId::new("skill:demo").unwrap(),
            crate::domain::Scope::Global,
        )
    }

    fn no_op(resource: ResourceKey, id: &str) -> Operation {
        let target = HarnessId::new("codex").unwrap();
        let compatibility = CompatibilityResult::new(
            target.clone(),
            CompatibilityClass::Compatible,
            crate::domain::TransferFidelity::Faithful,
            [],
            [],
        )
        .unwrap();
        Operation::new(
            OperationId::new(id).unwrap(),
            target.clone(),
            OperationSelector::Resource {
                resource: resource.clone(),
            },
            OperationSemantics::new(
                OperationAction::SkillInstall,
                resource.scope().clone(),
                OperationReason::new(
                    EvidenceCode::new("plan.noop").unwrap(),
                    EvidenceDetail::new("No change is required").unwrap(),
                ),
                compatibility,
                Provenance::Native,
                [AffectedSurface::file(
                    crate::domain::AbsolutePath::new("/tmp/skilltap-reconcile").unwrap(),
                )],
            ),
            OperationClass::NoOp,
            crate::domain::Reversibility::NotApplicable,
            [],
            crate::domain::AcknowledgmentRequirement::not_required(),
            None,
        )
        .unwrap()
    }

    #[test]
    fn empty_request_is_a_deterministic_empty_plan() {
        let result = plan_reconciliation(ReconciliationRequest::default()).unwrap();
        assert!(result.plan.is_empty());
        assert!(result.findings.is_empty());
        assert!(!result.requires_attention());
    }

    #[test]
    fn no_op_candidate_is_kept_without_external_evidence() {
        let resource = resource();
        let candidate = ReconciliationCandidate {
            operation: no_op(resource.clone(), "noop"),
            resource,
            expected_identity: None,
            expected_fingerprint: None,
            observed: None,
            prior_state: None,
        };
        let disposition = classify_candidate(&candidate).unwrap();
        assert!(matches!(disposition, ReconciliationDisposition::NoOp(_)));
        let plan = plan_reconciliation(ReconciliationRequest {
            candidates: vec![candidate],
        })
        .unwrap();
        assert!(!plan.requires_attention());
        assert_eq!(plan.plan.iter().count(), 1);
    }

    #[test]
    fn identity_change_is_a_drift_finding() {
        let resource = resource();
        let harness = HarnessId::new("codex").unwrap();
        let observed = crate::domain::ObservedResource::new(
            ObservationKey::new(
                resource.clone(),
                harness.clone(),
                ObservationLayer::Effective,
            ),
            ResourceKind::StandaloneSkill,
            Provenance::Native,
            Ownership::Harness,
            ResourceHealth::Healthy,
            None,
            ComponentGraph::new([]).unwrap(),
            [].into(),
            NativeId::new("new-native-id").unwrap(),
            None,
            None,
        );
        let candidate = ReconciliationCandidate {
            operation: no_op(resource.clone(), "drift"),
            resource,
            expected_identity: Some(NativeId::new("old-native-id").unwrap()),
            expected_fingerprint: None,
            observed: Some(observed),
            prior_state: None,
        };
        let result = plan_reconciliation(ReconciliationRequest {
            candidates: vec![candidate],
        })
        .unwrap();
        assert!(matches!(
            result.findings.as_slice(),
            [ReconciliationFinding::Drift { .. }]
        ));
        assert!(result.requires_attention());
    }

    #[test]
    fn duplicate_resource_candidates_fail_before_plan_assembly() {
        let resource = resource();
        let first = ReconciliationCandidate {
            operation: no_op(resource.clone(), "one"),
            resource: resource.clone(),
            expected_identity: None,
            expected_fingerprint: None,
            observed: None,
            prior_state: None,
        };
        let second = ReconciliationCandidate {
            operation: no_op(resource.clone(), "two"),
            resource: resource.clone(),
            expected_identity: None,
            expected_fingerprint: None,
            observed: None,
            prior_state: None,
        };
        assert!(matches!(
            plan_reconciliation(ReconciliationRequest {
                candidates: vec![first, second],
            }),
            Err(ReconciliationError::DuplicateResource { .. })
        ));
    }

    #[test]
    fn reconciliation_exposes_scope_exact_compatibility_selectors() {
        let resource = resource();
        let graph = crate::plugin_graph::normalize(
            crate::domain::Source::new(
                crate::domain::SourceKind::Git,
                crate::domain::SourceLocator::new("https://example.test/plugin.git").unwrap(),
                None,
            )
            .unwrap(),
            [crate::plugin_graph::ComponentDeclaration {
                id: crate::domain::ComponentId::new("hook:optional").unwrap(),
                kind: crate::domain::ComponentKind::Hook,
                requiredness: crate::domain::ComponentRequiredness::Optional,
                dependencies: BTreeSet::new(),
                relative_path: crate::domain::RelativeArtifactPath::new("hooks/optional").unwrap(),
                declared_name: Some("optional".to_owned()),
            }],
        )
        .unwrap();
        let target = HarnessId::new("codex").unwrap();
        let capabilities = crate::domain::CapabilitySet::new([]);
        let analysis = compatibility_for_target(CompatibilityRequest {
            resource: &resource,
            graph: &graph,
            target: &target,
            capabilities: &capabilities,
            occupied: &BTreeSet::new(),
        })
        .unwrap();
        assert_eq!(analysis.aggregate.fidelity(), TransferFidelity::Partial);
        assert!(
            analysis
                .acknowledgment_selectors
                .contains(&OperationSelector::Component {
                    resource,
                    component_id: crate::domain::ComponentId::new("hook:optional").unwrap(),
                })
        );
    }

    #[test]
    fn reconciliation_exposes_hook_equivalence_without_materialization() {
        let source = HookContract {
            component: ComponentId::new("hook:notify").unwrap(),
            event: "session_start".to_owned(),
            payload: crate::hook_mapping::HookPayload::Json,
            failure: crate::hook_mapping::HookFailure::Block,
            working_directory: crate::hook_mapping::HookWorkingDirectory::Plugin,
            environment_references: ["env:HOME".to_owned()].into_iter().collect(),
            executable: true,
        };
        let target = HookTargetContract {
            event: "session_start".to_owned(),
            payload: crate::hook_mapping::HookPayload::Json,
            failure: crate::hook_mapping::HookFailure::Block,
            working_directory: crate::hook_mapping::HookWorkingDirectory::Plugin,
            environment_references: ["env:HOME".to_owned()].into_iter().collect(),
            executable: true,
        };
        let result = hook_compatibility_for_target(
            &source,
            &target,
            ComponentRequiredness::Required,
            &HarnessId::new("codex").unwrap(),
            &resource(),
        )
        .unwrap();
        assert_eq!(result.fidelity(), TransferFidelity::Faithful);
        assert!(result.evidence().is_empty());
    }
}
