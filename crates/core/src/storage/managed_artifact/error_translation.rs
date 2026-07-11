use crate::{
    domain::{RelativeArtifactPath, ResourceKey},
    runtime::RuntimeError,
};

use super::{
    ManagedArtifactAction, ManagedArtifactError, ManagedArtifactFailure, ManagedArtifactResidual,
    ManagedRemovalResidual,
};

impl ManagedArtifactError {
    pub(super) fn new(
        action: ManagedArtifactAction,
        owner: &ResourceKey,
        path: Option<&RelativeArtifactPath>,
        failure: ManagedArtifactFailure,
    ) -> Self {
        Self {
            action,
            owner: owner.clone(),
            path: path.cloned(),
            failure,
            residual: None,
            removal_residual: None,
        }
    }

    pub(super) fn runtime(
        action: ManagedArtifactAction,
        owner: &ResourceKey,
        path: &RelativeArtifactPath,
        error: RuntimeError,
    ) -> Self {
        match error {
            RuntimeError::PartialDirectoryPublication {
                identity,
                presence,
                parent_sync,
                ..
            } => Self {
                action,
                owner: owner.clone(),
                path: Some(path.clone()),
                failure: ManagedArtifactFailure::PartialPublication,
                residual: Some(Box::new(ManagedArtifactResidual {
                    owner: owner.clone(),
                    path: path.clone(),
                    identity,
                    presence,
                    parent_sync,
                })),
                removal_residual: None,
            },
            RuntimeError::PartialDirectoryRemoval {
                expected,
                observed,
                presence,
                content,
                parent_sync,
                ..
            } => Self {
                action,
                owner: owner.clone(),
                path: Some(path.clone()),
                failure: ManagedArtifactFailure::PartialRemoval,
                residual: None,
                removal_residual: Some(Box::new(ManagedRemovalResidual {
                    owner: owner.clone(),
                    path: path.clone(),
                    expected_identity: expected,
                    observed_identity: observed,
                    presence,
                    content,
                    parent_sync,
                })),
            },
            _ => Self::new(action, owner, Some(path), ManagedArtifactFailure::Runtime),
        }
    }
}
