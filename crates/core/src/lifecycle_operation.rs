//! Constructors for native lifecycle operations.
//!
//! Harness adapters provide the executable and already-validated argument
//! vector. This module turns that boundary into the core operation contract
//! without allowing command details or target-specific policy to leak into
//! planning code.

use crate::domain::{
    AcknowledgmentRequirement, AffectedSurface, CommandArgument, CompatibilityClass,
    CompatibilityResult, EvidenceCode, EvidenceDetail, HarnessId, NativeId, Operation,
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
}
