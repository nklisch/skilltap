use super::*;
use crate::domain::{CompatibilityClass, CompatibilityEvidence, ConsequenceSummary};

fn operation_id(value: &str) -> OperationId {
    OperationId::new(value).unwrap()
}

fn resource_selector(value: &str) -> OperationSelector {
    OperationSelector::Resource {
        resource_id: ResourceId::new(value).unwrap(),
    }
}

fn component_selector(resource: &str, component: &str) -> OperationSelector {
    OperationSelector::Component {
        resource_id: ResourceId::new(resource).unwrap(),
        component_id: ComponentId::new(component).unwrap(),
    }
}

fn consequence() -> MaterialConsequence {
    MaterialConsequence::new(
        ConsequenceCode::new("component.omitted").unwrap(),
        [ComponentId::new("hook:format").unwrap()],
        ConsequenceSummary::new("The formatting hook will not be installed").unwrap(),
    )
}

fn semantics(target: &str) -> OperationSemantics {
    semantics_with(
        target,
        [AffectedSurface::file(
            AbsolutePath::new("/tmp/skilltap-operation-test").unwrap(),
        )],
    )
}

fn semantics_for_fidelity(
    target: &str,
    fidelity: TransferFidelity,
    consequences: impl IntoIterator<Item = MaterialConsequence>,
    affected_surfaces: impl IntoIterator<Item = AffectedSurface>,
) -> OperationSemantics {
    let target = HarnessId::new(target).unwrap();
    let evidence = (!fidelity.is_faithful()).then(|| {
        CompatibilityEvidence::new(
            EvidenceCode::new("target.transfer.non_faithful").unwrap(),
            target.clone(),
            [],
            EvidenceDetail::new("The target cannot preserve the resource faithfully").unwrap(),
        )
    });
    OperationSemantics::new(
        OperationAction::Materialize,
        Scope::Global,
        OperationReason::new(
            EvidenceCode::new("desired.plugin.missing").unwrap(),
            EvidenceDetail::new("The desired plugin is not installed").unwrap(),
        ),
        CompatibilityResult::new(
            target,
            CompatibilityClass::TargetSpecific,
            fidelity,
            evidence,
            consequences,
        )
        .unwrap(),
        Provenance::Materialized,
        affected_surfaces,
    )
}

fn partial_semantics(
    target: &str,
    consequences: impl IntoIterator<Item = MaterialConsequence>,
) -> OperationSemantics {
    semantics_for_fidelity(
        target,
        TransferFidelity::Partial,
        consequences,
        [AffectedSurface::file(
            AbsolutePath::new("/tmp/skilltap-operation-test").unwrap(),
        )],
    )
}

fn semantics_with(
    target: &str,
    affected_surfaces: impl IntoIterator<Item = AffectedSurface>,
) -> OperationSemantics {
    let target = HarnessId::new(target).unwrap();
    OperationSemantics::new(
        OperationAction::PluginInstall,
        Scope::Global,
        OperationReason::new(
            EvidenceCode::new("desired.plugin.missing").unwrap(),
            EvidenceDetail::new("The desired plugin is not installed").unwrap(),
        ),
        CompatibilityResult::new(
            target,
            CompatibilityClass::Compatible,
            TransferFidelity::Faithful,
            [],
            [],
        )
        .unwrap(),
        Provenance::Native,
        affected_surfaces,
    )
}

fn partial_operation(id: &str, dependencies: &[&str]) -> Operation {
    let selectors = [component_selector("plugin:tools", "hook:format")];
    let consequences = [consequence()];
    Operation::new(
        operation_id(id),
        HarnessId::new("codex").unwrap(),
        resource_selector("plugin:tools"),
        partial_semantics("codex", consequences.clone()),
        OperationClass::Partial,
        Reversibility::Reversible,
        dependencies
            .iter()
            .map(|id| OperationDependency::new(operation_id(id))),
        AcknowledgmentRequirement::required(selectors.clone(), consequences.clone()).unwrap(),
        Some(AttentionReason::acknowledgment_required(selectors, consequences).unwrap()),
    )
    .unwrap()
}

fn safe_operation(id: &str, dependencies: &[&str]) -> Operation {
    Operation::new(
        operation_id(id),
        HarnessId::new("claude").unwrap(),
        resource_selector("plugin:tools"),
        semantics("claude"),
        OperationClass::SafeNative,
        Reversibility::Reversible,
        dependencies
            .iter()
            .map(|id| OperationDependency::new(operation_id(id))),
        AcknowledgmentRequirement::not_required(),
        None,
    )
    .unwrap()
}

fn blocked_operation(id: &str, class: OperationClass) -> Operation {
    let attention = match class {
        OperationClass::Unsupported => AttentionReason::unsupported(
            EvidenceCode::new("native.unsupported").unwrap(),
            EvidenceDetail::new("No native operation is available").unwrap(),
        ),
        OperationClass::Conflict => AttentionReason::conflict(
            EvidenceCode::new("native.conflict").unwrap(),
            EvidenceDetail::new("An unmanaged native resource conflicts").unwrap(),
        ),
        _ => panic!("blocked_operation requires unsupported or conflict"),
    };
    Operation::new(
        operation_id(id),
        HarnessId::new("codex").unwrap(),
        resource_selector("plugin:tools"),
        semantics_for_fidelity("codex", TransferFidelity::Blocked, [consequence()], []),
        class,
        Reversibility::NotApplicable,
        [],
        AcknowledgmentRequirement::not_required(),
        Some(attention),
    )
    .unwrap()
}

fn no_op(id: &str) -> Operation {
    Operation::new(
        operation_id(id),
        HarnessId::new("codex").unwrap(),
        resource_selector("plugin:tools"),
        semantics("codex"),
        OperationClass::NoOp,
        Reversibility::NotApplicable,
        [],
        AcknowledgmentRequirement::not_required(),
        None,
    )
    .unwrap()
}

fn operation_with_class_and_fidelity(
    id: &str,
    class: OperationClass,
    fidelity: TransferFidelity,
) -> Result<Operation, OperationContractError> {
    let consequences = (!fidelity.is_faithful()).then(consequence);
    let surfaces = matches!(
        class,
        OperationClass::SafeNative
            | OperationClass::SafeFaithfulEquivalent
            | OperationClass::SafeMaterialization
            | OperationClass::Partial
    )
    .then(|| AffectedSurface::file(AbsolutePath::new("/tmp/skilltap-matrix").unwrap()));
    let semantics = semantics_for_fidelity("codex", fidelity, consequences.clone(), surfaces);
    let (acknowledgment, attention) = match class {
        OperationClass::Partial => {
            let consequences = consequences.clone().unwrap_or_else(consequence);
            let selectors = [resource_selector("plugin:tools")];
            (
                AcknowledgmentRequirement::required(selectors.clone(), [consequences.clone()])
                    .unwrap(),
                Some(AttentionReason::acknowledgment_required(selectors, [consequences]).unwrap()),
            )
        }
        OperationClass::Unsupported => (
            AcknowledgmentRequirement::not_required(),
            Some(AttentionReason::unsupported(
                EvidenceCode::new("native.unsupported").unwrap(),
                EvidenceDetail::new("No native operation exists").unwrap(),
            )),
        ),
        OperationClass::Conflict => (
            AcknowledgmentRequirement::not_required(),
            Some(AttentionReason::conflict(
                EvidenceCode::new("native.conflict").unwrap(),
                EvidenceDetail::new("An unmanaged resource conflicts").unwrap(),
            )),
        ),
        _ => (AcknowledgmentRequirement::not_required(), None),
    };
    let reversibility = if matches!(
        class,
        OperationClass::Unsupported | OperationClass::Conflict | OperationClass::NoOp
    ) {
        Reversibility::NotApplicable
    } else {
        Reversibility::Reversible
    };
    Operation::new(
        operation_id(id),
        HarnessId::new("codex").unwrap(),
        resource_selector("plugin:tools"),
        semantics,
        class,
        reversibility,
        [],
        acknowledgment,
        attention,
    )
}

#[test]
fn plan_rejects_duplicate_unknown_self_and_cyclic_dependencies() {
    assert_eq!(
        Plan::new([safe_operation("one", &[]), safe_operation("one", &[])]).unwrap_err(),
        OperationContractError::DuplicateOperation {
            id: operation_id("one")
        }
    );
    assert_eq!(
        Plan::new([safe_operation("one", &["missing"])]).unwrap_err(),
        OperationContractError::UnknownDependency {
            operation: operation_id("one"),
            dependency: operation_id("missing")
        }
    );
    assert_eq!(
        Plan::new([safe_operation("one", &["one"])]).unwrap_err(),
        OperationContractError::SelfDependency {
            id: operation_id("one")
        }
    );
    assert_eq!(
        Plan::new([
            safe_operation("one", &["three"]),
            safe_operation("two", &["one"]),
            safe_operation("three", &["two"]),
            safe_operation("downstream", &["one"]),
        ])
        .unwrap_err(),
        OperationContractError::DependencyCycle {
            operations: [
                operation_id("one"),
                operation_id("two"),
                operation_id("three")
            ]
            .into_iter()
            .collect()
        }
    );
}

#[test]
fn plan_reports_all_disjoint_cycle_members_without_downstream_nodes() {
    let error = Plan::new([
        safe_operation("one", &["two"]),
        safe_operation("two", &["one"]),
        safe_operation("three", &["four"]),
        safe_operation("four", &["three"]),
        safe_operation("downstream", &["one"]),
    ])
    .unwrap_err();

    assert_eq!(
        error,
        OperationContractError::DependencyCycle {
            operations: [
                operation_id("one"),
                operation_id("two"),
                operation_id("three"),
                operation_id("four"),
            ]
            .into_iter()
            .collect(),
        }
    );
}

#[test]
fn partial_operations_require_exact_selectors_consequences_and_matching_attention() {
    assert_eq!(
        AcknowledgmentRequirement::required([], [consequence()]).unwrap_err(),
        OperationContractError::EmptyAcknowledgmentSelectors
    );
    assert_eq!(
        AcknowledgmentRequirement::required([resource_selector("plugin:tools")], []).unwrap_err(),
        OperationContractError::EmptyAcknowledgmentConsequences
    );

    let missing_ack = Operation::new(
        operation_id("partial"),
        HarnessId::new("codex").unwrap(),
        resource_selector("plugin:tools"),
        partial_semantics("codex", [consequence()]),
        OperationClass::Partial,
        Reversibility::Irreversible,
        [],
        AcknowledgmentRequirement::not_required(),
        None,
    )
    .unwrap_err();
    assert!(matches!(
        missing_ack,
        OperationContractError::InvalidOperationClassification { .. }
    ));

    let different_selectors = Operation::new(
        operation_id("partial"),
        HarnessId::new("codex").unwrap(),
        resource_selector("plugin:tools"),
        partial_semantics("codex", [consequence()]),
        OperationClass::Partial,
        Reversibility::Irreversible,
        [],
        AcknowledgmentRequirement::required([resource_selector("plugin:tools")], [consequence()])
            .unwrap(),
        Some(
            AttentionReason::acknowledgment_required(
                [component_selector("plugin:tools", "hook:format")],
                [consequence()],
            )
            .unwrap(),
        ),
    )
    .unwrap_err();
    assert_eq!(
        different_selectors,
        OperationContractError::AcknowledgmentAttentionMismatch {
            id: operation_id("partial")
        }
    );

    let outside = Operation::new(
        operation_id("component-partial"),
        HarnessId::new("codex").unwrap(),
        component_selector("plugin:tools", "hook:format"),
        partial_semantics("codex", [consequence()]),
        OperationClass::Partial,
        Reversibility::Irreversible,
        [],
        AcknowledgmentRequirement::required([resource_selector("plugin:tools")], [consequence()])
            .unwrap(),
        Some(
            AttentionReason::acknowledgment_required(
                [resource_selector("plugin:tools")],
                [consequence()],
            )
            .unwrap(),
        ),
    )
    .unwrap_err();
    assert!(matches!(
        outside,
        OperationContractError::AcknowledgmentSelectorOutsideOperation { .. }
    ));
}

#[test]
fn material_consequences_require_matching_acknowledged_components() {
    let resource_wide = MaterialConsequence::new(
        ConsequenceCode::new("component.omitted").unwrap(),
        [ComponentId::new("hook:other").unwrap()],
        ConsequenceSummary::new("Another hook will be omitted").unwrap(),
    );
    let selectors = [resource_selector("plugin:tools")];
    assert!(
        Operation::new(
            operation_id("resource-partial"),
            HarnessId::new("codex").unwrap(),
            resource_selector("plugin:tools"),
            partial_semantics("codex", [resource_wide.clone()]),
            OperationClass::Partial,
            Reversibility::Irreversible,
            [],
            AcknowledgmentRequirement::required(selectors.clone(), [resource_wide.clone()])
                .unwrap(),
            Some(AttentionReason::acknowledgment_required(selectors, [resource_wide]).unwrap()),
        )
        .is_ok()
    );

    let uncovered = MaterialConsequence::new(
        ConsequenceCode::new("component.omitted").unwrap(),
        [ComponentId::new("hook:other").unwrap()],
        ConsequenceSummary::new("Another hook will be omitted").unwrap(),
    );
    let selectors = [component_selector("plugin:tools", "hook:format")];
    let error = Operation::new(
        operation_id("partial"),
        HarnessId::new("codex").unwrap(),
        resource_selector("plugin:tools"),
        partial_semantics("codex", [uncovered.clone()]),
        OperationClass::Partial,
        Reversibility::Irreversible,
        [],
        AcknowledgmentRequirement::required(selectors.clone(), [uncovered.clone()]).unwrap(),
        Some(AttentionReason::acknowledgment_required(selectors, [uncovered]).unwrap()),
    )
    .unwrap_err();
    assert!(matches!(
        error,
        OperationContractError::UncoveredComponentConsequence { .. }
    ));

    let resource_consequence = MaterialConsequence::new(
        ConsequenceCode::new("resource.materialized").unwrap(),
        [],
        ConsequenceSummary::new("The plugin will become skilltap-owned").unwrap(),
    );
    let selectors = [component_selector("plugin:tools", "hook:format")];
    let error = Operation::new(
        operation_id("partial"),
        HarnessId::new("codex").unwrap(),
        component_selector("plugin:tools", "hook:format"),
        partial_semantics("codex", [resource_consequence.clone()]),
        OperationClass::Partial,
        Reversibility::Irreversible,
        [],
        AcknowledgmentRequirement::required(selectors.clone(), [resource_consequence.clone()])
            .unwrap(),
        Some(AttentionReason::acknowledgment_required(selectors, [resource_consequence]).unwrap()),
    )
    .unwrap_err();
    assert!(matches!(
        error,
        OperationContractError::UncoveredResourceConsequence { .. }
    ));

    let mut raw = serde_json::to_value(partial_operation("partial", &[]))
        .unwrap()
        .as_object()
        .unwrap()
        .clone();
    for field in ["acknowledgment", "attention"] {
        raw.get_mut(field)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .get_mut("consequences")
            .unwrap()
            .as_array_mut()
            .unwrap()[0]
            .as_object_mut()
            .unwrap()
            .insert(
                "affected_components".into(),
                serde_json::json!(["hook:other"]),
            );
    }
    assert!(serde_json::from_value::<Operation>(raw.into()).is_err());
}

#[test]
fn operation_class_attention_and_reversibility_combinations_are_validated() {
    let safe_with_attention = Operation::new(
        operation_id("safe"),
        HarnessId::new("codex").unwrap(),
        resource_selector("skill:portable"),
        semantics("codex"),
        OperationClass::SafeFaithfulEquivalent,
        Reversibility::Reversible,
        [],
        AcknowledgmentRequirement::not_required(),
        Some(AttentionReason::unsupported(
            EvidenceCode::new("native.unsupported").unwrap(),
            EvidenceDetail::new("The native operation is unavailable").unwrap(),
        )),
    )
    .unwrap_err();
    assert!(matches!(
        safe_with_attention,
        OperationContractError::InvalidOperationClassification { .. }
    ));

    let no_op = Operation::new(
        operation_id("noop"),
        HarnessId::new("codex").unwrap(),
        resource_selector("skill:portable"),
        semantics("codex"),
        OperationClass::NoOp,
        Reversibility::Reversible,
        [],
        AcknowledgmentRequirement::not_required(),
        None,
    )
    .unwrap_err();
    assert_eq!(
        no_op,
        OperationContractError::InvalidReversibility {
            id: operation_id("noop"),
            class: OperationClass::NoOp,
        }
    );

    let unsupported = Operation::new(
        operation_id("unsupported"),
        HarnessId::new("codex").unwrap(),
        resource_selector("skill:portable"),
        semantics_for_fidelity("codex", TransferFidelity::Blocked, [consequence()], []),
        OperationClass::Unsupported,
        Reversibility::Reversible,
        [],
        AcknowledgmentRequirement::not_required(),
        Some(AttentionReason::unsupported(
            EvidenceCode::new("native.unsupported").unwrap(),
            EvidenceDetail::new("The native operation is unavailable").unwrap(),
        )),
    )
    .unwrap_err();
    assert_eq!(
        unsupported,
        OperationContractError::InvalidReversibility {
            id: operation_id("unsupported"),
            class: OperationClass::Unsupported,
        }
    );
}

#[test]
fn executable_operations_require_affected_surfaces_at_both_boundaries() {
    let safe = Operation::new(
        operation_id("safe-empty"),
        HarnessId::new("codex").unwrap(),
        resource_selector("plugin:tools"),
        semantics_with("codex", []),
        OperationClass::SafeNative,
        Reversibility::Reversible,
        [],
        AcknowledgmentRequirement::not_required(),
        None,
    )
    .unwrap_err();
    assert!(matches!(
        safe,
        OperationContractError::EmptyAffectedSurfaces {
            class: OperationClass::SafeNative,
            ..
        }
    ));

    let selectors = [component_selector("plugin:tools", "hook:format")];
    let consequences = [consequence()];
    let partial = Operation::new(
        operation_id("partial-empty"),
        HarnessId::new("codex").unwrap(),
        resource_selector("plugin:tools"),
        semantics_for_fidelity("codex", TransferFidelity::Partial, consequences.clone(), []),
        OperationClass::Partial,
        Reversibility::Reversible,
        [],
        AcknowledgmentRequirement::required(selectors.clone(), consequences.clone()).unwrap(),
        Some(AttentionReason::acknowledgment_required(selectors, consequences).unwrap()),
    )
    .unwrap_err();
    assert!(matches!(
        partial,
        OperationContractError::EmptyAffectedSurfaces {
            class: OperationClass::Partial,
            ..
        }
    ));

    for mut raw in [
        serde_json::to_value(safe_operation("safe-wire", &[])).unwrap(),
        serde_json::to_value(partial_operation("partial-wire", &[])).unwrap(),
    ] {
        raw.as_object_mut().unwrap().remove("affected_surfaces");
        assert!(serde_json::from_value::<Operation>(raw).is_err());
    }
}

#[test]
fn every_operation_class_enforces_its_transfer_fidelity() {
    let classes = [
        OperationClass::SafeNative,
        OperationClass::SafeFaithfulEquivalent,
        OperationClass::SafeMaterialization,
        OperationClass::Partial,
        OperationClass::Unsupported,
        OperationClass::Conflict,
        OperationClass::NoOp,
    ];
    let fidelities = [
        TransferFidelity::Faithful,
        TransferFidelity::Materializable,
        TransferFidelity::Partial,
        TransferFidelity::Blocked,
    ];
    let valid = |class, fidelity| match class {
        OperationClass::SafeNative | OperationClass::SafeFaithfulEquivalent => {
            fidelity == TransferFidelity::Faithful
        }
        OperationClass::SafeMaterialization => fidelity == TransferFidelity::Materializable,
        OperationClass::Partial => fidelity == TransferFidelity::Partial,
        OperationClass::Unsupported | OperationClass::Conflict => {
            fidelity == TransferFidelity::Blocked
        }
        OperationClass::NoOp => fidelity != TransferFidelity::Blocked,
    };

    for class in classes {
        for fidelity in fidelities {
            let result = operation_with_class_and_fidelity("matrix", class, fidelity);
            assert_eq!(
                result.is_ok(),
                valid(class, fidelity),
                "{class:?}/{fidelity:?}"
            );

            if !valid(class, fidelity) {
                let valid_fidelity = match class {
                    OperationClass::SafeNative
                    | OperationClass::SafeFaithfulEquivalent
                    | OperationClass::NoOp => TransferFidelity::Faithful,
                    OperationClass::SafeMaterialization => TransferFidelity::Materializable,
                    OperationClass::Partial => TransferFidelity::Partial,
                    OperationClass::Unsupported | OperationClass::Conflict => {
                        TransferFidelity::Blocked
                    }
                };
                let mut raw = serde_json::to_value(
                    operation_with_class_and_fidelity("matrix-wire", class, valid_fidelity)
                        .unwrap(),
                )
                .unwrap();
                raw["compatibility"] = serde_json::to_value(
                    semantics_for_fidelity(
                        "codex",
                        fidelity,
                        (!fidelity.is_faithful()).then(consequence),
                        [],
                    )
                    .compatibility(),
                )
                .unwrap();
                assert!(serde_json::from_value::<Operation>(raw).is_err());
            }
        }
    }
}

#[test]
fn partial_acknowledgment_exactly_matches_compatibility_consequences() {
    let extra = MaterialConsequence::new(
        ConsequenceCode::new("component.behavior.changed").unwrap(),
        [ComponentId::new("hook:format").unwrap()],
        ConsequenceSummary::new("The formatting hook behavior will change").unwrap(),
    );
    let selectors = [component_selector("plugin:tools", "hook:format")];

    for (compatibility, acknowledged) in [
        (vec![consequence()], vec![consequence(), extra.clone()]),
        (vec![consequence(), extra.clone()], vec![consequence()]),
    ] {
        let error = Operation::new(
            operation_id("partial-mismatch"),
            HarnessId::new("codex").unwrap(),
            resource_selector("plugin:tools"),
            partial_semantics("codex", compatibility),
            OperationClass::Partial,
            Reversibility::Reversible,
            [],
            AcknowledgmentRequirement::required(selectors.clone(), acknowledged.clone()).unwrap(),
            Some(
                AttentionReason::acknowledgment_required(selectors.clone(), acknowledged).unwrap(),
            ),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            OperationContractError::PartialConsequenceMismatch { .. }
        ));
    }

    let mut invented = serde_json::to_value(partial_operation("invented", &[])).unwrap();
    for field in ["acknowledgment", "attention"] {
        invented[field]["consequences"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::to_value(&extra).unwrap());
    }
    assert!(serde_json::from_value::<Operation>(invented).is_err());

    let mut omitted = serde_json::to_value(partial_operation("omitted", &[])).unwrap();
    omitted["compatibility"]["consequences"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::to_value(extra).unwrap());
    assert!(serde_json::from_value::<Operation>(omitted).is_err());
}

#[test]
fn operation_and_attention_payloads_are_readable_without_serialization() {
    let operation = partial_operation("materialize", &[]);
    assert_eq!(operation.target(), &HarnessId::new("codex").unwrap());
    assert_eq!(operation.selector(), &resource_selector("plugin:tools"));
    assert_eq!(operation.class(), OperationClass::Partial);
    assert_eq!(operation.reversibility(), Reversibility::Reversible);
    assert!(operation.dependencies().is_empty());
    assert!(operation.acknowledgment().is_required());

    let attention = operation.attention().unwrap();
    assert_eq!(attention.kind(), AttentionKind::AcknowledgmentRequired);
    assert_eq!(
        attention.selectors(),
        operation.acknowledgment().selectors()
    );
    assert_eq!(
        attention.consequences(),
        operation.acknowledgment().consequences()
    );
    assert!(attention.code().is_none());
    assert!(attention.detail().is_none());
    assert!(attention.dependencies().is_none());

    let unsupported = AttentionReason::unsupported(
        EvidenceCode::new("native.unsupported").unwrap(),
        EvidenceDetail::new("No native mutation exists").unwrap(),
    );
    assert_eq!(unsupported.code().unwrap().as_str(), "native.unsupported");
    assert_eq!(
        unsupported.detail().unwrap().as_str(),
        "No native mutation exists"
    );
}

#[test]
fn semantic_payloads_are_target_bound_redactable_and_deterministic() {
    let surfaces = [
        AffectedSurface::native_command(
            HarnessId::new("codex").unwrap(),
            NativeId::new("codex").unwrap(),
            [
                CommandArgument::literal(NativeId::new("plugin").unwrap()),
                CommandArgument::redacted(),
            ],
        ),
        AffectedSurface::file(AbsolutePath::new("/home/user/.codex/config.toml").unwrap()),
    ];
    let operation = Operation::new(
        operation_id("install"),
        HarnessId::new("codex").unwrap(),
        resource_selector("plugin:tools"),
        semantics_with("codex", surfaces),
        OperationClass::SafeNative,
        Reversibility::Reversible,
        [],
        AcknowledgmentRequirement::not_required(),
        None,
    )
    .unwrap();

    assert_eq!(operation.action(), OperationAction::PluginInstall);
    assert_eq!(operation.scope(), &Scope::Global);
    assert_eq!(operation.reason().code().as_str(), "desired.plugin.missing");
    assert_eq!(operation.compatibility().target(), operation.target());
    assert_eq!(operation.provenance(), Provenance::Native);
    let command = operation
        .affected_surfaces()
        .iter()
        .find(|surface| surface.executable().is_some())
        .unwrap();
    assert_eq!(command.target(), Some(operation.target()));
    assert_eq!(command.executable().unwrap().as_str(), "codex");
    assert!(command.arguments().unwrap()[1].is_redacted());
    assert!(command.arguments().unwrap()[1].literal_value().is_none());

    let json = serde_json::to_string(&operation).unwrap();
    assert_eq!(serde_json::from_str::<Operation>(&json).unwrap(), operation);
    assert!(json.contains(r#""action":"plugin_install""#));
    assert!(json.contains(r#"{"kind":"redacted"}"#));
    assert!(
        json.find(r#""kind":"file""#).unwrap() < json.find(r#""kind":"native_command""#).unwrap()
    );
    assert!(
        serde_json::from_str::<CommandArgument>(
            r#"{"kind":"redacted","value":"must-not-survive"}"#
        )
        .is_err()
    );
}

#[test]
fn semantic_targets_are_validated_by_constructor_and_serde() {
    let mismatch = Operation::new(
        operation_id("install"),
        HarnessId::new("codex").unwrap(),
        resource_selector("plugin:tools"),
        semantics("claude"),
        OperationClass::SafeNative,
        Reversibility::Reversible,
        [],
        AcknowledgmentRequirement::not_required(),
        None,
    )
    .unwrap_err();
    assert!(matches!(
        mismatch,
        OperationContractError::CompatibilityTargetMismatch { .. }
    ));

    let command_mismatch = Operation::new(
        operation_id("install"),
        HarnessId::new("codex").unwrap(),
        resource_selector("plugin:tools"),
        semantics_with(
            "codex",
            [AffectedSurface::native_command(
                HarnessId::new("claude").unwrap(),
                NativeId::new("claude").unwrap(),
                [],
            )],
        ),
        OperationClass::SafeNative,
        Reversibility::Reversible,
        [],
        AcknowledgmentRequirement::not_required(),
        None,
    )
    .unwrap_err();
    assert!(matches!(
        command_mismatch,
        OperationContractError::AffectedSurfaceTargetMismatch { .. }
    ));

    let raw = serde_json::to_value(safe_operation("install", &[])).unwrap();
    let mut raw = raw.as_object().unwrap().clone();
    raw.get_mut("compatibility")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .insert("target".into(), serde_json::json!("codex"));
    assert!(serde_json::from_value::<Operation>(raw.into()).is_err());

    let command = AffectedSurface::native_command(
        HarnessId::new("codex").unwrap(),
        NativeId::new("codex").unwrap(),
        [],
    );
    let operation = Operation::new(
        operation_id("install"),
        HarnessId::new("codex").unwrap(),
        resource_selector("plugin:tools"),
        semantics_with("codex", [command]),
        OperationClass::SafeNative,
        Reversibility::Reversible,
        [],
        AcknowledgmentRequirement::not_required(),
        None,
    )
    .unwrap();
    let mut raw = serde_json::to_value(operation).unwrap();
    raw["affected_surfaces"][0]["target"] = serde_json::json!("claude");
    assert!(serde_json::from_value::<Operation>(raw).is_err());
}

#[test]
fn constructor_and_deserialization_enforce_plan_invariants() {
    let mut invalid =
        serde_json::to_value(Plan::new([safe_operation("one", &[])]).unwrap()).unwrap();
    invalid["operations"][0]["dependencies"] = serde_json::json!([{"operation_id": "missing"}]);
    let error = serde_json::from_value::<Plan>(invalid).unwrap_err();
    assert!(error.to_string().contains("unknown operation `missing`"));

    let empty_ack = r#"{"kind":"required","selectors":[],"consequences":[]}"#;
    assert!(serde_json::from_str::<AcknowledgmentRequirement>(empty_ack).is_err());

    let unknown_field = r#"{"kind":"not_required","yes":true}"#;
    assert!(serde_json::from_str::<AcknowledgmentRequirement>(unknown_field).is_err());

    let mut invalid_reversibility = serde_json::to_value(blocked_operation(
        "unsupported",
        OperationClass::Unsupported,
    ))
    .unwrap();
    invalid_reversibility["reversibility"] = serde_json::json!("reversible");
    assert!(serde_json::from_value::<Operation>(invalid_reversibility).is_err());

    let mut outside_selector = serde_json::to_value(partial_operation("partial", &[])).unwrap();
    outside_selector["selector"] = serde_json::json!({
        "kind": "component",
        "resource_id": "plugin:tools",
        "component_id": "hook:format"
    });
    for field in ["acknowledgment", "attention"] {
        outside_selector[field]["selectors"] = serde_json::json!([{
            "kind": "resource",
            "resource_id": "plugin:tools"
        }]);
    }
    assert!(serde_json::from_value::<Operation>(outside_selector).is_err());
}

#[test]
fn operation_results_validate_attention_kinds_at_both_boundaries() {
    let unsupported = AttentionReason::unsupported(
        EvidenceCode::new("native.unsupported").unwrap(),
        EvidenceDetail::new("No native mutation exists").unwrap(),
    );
    assert!(matches!(
        OperationResult::new(
            operation_id("one"),
            OperationOutcome::Failed {
                reason: unsupported
            }
        ),
        Err(OperationContractError::InvalidFailedAttention { .. })
    ));

    let failed = AttentionReason::operation_failed(
        EvidenceCode::new("native.command.failed").unwrap(),
        EvidenceDetail::new("Native command failed").unwrap(),
    );
    assert!(matches!(
        OperationResult::new(
            operation_id("one"),
            OperationOutcome::Blocked { reason: failed }
        ),
        Err(OperationContractError::InvalidBlockedAttention { .. })
    ));

    let failed_with_unsupported = r#"{"operation_id":"one","outcome":{"status":"failed","reason":{"kind":"unsupported","code":"native.unsupported","detail":"No native mutation exists"}}}"#;
    assert!(serde_json::from_str::<OperationResult>(failed_with_unsupported).is_err());
    let blocked_with_failure = r#"{"operation_id":"one","outcome":{"status":"blocked","reason":{"kind":"operation_failed","code":"native.command.failed","detail":"Native command failed"}}}"#;
    assert!(serde_json::from_str::<OperationResult>(blocked_with_failure).is_err());
}

#[test]
fn representative_plan_serializes_deterministically_and_round_trips() {
    let plan = Plan::new([
        partial_operation("materialize", &["register"]),
        safe_operation("register", &[]),
    ])
    .unwrap();
    let json = serde_json::to_string(&plan).unwrap();

    assert_eq!(serde_json::from_str::<Plan>(&json).unwrap(), plan);
    assert!(json.find(r#""id":"materialize""#).unwrap() < json.find(r#""id":"register""#).unwrap());
    assert!(json.contains(r#""class":"partial""#));
    assert!(json.contains(r#""kind":"component""#));
    assert!(!json.contains("yes"));

    assert_eq!(
        serde_json::to_string(&OperationClass::SafeNative).unwrap(),
        r#""safe_native""#
    );
    assert_eq!(
        serde_json::to_string(&component_selector("plugin:tools", "hook:format")).unwrap(),
        r#"{"kind":"component","resource_id":"plugin:tools","component_id":"hook:format"}"#
    );
}

#[test]
fn dependent_skips_are_distinct_and_final_success_rejects_unfinished_work() {
    let plan = Plan::new([
        safe_operation("register", &[]),
        safe_operation("materialize", &["register"]),
    ])
    .unwrap();
    let failure = OperationResult::new(
        operation_id("register"),
        OperationOutcome::Failed {
            reason: AttentionReason::operation_failed(
                EvidenceCode::new("native.command.failed").unwrap(),
                EvidenceDetail::new("Native command exited with status 1").unwrap(),
            ),
        },
    )
    .unwrap();
    let skipped = OperationResult::new(
        operation_id("materialize"),
        OperationOutcome::SkippedDependency {
            dependencies: [operation_id("register")].into_iter().collect(),
        },
    )
    .unwrap();

    assert!(matches!(
        skipped.outcome(),
        OperationOutcome::SkippedDependency { .. }
    ));
    assert!(matches!(
        ApplyResult::new(
            plan.clone(),
            ApplyOutcome::Succeeded,
            [failure.clone(), skipped.clone()]
        ),
        Err(OperationContractError::InconsistentApplyOutcome { .. })
    ));
    assert!(matches!(
        ApplyResult::new(plan, ApplyOutcome::AttentionRequired, [failure, skipped]),
        Err(OperationContractError::InconsistentApplyOutcome { .. })
    ));
}

#[test]
fn apply_results_enforce_plan_class_outcomes() {
    for class in [OperationClass::Unsupported, OperationClass::Conflict] {
        let operation = blocked_operation("blocked", class);
        let error = ApplyResult::new(
            Plan::new([operation.clone()]).unwrap(),
            ApplyOutcome::Succeeded,
            [OperationResult::new(operation_id("blocked"), OperationOutcome::Applied).unwrap()],
        )
        .unwrap_err();
        assert_eq!(
            error,
            OperationContractError::InvalidOutcomeForOperationClass {
                operation: operation_id("blocked"),
                class,
            }
        );

        let wrong_reason = match class {
            OperationClass::Unsupported => AttentionReason::conflict(
                EvidenceCode::new("native.conflict").unwrap(),
                EvidenceDetail::new("An unmanaged resource conflicts").unwrap(),
            ),
            OperationClass::Conflict => AttentionReason::unsupported(
                EvidenceCode::new("native.unsupported").unwrap(),
                EvidenceDetail::new("No native operation exists").unwrap(),
            ),
            _ => unreachable!(),
        };
        assert!(matches!(
            ApplyResult::new(
                Plan::new([operation]).unwrap(),
                ApplyOutcome::AttentionRequired,
                [OperationResult::new(
                    operation_id("blocked"),
                    OperationOutcome::Blocked {
                        reason: wrong_reason
                    }
                )
                .unwrap()]
            ),
            Err(OperationContractError::InvalidOutcomeForOperationClass { .. })
        ));
    }

    let error = ApplyResult::new(
        Plan::new([no_op("noop")]).unwrap(),
        ApplyOutcome::Succeeded,
        [OperationResult::new(operation_id("noop"), OperationOutcome::Applied).unwrap()],
    )
    .unwrap_err();
    assert_eq!(
        error,
        OperationContractError::InvalidOutcomeForOperationClass {
            operation: operation_id("noop"),
            class: OperationClass::NoOp,
        }
    );

    let valid = ApplyResult::new(
        Plan::new([no_op("noop")]).unwrap(),
        ApplyOutcome::Succeeded,
        [OperationResult::new(operation_id("noop"), OperationOutcome::NoChange).unwrap()],
    )
    .unwrap();
    let mut raw = serde_json::to_value(valid).unwrap();
    raw["operations"][0]["outcome"] = serde_json::json!({"status": "applied"});
    assert!(serde_json::from_value::<ApplyResult>(raw).is_err());
}

#[test]
fn dependency_blockers_require_an_exact_skip_set() {
    let plan = Plan::new([
        safe_operation("pending", &[]),
        safe_operation("failed", &[]),
        safe_operation("dependent", &["pending", "failed"]),
    ])
    .unwrap();
    let pending = OperationResult::new(operation_id("pending"), OperationOutcome::Pending).unwrap();
    let failed = OperationResult::new(
        operation_id("failed"),
        OperationOutcome::Failed {
            reason: AttentionReason::operation_failed(
                EvidenceCode::new("native.command.failed").unwrap(),
                EvidenceDetail::new("Native command failed").unwrap(),
            ),
        },
    )
    .unwrap();

    let error = ApplyResult::new(
        plan.clone(),
        ApplyOutcome::PartialFailure,
        [
            pending.clone(),
            failed.clone(),
            OperationResult::new(operation_id("dependent"), OperationOutcome::Applied).unwrap(),
        ],
    )
    .unwrap_err();
    assert!(matches!(
        error,
        OperationContractError::DependencyBlockersRequireSkip { .. }
    ));

    let error = ApplyResult::new(
        plan.clone(),
        ApplyOutcome::PartialFailure,
        [
            pending.clone(),
            failed.clone(),
            OperationResult::new(
                operation_id("dependent"),
                OperationOutcome::SkippedDependency {
                    dependencies: [operation_id("failed")].into_iter().collect(),
                },
            )
            .unwrap(),
        ],
    )
    .unwrap_err();
    assert!(matches!(
        error,
        OperationContractError::DependencySkipMismatch { .. }
    ));

    let result = ApplyResult::new(
        plan,
        ApplyOutcome::PartialFailure,
        [
            pending,
            failed,
            OperationResult::new(
                operation_id("dependent"),
                OperationOutcome::SkippedDependency {
                    dependencies: [operation_id("failed"), operation_id("pending")]
                        .into_iter()
                        .collect(),
                },
            )
            .unwrap(),
        ],
    )
    .unwrap();
    let json = serde_json::to_string(&result).unwrap();
    assert_eq!(serde_json::from_str::<ApplyResult>(&json).unwrap(), result);

    let mut raw = serde_json::to_value(result).unwrap();
    let dependent = raw["operations"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|result| result["operation_id"] == "dependent")
        .unwrap();
    dependent["outcome"] = serde_json::json!({"status": "applied"});
    assert!(serde_json::from_value::<ApplyResult>(raw).is_err());
}

#[test]
fn partial_apply_result_is_deterministic_and_round_trips() {
    let plan = Plan::new([
        safe_operation("z-pending", &[]),
        safe_operation("a-applied", &[]),
        safe_operation("m-blocked", &[]),
    ])
    .unwrap();
    let result = ApplyResult::new(
        plan,
        ApplyOutcome::AttentionRequired,
        [
            OperationResult::new(operation_id("z-pending"), OperationOutcome::Pending).unwrap(),
            OperationResult::new(operation_id("a-applied"), OperationOutcome::Applied).unwrap(),
            OperationResult::new(
                operation_id("m-blocked"),
                OperationOutcome::Blocked {
                    reason: AttentionReason::unsupported(
                        EvidenceCode::new("native.operation.unsupported").unwrap(),
                        EvidenceDetail::new("No deterministic native mutation is available")
                            .unwrap(),
                    ),
                },
            )
            .unwrap(),
        ],
    )
    .unwrap();
    let json = serde_json::to_string(&result).unwrap();

    assert_eq!(serde_json::from_str::<ApplyResult>(&json).unwrap(), result);
    assert!(json.find("a-applied").unwrap() < json.find("m-blocked").unwrap());
    assert!(json.find("m-blocked").unwrap() < json.find("z-pending").unwrap());
    assert!(json.contains(r#""status":"blocked""#));
    assert!(json.contains(r#""status":"pending""#));
}

#[test]
fn result_deserialization_rejects_invalid_summaries_and_skips() {
    let pending = ApplyResult::new(
        Plan::new([safe_operation("one", &[])]).unwrap(),
        ApplyOutcome::AttentionRequired,
        [OperationResult::new(operation_id("one"), OperationOutcome::Pending).unwrap()],
    )
    .unwrap();
    let mut false_success = serde_json::to_value(pending).unwrap();
    false_success["outcome"] = serde_json::json!("succeeded");
    assert!(serde_json::from_value::<ApplyResult>(false_success).is_err());

    let empty_skip =
        r#"{"operation_id":"one","outcome":{"status":"skipped_dependency","dependencies":[]}}"#;
    assert!(serde_json::from_str::<OperationResult>(empty_skip).is_err());

    let self_skip = r#"{"operation_id":"one","outcome":{"status":"skipped_dependency","dependencies":["one"]}}"#;
    assert!(serde_json::from_str::<OperationResult>(self_skip).is_err());

    let unknown =
        r#"{"plan":{"operations":[]},"outcome":"succeeded","operations":[],"assume_ok":true}"#;
    assert!(serde_json::from_str::<ApplyResult>(unknown).is_err());

    let unknown_dependency = OperationResult::new(
        operation_id("two"),
        OperationOutcome::SkippedDependency {
            dependencies: [operation_id("missing")].into_iter().collect(),
        },
    )
    .unwrap();
    assert_eq!(
        ApplyResult::new(
            Plan::new([safe_operation("two", &[])]).unwrap(),
            ApplyOutcome::AttentionRequired,
            [unknown_dependency]
        )
        .unwrap_err(),
        OperationContractError::UndeclaredSkippedDependency {
            operation: operation_id("two"),
            dependency: operation_id("missing")
        }
    );

    assert_eq!(
        ApplyResult::new(
            Plan::new([safe_operation("one", &[])]).unwrap(),
            ApplyOutcome::Succeeded,
            [],
        )
        .unwrap_err(),
        OperationContractError::MissingOperationResult {
            id: operation_id("one")
        }
    );
    assert_eq!(
        ApplyResult::new(
            Plan::new([]).unwrap(),
            ApplyOutcome::Succeeded,
            [OperationResult::new(operation_id("extra"), OperationOutcome::Applied).unwrap()],
        )
        .unwrap_err(),
        OperationContractError::UnknownOperationResult {
            id: operation_id("extra")
        }
    );

    let missing_persisted = serde_json::json!({
        "plan": serde_json::to_value(Plan::new([safe_operation("one", &[])]).unwrap())
            .unwrap(),
        "outcome": "succeeded",
        "operations": []
    });
    assert!(serde_json::from_value::<ApplyResult>(missing_persisted).is_err());
}
