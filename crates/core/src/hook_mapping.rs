//! Normalized hook contracts and target-bound equivalence analysis.

use std::collections::BTreeSet;

use crate::{
    domain::{
        CompatibilityClass, CompatibilityError, CompatibilityEvidence, CompatibilityResult,
        ComponentId, ComponentRequiredness, ConsequenceCode, ConsequenceSummary, EvidenceCode,
        EvidenceDetail, HarnessId, MaterialConsequence, ResourceKey, TransferFidelity,
    },
    plugin_graph::SourceComponentGraph,
};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum HookPayload {
    Json,
    Text,
    Unknown(String),
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum HookFailure {
    Continue,
    Block,
    Unknown(String),
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum HookWorkingDirectory {
    Plugin,
    Project,
    Inherited,
    Unknown(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HookContract {
    pub component: ComponentId,
    pub event: String,
    pub payload: HookPayload,
    pub failure: HookFailure,
    pub working_directory: HookWorkingDirectory,
    pub environment_references: BTreeSet<String>,
    pub executable: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HookTargetContract {
    pub event: String,
    pub payload: HookPayload,
    pub failure: HookFailure,
    pub working_directory: HookWorkingDirectory,
    pub environment_references: BTreeSet<String>,
    pub executable: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HookMappingError {
    InvalidCompatibility(CompatibilityError),
    UnsupportedHook {
        component: ComponentId,
        reason: &'static str,
    },
}

pub trait HookContractReader {
    fn read(
        &self,
        graph: &SourceComponentGraph,
        component: &ComponentId,
    ) -> Result<HookContract, HookMappingError>;
}

pub fn analyze_hook(
    source: &HookContract,
    target: &HookTargetContract,
    requiredness: ComponentRequiredness,
    target_harness: &HarnessId,
    _resource: &ResourceKey,
) -> Result<CompatibilityResult, HookMappingError> {
    let mut evidence = Vec::new();
    let mut consequences = Vec::new();
    compare_field(
        source,
        target_harness,
        &mut evidence,
        &mut consequences,
        source.event == target.event,
        "hook.event_mismatch",
        "Hook lifecycle events are not equivalent.",
    );
    compare_field(
        source,
        target_harness,
        &mut evidence,
        &mut consequences,
        source.payload == target.payload,
        "hook.payload_mismatch",
        "Hook payload semantics are not equivalent.",
    );
    compare_field(
        source,
        target_harness,
        &mut evidence,
        &mut consequences,
        source.failure == target.failure,
        "hook.failure_mismatch",
        "Hook failure behavior is not equivalent.",
    );
    compare_field(
        source,
        target_harness,
        &mut evidence,
        &mut consequences,
        source.working_directory == target.working_directory,
        "hook.working_directory_mismatch",
        "Hook working-directory semantics are not equivalent.",
    );
    compare_field(
        source,
        target_harness,
        &mut evidence,
        &mut consequences,
        source.environment_references == target.environment_references,
        "hook.environment_mismatch",
        "Hook environment references are not equivalent.",
    );
    compare_field(
        source,
        target_harness,
        &mut evidence,
        &mut consequences,
        source.executable == target.executable,
        "hook.permission_mismatch",
        "Hook executable permission semantics are not equivalent.",
    );
    let (compatibility, fidelity) = if evidence.is_empty() {
        (CompatibilityClass::Compatible, TransferFidelity::Faithful)
    } else if requiredness == ComponentRequiredness::Required {
        (CompatibilityClass::Incompatible, TransferFidelity::Blocked)
    } else {
        (
            CompatibilityClass::TargetSpecific,
            TransferFidelity::Partial,
        )
    };
    CompatibilityResult::new(
        target_harness.clone(),
        compatibility,
        fidelity,
        evidence,
        consequences,
    )
    .map_err(HookMappingError::InvalidCompatibility)
}

fn compare_field(
    source: &HookContract,
    target: &HarnessId,
    evidence: &mut Vec<CompatibilityEvidence>,
    consequences: &mut Vec<MaterialConsequence>,
    matches: bool,
    code: &'static str,
    detail: &'static str,
) {
    if matches {
        return;
    }
    evidence.push(CompatibilityEvidence::new(
        EvidenceCode::new(code).expect("static hook evidence code"),
        target.clone(),
        [source.component.clone()],
        EvidenceDetail::new(detail).expect("static hook evidence detail"),
    ));
    consequences.push(MaterialConsequence::new(
        ConsequenceCode::new(code).expect("static hook consequence code"),
        [source.component.clone()],
        ConsequenceSummary::new(detail).expect("static hook consequence summary"),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source() -> HookContract {
        HookContract {
            component: ComponentId::new("hook:notify").unwrap(),
            event: "session_start".to_owned(),
            payload: HookPayload::Json,
            failure: HookFailure::Block,
            working_directory: HookWorkingDirectory::Plugin,
            environment_references: ["env:HOME".to_owned()].into_iter().collect(),
            executable: true,
        }
    }

    fn target() -> HookTargetContract {
        HookTargetContract {
            event: "session_start".to_owned(),
            payload: HookPayload::Json,
            failure: HookFailure::Block,
            working_directory: HookWorkingDirectory::Plugin,
            environment_references: ["env:HOME".to_owned()].into_iter().collect(),
            executable: true,
        }
    }

    #[test]
    fn identical_hook_contracts_are_faithful() {
        let resource = ResourceKey::new(
            crate::domain::ResourceId::new("plugin:test").unwrap(),
            crate::domain::Scope::Global,
        );
        let result = analyze_hook(
            &source(),
            &target(),
            ComponentRequiredness::Required,
            &HarnessId::new("codex").unwrap(),
            &resource,
        )
        .unwrap();
        assert_eq!(result.fidelity(), TransferFidelity::Faithful);
        assert!(result.evidence().is_empty());
    }

    #[test]
    fn required_mismatch_blocks_and_optional_mismatch_is_partial() {
        let resource = ResourceKey::new(
            crate::domain::ResourceId::new("plugin:test").unwrap(),
            crate::domain::Scope::Global,
        );
        let mut mismatched = target();
        mismatched.payload = HookPayload::Text;
        let target_harness = HarnessId::new("codex").unwrap();
        let blocked = analyze_hook(
            &source(),
            &mismatched,
            ComponentRequiredness::Required,
            &target_harness,
            &resource,
        )
        .unwrap();
        assert_eq!(blocked.fidelity(), TransferFidelity::Blocked);
        assert_eq!(blocked.evidence().len(), 1);
        let partial = analyze_hook(
            &source(),
            &mismatched,
            ComponentRequiredness::Optional,
            &target_harness,
            &resource,
        )
        .unwrap();
        assert_eq!(partial.fidelity(), TransferFidelity::Partial);
    }
}
