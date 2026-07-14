//! Constructors for native lifecycle operations.
//!
//! Harness adapters provide the executable and already-validated argument
//! vector. This module turns that boundary into the core operation contract
//! without allowing command details or target-specific policy to leak into
//! planning code.

use crate::domain::{
    AcknowledgmentRequirement, AffectedSurface, CommandArgument, CompatibilityClass,
    CompatibilityEvidence, CompatibilityResult, ComponentId, ConsequenceCode, ConsequenceSummary,
    EvidenceCode, EvidenceDetail, HarnessId, MaterialConsequence, NativeId, Operation,
    OperationAction, OperationClass, OperationId, OperationReason, OperationSelector,
    OperationSemantics, Provenance, ResourceKey, Reversibility, TransferFidelity,
};

/// Build a faithful, lock-eligible native lifecycle operation.
///
/// Arguments are retained only as typed command arguments so renderers can
/// redact them where required. The operation reason is deliberately generic;
/// adapters must not put source locators, paths, or other user-controlled
/// values into evidence text.
pub fn native_operation(
    id: OperationId,
    target: HarnessId,
    resource: ResourceKey,
    action: OperationAction,
    executable: NativeId,
    arguments: impl IntoIterator<Item = CommandArgument>,
) -> Result<Operation, crate::domain::OperationContractError> {
    let compatibility = CompatibilityResult::new(
        target.clone(),
        CompatibilityClass::Compatible,
        TransferFidelity::Faithful,
        [],
        [],
    )
    .expect("faithful native operations have no evidence or consequences");
    let semantics = OperationSemantics::new(
        action,
        resource.scope().clone(),
        OperationReason::new(
            EvidenceCode::new("native.lifecycle").expect("static evidence code is valid"),
            EvidenceDetail::new("The verified harness native lifecycle command will be used.")
                .expect("static evidence detail is valid"),
        ),
        compatibility,
        Provenance::Native,
        [AffectedSurface::native_command(
            target.clone(),
            executable,
            arguments,
        )],
    );
    Operation::new(
        id,
        target,
        OperationSelector::Resource { resource },
        semantics,
        OperationClass::SafeNative,
        Reversibility::Reversible,
        [],
        AcknowledgmentRequirement::not_required(),
        None,
    )
}

/// Build a journaled no-op after fresh native observation proves that a prior
/// attempted lifecycle operation already achieved its requested state.
/// Build a typed blocked native lifecycle operation. The operation remains in
/// the validated plan so dependency skips and journal evidence are explicit,
/// while the native port is not given a request it could accidentally run.
pub fn blocked_native_operation(
    id: OperationId,
    target: HarnessId,
    resource: ResourceKey,
    action: OperationAction,
    code: EvidenceCode,
    detail: EvidenceDetail,
) -> Result<Operation, crate::domain::OperationContractError> {
    let compatibility = CompatibilityResult::new(
        target.clone(),
        CompatibilityClass::Incompatible,
        TransferFidelity::Blocked,
        [],
        [],
    )
    .expect("blocked native operations have bounded compatibility evidence");
    let semantics = OperationSemantics::new(
        action,
        resource.scope().clone(),
        OperationReason::new(code.clone(), detail.clone()),
        compatibility,
        Provenance::Native,
        [],
    );
    Operation::new(
        id,
        target,
        OperationSelector::Resource { resource },
        semantics,
        OperationClass::Unsupported,
        Reversibility::NotApplicable,
        [],
        AcknowledgmentRequirement::not_required(),
        Some(crate::domain::AttentionReason::unsupported(code, detail)),
    )
}

pub fn native_noop_operation(
    id: OperationId,
    target: HarnessId,
    resource: ResourceKey,
    action: OperationAction,
    executable: NativeId,
    arguments: impl IntoIterator<Item = CommandArgument>,
) -> Result<Operation, crate::domain::OperationContractError> {
    let compatibility = CompatibilityResult::new(
        target.clone(),
        CompatibilityClass::Compatible,
        TransferFidelity::Faithful,
        [],
        [],
    )
    .expect("faithful native no-op operations have no evidence or consequences");
    let semantics = OperationSemantics::new(
        action,
        resource.scope().clone(),
        OperationReason::new(
            EvidenceCode::new("native.lifecycle.verified_noop")
                .expect("static evidence code is valid"),
            EvidenceDetail::new(
                "Fresh native observation already satisfies the requested lifecycle state.",
            )
            .expect("static evidence detail is valid"),
        ),
        compatibility,
        Provenance::Native,
        [AffectedSurface::native_command(
            target.clone(),
            executable,
            arguments,
        )],
    );
    Operation::new(
        id,
        target,
        OperationSelector::Resource { resource },
        semantics,
        OperationClass::NoOp,
        Reversibility::NotApplicable,
        [],
        AcknowledgmentRequirement::not_required(),
        None,
    )
}

/// Build a faithful managed-file operation for a complete resource tree.
pub fn faithful_file_operation(
    id: OperationId,
    target: HarnessId,
    resource: ResourceKey,
    action: OperationAction,
    path: crate::domain::AbsolutePath,
) -> Result<Operation, crate::domain::OperationContractError> {
    faithful_file_operation_with_dependencies(id, target, resource, action, path, [])
}

/// Variant of [`faithful_file_operation`] that declares exact operation
/// dependencies for multi-file setup workflows.
pub fn faithful_file_operation_with_dependencies(
    id: OperationId,
    target: HarnessId,
    resource: ResourceKey,
    action: OperationAction,
    path: crate::domain::AbsolutePath,
    dependencies: impl IntoIterator<Item = crate::domain::OperationDependency>,
) -> Result<Operation, crate::domain::OperationContractError> {
    let compatibility = CompatibilityResult::new(
        target.clone(),
        CompatibilityClass::Compatible,
        TransferFidelity::Faithful,
        [],
        [],
    )
    .expect("faithful file operations have no evidence or consequences");
    let semantics = OperationSemantics::new(
        action,
        resource.scope().clone(),
        OperationReason::new(
            EvidenceCode::new("managed.file").expect("static evidence code is valid"),
            EvidenceDetail::new("The complete managed resource tree will be published.")
                .expect("static evidence detail is valid"),
        ),
        compatibility,
        Provenance::Direct,
        [AffectedSurface::file(path)],
    );
    Operation::new(
        id,
        target,
        OperationSelector::Resource { resource },
        semantics,
        OperationClass::SafeFaithfulEquivalent,
        Reversibility::Reversible,
        dependencies,
        AcknowledgmentRequirement::not_required(),
        None,
    )
}

/// Build a managed materialization operation that names every file or
/// directory the adapter will touch.
pub fn managed_materialization_operation(
    id: OperationId,
    target: HarnessId,
    resource: ResourceKey,
    action: OperationAction,
    paths: impl IntoIterator<Item = crate::domain::AbsolutePath>,
) -> Result<Operation, crate::domain::OperationContractError> {
    let component =
        ComponentId::new("managed:representation").expect("static managed component id is valid");
    let compatibility = CompatibilityResult::new(
        target.clone(),
        CompatibilityClass::Compatible,
        TransferFidelity::Materializable,
        [CompatibilityEvidence::new(
            EvidenceCode::new("managed.load_path").expect("static evidence code is valid"),
            target.clone(),
            [component.clone()],
            EvidenceDetail::new(
                "The harness lacks a verified native project lifecycle command; skilltap will own the documented load-path representation.",
            )
            .expect("static evidence detail is valid"),
        )],
        [MaterialConsequence::new(
            ConsequenceCode::new("managed.ownership")
                .expect("static consequence code is valid"),
            [component],
            ConsequenceSummary::new(
                "The project representation is managed by skilltap rather than the harness lifecycle.",
            )
            .expect("static consequence summary is valid"),
        )],
    )
    .expect("managed materialization operations have no partial consequences");
    let semantics = OperationSemantics::new(
        action,
        resource.scope().clone(),
        OperationReason::new(
            EvidenceCode::new("managed.lifecycle").expect("static evidence code is valid"),
            EvidenceDetail::new("The documented managed harness load paths will be updated.")
                .expect("static evidence detail is valid"),
        ),
        compatibility,
        Provenance::Materialized,
        paths.into_iter().map(AffectedSurface::file),
    );
    Operation::new(
        id,
        target,
        OperationSelector::Resource { resource },
        semantics,
        OperationClass::SafeMaterialization,
        Reversibility::Reversible,
        [],
        AcknowledgmentRequirement::not_required(),
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ResourceId, Scope};

    #[test]
    fn native_operation_is_faithful_and_keeps_typed_arguments() {
        let target = HarnessId::new("claude").unwrap();
        let operation = native_operation(
            OperationId::new("plugin-install-claude").unwrap(),
            target.clone(),
            ResourceKey::new(ResourceId::new("plugin:demo@team").unwrap(), Scope::Global),
            OperationAction::PluginInstall,
            NativeId::new("claude").unwrap(),
            [CommandArgument::literal(
                NativeId::new("plugin:demo@team").unwrap(),
            )],
        )
        .unwrap();

        assert_eq!(operation.class(), OperationClass::SafeNative);
        assert_eq!(
            operation.compatibility().fidelity(),
            TransferFidelity::Faithful
        );
        let surface = operation.affected_surfaces().iter().next().unwrap();
        assert_eq!(surface.target(), Some(&target));
        assert_eq!(surface.arguments().unwrap().len(), 1);
    }

    #[test]
    fn native_operation_supports_project_scopes_without_guessing_capabilities() {
        let target = HarnessId::new("claude").unwrap();
        let result = native_operation(
            OperationId::new("plugin-install-claude").unwrap(),
            target,
            ResourceKey::new(
                ResourceId::new("plugin:demo").unwrap(),
                Scope::Project(crate::domain::AbsolutePath::new("/tmp/project").unwrap()),
            ),
            OperationAction::PluginInstall,
            NativeId::new("claude").unwrap(),
            [],
        );
        assert!(result.is_ok());
    }

    #[test]
    fn faithful_file_operation_uses_the_complete_tree_surface() {
        let target = HarnessId::new("codex").unwrap();
        let operation = faithful_file_operation(
            OperationId::new("skill-install-codex").unwrap(),
            target,
            ResourceKey::new(ResourceId::new("skill:demo").unwrap(), Scope::Global),
            OperationAction::SkillInstall,
            crate::domain::AbsolutePath::new("/home/user/.agents/skills/demo").unwrap(),
        )
        .unwrap();
        assert_eq!(operation.class(), OperationClass::SafeFaithfulEquivalent);
        assert!(operation.affected_surfaces().iter().any(|surface| {
            surface
                .path()
                .is_some_and(|path| path.as_str().ends_with("/demo"))
        }));
    }

    #[test]
    fn managed_lifecycle_uses_the_materialization_contract() {
        let operation = managed_materialization_operation(
            OperationId::new("managed-project-marketplace").unwrap(),
            HarnessId::new("codex").unwrap(),
            ResourceKey::new(ResourceId::new("marketplace:local").unwrap(), Scope::Global),
            OperationAction::MarketplaceRegister,
            [crate::domain::AbsolutePath::new("/tmp/marketplace.json").unwrap()],
        )
        .unwrap();
        assert_eq!(operation.class(), OperationClass::SafeMaterialization);
        assert_eq!(
            operation.compatibility().fidelity(),
            TransferFidelity::Materializable
        );
    }
}
