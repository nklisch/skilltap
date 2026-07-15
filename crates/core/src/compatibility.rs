//! Pure target-bound cross-harness compatibility analysis.

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    domain::{
        CapabilityId, CapabilitySet, CapabilitySupport, CompatibilityClass, CompatibilityError,
        CompatibilityEvidence, CompatibilityResult, ComponentId, ComponentKind,
        ComponentRequiredness, ConsequenceCode, ConsequenceSummary, HarnessId, MaterialConsequence,
        OperationSelector, ResourceComponent, ResourceKey, TransferFidelity,
    },
    plugin_graph::SourceComponentGraph,
};

/// Inputs to one target-bound compatibility analysis.
pub struct CompatibilityRequest<'a> {
    pub resource: &'a ResourceKey,
    pub graph: &'a SourceComponentGraph,
    pub target: &'a HarnessId,
    pub capabilities: &'a CapabilitySet,
    pub occupied: &'a BTreeSet<ComponentId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompatibilityAnalysisError {
    InvalidResult(CompatibilityError),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentDecision {
    pub component: ComponentId,
    pub requiredness: ComponentRequiredness,
    pub result: CompatibilityResult,
    pub selector: OperationSelector,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompatibilityAnalysis {
    pub target: HarnessId,
    pub resource: ResourceKey,
    pub aggregate: CompatibilityResult,
    pub components: BTreeMap<ComponentId, ComponentDecision>,
    pub acknowledgment_selectors: BTreeSet<OperationSelector>,
}

/// The single source of truth for normalized component capability IDs.
const COMPONENT_CAPABILITY_RULES: &[(&str, &str)] = &[
    ("skill", "component.skill"),
    ("mcp_server", "component.mcp"),
    ("hook", "component.hook"),
    ("agent", "component.agent"),
    ("app", "component.app"),
    ("connector", "component.connector"),
    ("lsp_server", "component.lsp"),
    ("command", "component.command"),
    ("output_style", "component.output_style"),
    ("theme", "component.theme"),
    ("monitor", "component.monitor"),
    ("executable", "component.executable"),
    ("settings", "component.settings"),
];

pub fn analyze_component(
    request: &CompatibilityRequest<'_>,
    component: &ResourceComponent,
) -> Result<ComponentDecision, CompatibilityAnalysisError> {
    let selector = OperationSelector::Component {
        resource: request.resource.clone(),
        component_id: component.id.clone(),
    };
    let (compatibility, fidelity, evidence, consequences) =
        if request.occupied.contains(&component.id) {
            (
                CompatibilityClass::Incompatible,
                TransferFidelity::Blocked,
                vec![evidence(
                    "component.identity_conflict",
                    request.target,
                    &component.id,
                    "The target already owns this behavior-bearing component identity.",
                )],
                vec![consequence(
                    "component.identity_conflict",
                    &component.id,
                    "The component cannot be renamed or replaced across harnesses.",
                )],
            )
        } else {
            match capability_for(&component.kind)
                .and_then(|capability| request.capabilities.support(&capability))
            {
                Some(CapabilitySupport::Supported) => (
                    CompatibilityClass::Compatible,
                    TransferFidelity::Faithful,
                    Vec::new(),
                    Vec::new(),
                ),
                support => lost_component_result(request, component, support),
            }
        };
    let result = CompatibilityResult::new(
        request.target.clone(),
        compatibility,
        fidelity,
        evidence,
        consequences,
    )
    .map_err(CompatibilityAnalysisError::InvalidResult)?;
    Ok(ComponentDecision {
        component: component.id.clone(),
        requiredness: component.requiredness,
        result,
        selector,
    })
}

pub fn analyze(
    request: CompatibilityRequest<'_>,
) -> Result<CompatibilityAnalysis, CompatibilityAnalysisError> {
    let mut components = request
        .graph
        .components()
        .iter()
        .map(|(id, component)| Ok((id.clone(), analyze_component(&request, component)?)))
        .collect::<Result<BTreeMap<_, _>, CompatibilityAnalysisError>>()?;

    // Dependency loss is monotonic: repeatedly promote a dependent until all
    // dependency consequences are represented in its own result.
    let mut changed = true;
    while changed {
        changed = false;
        for (id, component) in request.graph.components().iter() {
            let dependency_loss = component.dependencies.iter().any(|dependency| {
                components.get(dependency).is_some_and(|decision| {
                    decision.result.fidelity() != TransferFidelity::Faithful
                })
            });
            if !dependency_loss {
                continue;
            }
            let Some(current) = components.get(id) else {
                continue;
            };
            let desired_fidelity = match component.requiredness {
                ComponentRequiredness::Required => TransferFidelity::Blocked,
                ComponentRequiredness::Optional => TransferFidelity::Partial,
            };
            let dependency_evidence_present = current
                .result
                .evidence()
                .iter()
                .any(|item| item.code.as_str() == "component.dependency_unavailable");
            let needs_promotion =
                fidelity_rank(desired_fidelity) > fidelity_rank(current.result.fidelity());
            if !needs_promotion && dependency_evidence_present {
                continue;
            }
            let mut evidence_items = current
                .result
                .evidence()
                .iter()
                .cloned()
                .collect::<Vec<_>>();
            evidence_items.push(evidence(
                "component.dependency_unavailable",
                request.target,
                id,
                "A required dependency cannot be represented faithfully on the target.",
            ));
            let mut consequence_items = current
                .result
                .consequences()
                .iter()
                .cloned()
                .collect::<Vec<_>>();
            consequence_items.push(consequence(
                "component.dependency_unavailable",
                id,
                "The component's dependency behavior cannot be preserved.",
            ));
            let fidelity = if needs_promotion {
                desired_fidelity
            } else {
                current.result.fidelity()
            };
            let compatibility = if fidelity == TransferFidelity::Blocked {
                CompatibilityClass::Incompatible
            } else {
                CompatibilityClass::TargetSpecific
            };
            let result = CompatibilityResult::new(
                request.target.clone(),
                compatibility,
                fidelity,
                evidence_items,
                consequence_items,
            )
            .map_err(CompatibilityAnalysisError::InvalidResult)?;
            components.insert(
                id.clone(),
                ComponentDecision {
                    component: id.clone(),
                    requiredness: current.requiredness,
                    result,
                    selector: current.selector.clone(),
                },
            );
            changed = true;
        }
    }

    let mut evidence = BTreeSet::new();
    let mut consequences = BTreeSet::new();
    let mut blocked = false;
    let mut partial = false;
    let mut acknowledgment_selectors = BTreeSet::new();
    for decision in components.values() {
        evidence.extend(decision.result.evidence().iter().cloned());
        consequences.extend(decision.result.consequences().iter().cloned());
        match decision.result.fidelity() {
            TransferFidelity::Blocked => blocked = true,
            TransferFidelity::Partial => {
                partial = true;
                acknowledgment_selectors.insert(decision.selector.clone());
            }
            TransferFidelity::Materializable => {
                acknowledgment_selectors.insert(decision.selector.clone());
            }
            TransferFidelity::Faithful => {}
        }
    }
    let (compatibility, fidelity) = if blocked {
        (CompatibilityClass::Incompatible, TransferFidelity::Blocked)
    } else if partial {
        (
            CompatibilityClass::TargetSpecific,
            TransferFidelity::Partial,
        )
    } else {
        (CompatibilityClass::Compatible, TransferFidelity::Faithful)
    };
    let aggregate = CompatibilityResult::new(
        request.target.clone(),
        compatibility,
        fidelity,
        evidence,
        consequences,
    )
    .map_err(CompatibilityAnalysisError::InvalidResult)?;
    Ok(CompatibilityAnalysis {
        target: request.target.clone(),
        resource: request.resource.clone(),
        aggregate,
        components,
        acknowledgment_selectors,
    })
}

fn lost_component_result(
    request: &CompatibilityRequest<'_>,
    component: &ResourceComponent,
    support: Option<CapabilitySupport>,
) -> (
    CompatibilityClass,
    TransferFidelity,
    Vec<CompatibilityEvidence>,
    Vec<MaterialConsequence>,
) {
    let (code, detail) = match support {
        Some(CapabilitySupport::Unsupported) => (
            "component.capability_unsupported",
            "The target explicitly does not support this component kind.",
        ),
        Some(CapabilitySupport::Unverified) => (
            "component.capability_unverified",
            "The target capability is not verified for this harness version.",
        ),
        Some(CapabilitySupport::Supported) => unreachable!("supported is handled above"),
        None => (
            "component.capability_unknown",
            "The target has no verified capability evidence for this component kind.",
        ),
    };
    let blocked = component.requiredness == ComponentRequiredness::Required;
    (
        if blocked {
            CompatibilityClass::Incompatible
        } else {
            CompatibilityClass::TargetSpecific
        },
        if blocked {
            TransferFidelity::Blocked
        } else {
            TransferFidelity::Partial
        },
        vec![evidence(code, request.target, &component.id, detail)],
        vec![consequence(
            if blocked {
                "component.required_unsupported"
            } else {
                "component.optional_omitted"
            },
            &component.id,
            if blocked {
                "A required component blocks this target transfer."
            } else {
                "An optional component would be omitted on this target."
            },
        )],
    )
}

/// Map a normalized component kind to its one declared capability id.
///
/// This registry is shared by compatibility and mutation-authority planning;
/// callers must not duplicate the mapping in adapters or the CLI.
pub fn capability_for(kind: &ComponentKind) -> Option<CapabilityId> {
    let name = match kind {
        ComponentKind::Skill => "skill",
        ComponentKind::McpServer => "mcp_server",
        ComponentKind::Hook => "hook",
        ComponentKind::Agent => "agent",
        ComponentKind::App => "app",
        ComponentKind::Connector => "connector",
        ComponentKind::LspServer => "lsp_server",
        ComponentKind::Command => "command",
        ComponentKind::OutputStyle => "output_style",
        ComponentKind::Theme => "theme",
        ComponentKind::Monitor => "monitor",
        ComponentKind::Executable => "executable",
        ComponentKind::Settings => "settings",
        ComponentKind::HarnessSpecific(_) => return None,
    };
    let capability = COMPONENT_CAPABILITY_RULES
        .iter()
        .find(|(kind_name, _)| *kind_name == name)
        .map(|(_, capability)| *capability)?;
    CapabilityId::new(capability).ok()
}

fn fidelity_rank(fidelity: TransferFidelity) -> u8 {
    match fidelity {
        TransferFidelity::Faithful => 0,
        TransferFidelity::Materializable => 1,
        TransferFidelity::Partial => 2,
        TransferFidelity::Blocked => 3,
    }
}

fn evidence(
    code: &'static str,
    target: &HarnessId,
    component: &ComponentId,
    detail: &'static str,
) -> CompatibilityEvidence {
    CompatibilityEvidence::new(
        crate::domain::EvidenceCode::new(code).expect("static compatibility evidence code"),
        target.clone(),
        [component.clone()],
        crate::domain::EvidenceDetail::new(detail).expect("static compatibility evidence detail"),
    )
}

fn consequence(
    code: &'static str,
    component: &ComponentId,
    summary: &'static str,
) -> MaterialConsequence {
    MaterialConsequence::new(
        ConsequenceCode::new(code).expect("static compatibility consequence code"),
        [component.clone()],
        ConsequenceSummary::new(summary).expect("static compatibility consequence summary"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{ResourceId, Scope, Source, SourceKind, SourceLocator},
        plugin_graph::{ComponentDeclaration, normalize},
    };

    fn id(value: &str) -> ComponentId {
        ComponentId::new(value).unwrap()
    }

    fn graph(components: impl IntoIterator<Item = ComponentDeclaration>) -> SourceComponentGraph {
        normalize(
            Source::new(
                SourceKind::Git,
                SourceLocator::new("https://example.test/plugin.git").unwrap(),
                None,
            )
            .unwrap(),
            components,
        )
        .unwrap()
    }

    fn component(
        name: &str,
        kind: ComponentKind,
        requiredness: ComponentRequiredness,
        dependencies: &[&str],
    ) -> ComponentDeclaration {
        ComponentDeclaration {
            id: id(name),
            kind,
            requiredness,
            dependencies: dependencies.iter().map(|value| id(value)).collect(),
            relative_path: crate::domain::RelativeArtifactPath::new(format!("components/{name}"))
                .unwrap(),
            declared_name: Some(name.to_owned()),
        }
    }

    fn request<'a>(
        resource: &'a ResourceKey,
        graph: &'a SourceComponentGraph,
        capabilities: &'a CapabilitySet,
        occupied: &'a BTreeSet<ComponentId>,
        target: &'a HarnessId,
    ) -> CompatibilityRequest<'a> {
        CompatibilityRequest {
            resource,
            graph,
            target,
            capabilities,
            occupied,
        }
    }

    #[test]
    fn required_and_optional_capability_loss_have_distinct_fidelity() {
        let graph = graph([
            component(
                "skill:required",
                ComponentKind::Skill,
                ComponentRequiredness::Required,
                &[],
            ),
            component(
                "hook:optional",
                ComponentKind::Hook,
                ComponentRequiredness::Optional,
                &[],
            ),
        ]);
        let resource = ResourceKey::new(ResourceId::new("plugin:test").unwrap(), Scope::Global);
        let capabilities = CapabilitySet::new([]);
        let occupied = BTreeSet::new();
        let target = HarnessId::new("codex").unwrap();
        let request = request(&resource, &graph, &capabilities, &occupied, &target);
        let analysis = analyze(request).unwrap();
        assert_eq!(analysis.aggregate.fidelity(), TransferFidelity::Blocked);
        assert_eq!(
            analysis.components[&id("skill:required")].result.fidelity(),
            TransferFidelity::Blocked
        );
        assert_eq!(
            analysis.components[&id("hook:optional")].result.fidelity(),
            TransferFidelity::Partial
        );
        assert!(
            analysis
                .acknowledgment_selectors
                .contains(&OperationSelector::Component {
                    resource,
                    component_id: id("hook:optional"),
                })
        );
    }

    #[test]
    fn supported_component_and_collision_are_target_bound() {
        let graph = graph([component(
            "skill:demo",
            ComponentKind::Skill,
            ComponentRequiredness::Required,
            &[],
        )]);
        let resource = ResourceKey::new(ResourceId::new("plugin:test").unwrap(), Scope::Global);
        let capabilities = CapabilitySet::new([(
            CapabilityId::new("component.skill").unwrap(),
            CapabilitySupport::Supported,
        )]);
        let occupied = [id("skill:demo")].into_iter().collect();
        let target = HarnessId::new("codex").unwrap();
        let analysis = analyze(request(
            &resource,
            &graph,
            &capabilities,
            &occupied,
            &target,
        ))
        .unwrap();
        assert_eq!(
            analysis.aggregate.compatibility(),
            CompatibilityClass::Incompatible
        );
        assert_eq!(
            analysis.aggregate.target(),
            &HarnessId::new("codex").unwrap()
        );
        assert!(analysis.acknowledgment_selectors.is_empty());
    }

    #[test]
    fn dependency_loss_propagates_deterministically() {
        let graph = graph([
            component(
                "hook:optional",
                ComponentKind::Hook,
                ComponentRequiredness::Optional,
                &[],
            ),
            component(
                "skill:dependent",
                ComponentKind::Skill,
                ComponentRequiredness::Required,
                &["hook:optional"],
            ),
        ]);
        let resource = ResourceKey::new(
            ResourceId::new("plugin:test").unwrap(),
            Scope::Project(crate::domain::AbsolutePath::new("/work/project").unwrap()),
        );
        let capabilities = CapabilitySet::new([(
            CapabilityId::new("component.skill").unwrap(),
            CapabilitySupport::Supported,
        )]);
        let target = HarnessId::new("codex").unwrap();
        let analysis = analyze(request(
            &resource,
            &graph,
            &capabilities,
            &BTreeSet::new(),
            &target,
        ))
        .unwrap();
        assert_eq!(
            analysis.components[&id("skill:dependent")]
                .result
                .fidelity(),
            TransferFidelity::Blocked
        );
        assert!(
            analysis.components[&id("skill:dependent")]
                .result
                .evidence()
                .iter()
                .any(|item| item.code.as_str() == "component.dependency_unavailable")
        );
    }

    #[test]
    fn update_change_summary_counts_new_required_and_partial_components() {
        let resource = ResourceKey::new(ResourceId::new("plugin:test").unwrap(), Scope::Global);
        let target = HarnessId::new("codex").unwrap();
        let capabilities = CapabilitySet::new([(
            CapabilityId::new("component.skill").unwrap(),
            CapabilitySupport::Supported,
        )]);
        let before_graph = graph([component(
            "skill:demo",
            ComponentKind::Skill,
            ComponentRequiredness::Required,
            &[],
        )]);
        let after_graph = graph([
            component(
                "skill:demo",
                ComponentKind::Skill,
                ComponentRequiredness::Required,
                &[],
            ),
            component(
                "hook:required",
                ComponentKind::Hook,
                ComponentRequiredness::Required,
                &[],
            ),
            component(
                "hook:optional",
                ComponentKind::Hook,
                ComponentRequiredness::Optional,
                &[],
            ),
        ]);
        let before = analyze(request(
            &resource,
            &before_graph,
            &capabilities,
            &BTreeSet::new(),
            &target,
        ))
        .unwrap();
        let after = analyze(request(
            &resource,
            &after_graph,
            &capabilities,
            &BTreeSet::new(),
            &target,
        ))
        .unwrap();
        let summary = crate::updates::update_change_summary(&before, &after);
        assert_eq!(summary.added_required_components, 1);
        assert_eq!(summary.partial_components, 1);
        assert!(summary.compatibility_changed);
        assert!(summary.requires_acknowledgment);
    }
}
