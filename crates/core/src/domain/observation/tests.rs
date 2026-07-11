use std::collections::BTreeSet;

use super::*;
use crate::domain::{
    AbsolutePath, CapabilityProfileId, CapabilitySupport, ComponentGraph, ConfiguredBinary,
    ExecutableFileIdentity, HarnessReachability, NativeId, ObservationField, ObservationFields,
    ObservationFindingCode, ObservationLayer, ObservationSeverity, ObservationSubject,
    ObservationSummary, ObservedDependency, Ownership, Provenance, ResourceHealth, ResourceId,
    ResourceKey, ResourceKind, ScopedCapabilitySets,
};

fn harness(value: &str) -> HarnessId {
    HarnessId::new(value).unwrap()
}

fn path(value: &str) -> AbsolutePath {
    AbsolutePath::new(value).unwrap()
}

fn capabilities(support: CapabilitySupport) -> ScopedCapabilitySets {
    let set = || CapabilitySet::new([(CapabilityId::new("plugin.observe").unwrap(), support)]);
    ScopedCapabilitySets::new(set(), set())
}

fn evidence(harness_name: &str, unknown_version: bool) -> ObservationEvidence {
    let installation = HarnessInstallation::new(
        harness(harness_name),
        ConfiguredBinary::path_lookup(NativeId::new(harness_name).unwrap()).unwrap(),
        HarnessReachability::Reachable {
            executable: ExecutableIdentity::new(
                path(&format!("/usr/bin/{harness_name}")),
                ExecutableFileIdentity::new(7, 11),
            ),
            native_version: NativeVersion::new("3.7.0").unwrap(),
        },
    );
    let profile = if unknown_version {
        CapabilityProfileSelection::unknown_version(capabilities(CapabilitySupport::Unverified))
    } else {
        CapabilityProfileSelection::verified(
            CapabilityProfileId::new(format!("{harness_name}-v3")).unwrap(),
            capabilities(CapabilitySupport::Supported),
        )
    };
    ObservationEvidence::new(&installation, profile).unwrap()
}

fn request(harness_name: &str, scope: Scope) -> ObservationRequest {
    ObservationRequest::new(scope, evidence(harness_name, false))
}

fn resource(request: &ObservationRequest, id: &str, layer: ObservationLayer) -> ObservedResource {
    let key = ResourceKey::new(ResourceId::new(id).unwrap(), request.scope().clone());
    ObservedResource::new(
        ObservationKey::new(key, request.evidence().harness().clone(), layer),
        ResourceKind::Plugin,
        Provenance::Native,
        Ownership::Harness,
        ResourceHealth::Healthy,
        None,
        ComponentGraph::default(),
        BTreeSet::from([ObservedDependency::Unresolved {
            native_identity: NativeId::new("native-missing-edge").unwrap(),
        }]),
        NativeId::new(format!("{id}@native")).unwrap(),
        None,
        None,
    )
}

fn finding(request: &ObservationRequest) -> ObservationFinding {
    ObservationFinding::new(
        ObservationFindingCode::ResourceUnmanaged,
        ObservationSummary::ResourceUnmanaged,
        ObservationSeverity::Info,
        ObservationSubject::Harness {
            harness: request.evidence().harness().clone(),
            scope: request.scope().clone(),
        },
        ObservationFields::new([ObservationField::AffectedCount(1)]).unwrap(),
    )
}

fn observation(request: ObservationRequest, id: &str) -> HarnessObservation {
    HarnessObservation::new(
        request.clone(),
        [
            resource(&request, id, ObservationLayer::Declared),
            resource(&request, id, ObservationLayer::Effective),
        ],
        [finding(&request)],
    )
    .unwrap()
}

#[test]
fn evidence_is_one_reachable_executable_version_profile_envelope() {
    let evidence = evidence("codex", false);
    assert_eq!(evidence.harness().as_str(), "codex");
    assert_eq!(evidence.executable().path().as_str(), "/usr/bin/codex");
    assert_eq!(evidence.native_version().as_str(), "3.7.0");
    assert_eq!(
        evidence.profile_authority(),
        ProfileAuthority::VerifiedCompiled
    );
    assert_eq!(
        evidence
            .observation_capabilities(&Scope::Global)
            .support(&CapabilityId::new("plugin.observe").unwrap()),
        Some(CapabilitySupport::Supported)
    );

    let unreachable = HarnessInstallation::new(
        harness("claude"),
        ConfiguredBinary::path_lookup(NativeId::new("claude").unwrap()).unwrap(),
        HarnessReachability::Unreachable {
            reason: crate::domain::UnreachableReason::NotFound,
        },
    );
    assert!(matches!(
        ObservationEvidence::new(
            &unreachable,
            CapabilityProfileSelection::unknown_version(capabilities(
                CapabilitySupport::Unverified
            ))
        ),
        Err(ObservationContractError::InstallationUnreachable { .. })
    ));
}

#[test]
fn unknown_versions_remain_observation_valid_and_observe_only() {
    let request = ObservationRequest::new(Scope::Global, evidence("codex", true));
    assert_eq!(
        request.evidence().profile_authority(),
        ProfileAuthority::ObserveOnly
    );
    assert!(
        request
            .evidence()
            .profile()
            .mutation_capabilities()
            .is_none()
    );
    assert!(HarnessObservation::new(request, [], []).is_ok());
}

#[test]
fn requests_and_evidence_have_strict_deterministic_wires() {
    let request = request("codex", Scope::Project(path("/work/project")));
    let json = serde_json::to_string(&request).unwrap();
    assert_eq!(
        serde_json::from_str::<ObservationRequest>(&json).unwrap(),
        request
    );
    assert_eq!(serde_json::to_string(&request).unwrap(), json);

    let mut unknown = serde_json::to_value(&request).unwrap();
    unknown["harness"] = serde_json::json!("claude");
    assert!(serde_json::from_value::<ObservationRequest>(unknown).is_err());
    let mut unknown = serde_json::to_value(request.evidence()).unwrap();
    unknown["stdout"] = serde_json::json!("native-secret");
    assert!(serde_json::from_value::<ObservationEvidence>(unknown).is_err());
}

#[test]
fn harness_observations_validate_every_resource_and_finding_context() {
    let global = request("codex", Scope::Global);
    let valid = observation(global.clone(), "plugin:shared");
    assert_eq!(valid.resources().len(), 2);
    assert_eq!(valid.findings().len(), 1);
    assert_eq!(
        serde_json::from_str::<HarnessObservation>(&serde_json::to_string(&valid).unwrap())
            .unwrap(),
        valid
    );

    assert!(matches!(
        HarnessObservation::new(
            global.clone(),
            [resource(
                &request("claude", Scope::Global),
                "plugin:shared",
                ObservationLayer::Effective
            )],
            []
        ),
        Err(ObservationContractError::ResourceContextMismatch { .. })
    ));
    assert!(matches!(
        HarnessObservation::new(
            global.clone(),
            [resource(
                &request("codex", Scope::Project(path("/work/project"))),
                "plugin:shared",
                ObservationLayer::Effective
            )],
            []
        ),
        Err(ObservationContractError::ResourceContextMismatch { .. })
    ));
    assert!(matches!(
        {
            let duplicate = resource(&global, "plugin:shared", ObservationLayer::Effective);
            HarnessObservation::new(global.clone(), [duplicate.clone(), duplicate], [])
        },
        Err(ObservationContractError::DuplicateObservation { .. })
    ));
    let foreign = request("claude", Scope::Global);
    assert!(matches!(
        HarnessObservation::new(global, [], [finding(&foreign)]),
        Err(ObservationContractError::FindingContextMismatch { .. })
    ));
}

#[test]
fn batches_and_partial_environments_are_duplicate_rejecting_and_deterministic() {
    let global = request("codex", Scope::Global);
    let project_a = request("codex", Scope::Project(path("/work/a")));
    let project_b = request("codex", Scope::Project(path("/work/b")));
    let claude = request("claude", Scope::Global);
    let forward = ObservationBatch::new([
        global.clone(),
        project_a.clone(),
        project_b.clone(),
        claude.clone(),
    ])
    .unwrap();
    let reverse = ObservationBatch::new([
        claude.clone(),
        project_b.clone(),
        project_a.clone(),
        global.clone(),
    ])
    .unwrap();
    assert_eq!(
        serde_json::to_string(&forward).unwrap(),
        serde_json::to_string(&reverse).unwrap()
    );
    assert!(matches!(
        ObservationBatch::new([global.clone(), global.clone()]),
        Err(ObservationContractError::DuplicateTarget { .. })
    ));

    let outcomes = [
        HarnessObservationOutcome::observed(observation(global.clone(), "plugin:shared")),
        HarnessObservationOutcome::observed(observation(project_a, "plugin:shared")),
        HarnessObservationOutcome::observed(observation(project_b, "plugin:shared")),
        HarnessObservationOutcome::failed(
            claude,
            ObservationAdapterError::NativeStateUnreadable {},
        ),
    ];
    let environment = ObservedEnvironment::new(forward.clone(), outcomes.clone()).unwrap();
    let reversed = ObservedEnvironment::new(reverse, outcomes.into_iter().rev()).unwrap();
    assert_eq!(environment.iter().count(), 4);
    assert_eq!(
        serde_json::to_string(&environment).unwrap(),
        serde_json::to_string(&reversed).unwrap()
    );
    let json = serde_json::to_string(&environment).unwrap();
    assert_eq!(
        serde_json::from_str::<ObservedEnvironment>(&json).unwrap(),
        environment
    );
    assert!(matches!(
        ObservedEnvironment::new(
            ObservationBatch::new([global.clone(), request("claude", Scope::Global)]).unwrap(),
            [HarnessObservationOutcome::observed(observation(
                global,
                "plugin:shared"
            ))]
        ),
        Err(ObservationContractError::MissingTarget { .. })
    ));
    let verified = request("codex", Scope::Global);
    let observe_only = ObservationRequest::new(Scope::Global, evidence("codex", true));
    assert!(matches!(
        ObservedEnvironment::new(
            ObservationBatch::new([verified]).unwrap(),
            [HarnessObservationOutcome::failed(
                observe_only,
                ObservationAdapterError::ExecutableChanged {}
            )]
        ),
        Err(ObservationContractError::OutcomeRequestMismatch { .. })
    ));
}

#[test]
fn adapter_errors_are_closed_source_free_and_canary_safe() {
    const SECRET: &str = "sk-test-auth-must-not-enter";
    let error = ObservationAdapterError::DeadlineExceeded {};
    for rendered in [
        error.to_string(),
        format!("{error:?}"),
        serde_json::to_string(&error).unwrap(),
    ] {
        assert!(!rendered.contains(SECRET));
    }
    assert!(
        serde_json::from_value::<ObservationAdapterError>(serde_json::json!({
            "kind":"deadline_exceeded", "message":SECRET
        }))
        .is_err()
    );
    assert!(serde_json::from_str::<ObservationAdapterError>(&format!(r#""{SECRET}""#)).is_err());
}

struct FakeAdapter {
    harness: HarnessId,
    fail: bool,
}

impl HarnessObservationAdapter for FakeAdapter {
    fn harness(&self) -> &HarnessId {
        &self.harness
    }
    fn observe(
        &self,
        request: &ObservationRequest,
    ) -> Result<HarnessObservation, ObservationAdapterError> {
        if self.fail {
            Err(ObservationAdapterError::NativeShapeUnsupported {})
        } else {
            HarnessObservation::new(request.clone(), [], [])
                .map_err(|_| ObservationAdapterError::NativeShapeUnsupported {})
        }
    }
}

struct FakeCoordinator {
    codex: FakeAdapter,
    claude: FakeAdapter,
}

impl ObservationCoordinator for FakeCoordinator {
    fn observe(&self, batch: &ObservationBatch) -> ObservedEnvironment {
        let outcomes = batch.iter().map(|(_, request)| {
            let adapter = if request.evidence().harness() == self.codex.harness() {
                &self.codex
            } else {
                &self.claude
            };
            match adapter.observe(request) {
                Ok(observation) => HarnessObservationOutcome::observed(observation),
                Err(error) => HarnessObservationOutcome::failed(request.clone(), error),
            }
        });
        ObservedEnvironment::new(batch.clone(), outcomes).unwrap()
    }
}

#[test]
fn behavior_ports_cross_only_normalized_partial_snapshot_values() {
    let coordinator = FakeCoordinator {
        codex: FakeAdapter {
            harness: harness("codex"),
            fail: false,
        },
        claude: FakeAdapter {
            harness: harness("claude"),
            fail: true,
        },
    };
    let codex = request("codex", Scope::Global);
    let claude = request("claude", Scope::Global);
    let environment =
        coordinator.observe(&ObservationBatch::new([claude.clone(), codex.clone()]).unwrap());
    assert!(matches!(
        environment.get(&codex.target()),
        Some(HarnessObservationOutcome::Observed { .. })
    ));
    assert!(matches!(
        environment.get(&claude.target()),
        Some(HarnessObservationOutcome::Failed { .. })
    ));
}
