use skilltap_core::{
    domain::{HarnessId, ResourceKey, ResourceKind, Scope},
    managed_projection::{ManagedProjectionError, ManagedProjectionPlan, ResolvedSourceCheckout},
    runtime::{ConfinedFileSystem, JsonLimits, PlatformPaths},
    storage::ManagedProjection,
};

use crate::lifecycle::NativeLifecycleRequest;

/// Managed lifecycle action shared with target projection adapters.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ManagedLifecycleKind {
    MarketplaceAdd,
    MarketplaceRemove,
    MarketplaceUpdate,
    PluginInstall,
    PluginRemove,
    PluginUpdate,
}

/// The source requirement for one managed-projection plan.
///
/// Invalid states are unrepresentable: apply always has one authoritative
/// checkout, while removal cannot carry or require source evidence.
#[derive(Clone, Debug)]
pub enum ManagedProjectionInput<'a> {
    Apply {
        checkout: &'a ResolvedSourceCheckout,
    },
    Remove,
}

/// Inputs required to plan one operation on documented target surfaces.
pub struct ManagedProjectionContext<'a> {
    pub target: &'a HarnessId,
    /// The exact concrete scope being planned. Adapters derive their native
    /// global/project roots from this value and `PlatformPaths`.
    pub scope: &'a Scope,
    pub paths: &'a PlatformPaths,
    pub resource_key: &'a ResourceKey,
    pub resource_kind: ResourceKind,
    pub request: &'a NativeLifecycleRequest,
    pub kind: ManagedLifecycleKind,
    pub input: ManagedProjectionInput<'a>,
    pub prior: &'a [ManagedProjection],
    pub filesystem: &'a dyn ConfinedFileSystem,
    pub json_limits: JsonLimits,
}

/// Target-specific acquisition and projection for managed fallback lifecycle.
///
/// Apply receives the caller-resolved checkout that the adapter reads and
/// projects in one pass. Remove plans only from prior evidence and current
/// filesystem observation. Shared orchestration owns state, drift,
/// acknowledgment, publication, and load verification.
pub trait ManagedProjectionPort: Sync {
    fn plan(
        &self,
        context: &ManagedProjectionContext<'_>,
    ) -> Result<ManagedProjectionPlan, ManagedProjectionError>;
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use skilltap_core::{
        domain::{
            AbsolutePath, Fingerprint, FingerprintAlgorithm, GitCommit, NativeId,
            RelativeArtifactPath, ResolvedRevision, ResourceId, Scope, Source, SourceKind,
            SourceLocator,
        },
        managed_projection::{ManagedFileWrite, ResolvedSourceCheckout},
        runtime::{Environment, EnvironmentVariable, SupportedPlatform, SystemFileSystem},
    };

    use super::*;
    use crate::lifecycle::NativeLifecycleAction;

    struct TestEnvironment;

    impl Environment for TestEnvironment {
        fn value(&self, variable: EnvironmentVariable) -> Option<OsString> {
            (variable == EnvironmentVariable::Home).then(|| OsString::from("/test-home"))
        }
    }

    struct TestPort {
        checkout: ResolvedSourceCheckout,
        apply_plan: ManagedProjectionPlan,
    }

    impl ManagedProjectionPort for TestPort {
        fn plan(
            &self,
            context: &ManagedProjectionContext<'_>,
        ) -> Result<ManagedProjectionPlan, ManagedProjectionError> {
            match &context.input {
                ManagedProjectionInput::Apply { checkout } => {
                    assert_eq!(*checkout, &self.checkout);
                    Ok(self.apply_plan.clone())
                }
                ManagedProjectionInput::Remove => Ok(ManagedProjectionPlan::default()),
            }
        }
    }

    fn fingerprint(digit: char) -> Fingerprint {
        Fingerprint::new(FingerprintAlgorithm::Sha256, digit.to_string().repeat(64))
            .expect("test fingerprint is valid")
    }

    #[test]
    fn managed_projection_port_is_object_safe_and_round_trips_apply_and_remove() {
        let target = HarnessId::new("test").unwrap();
        let project = AbsolutePath::new("/project").unwrap();
        let paths = PlatformPaths::resolve_for(SupportedPlatform::Linux, &TestEnvironment).unwrap();
        let resource_key = ResourceKey::new(
            ResourceId::new("plugin:test").unwrap(),
            Scope::Project(project.clone()),
        );
        let source = Source::new(
            SourceKind::Git,
            SourceLocator::new("https://example.invalid/marketplace.git").unwrap(),
            None,
        )
        .unwrap();
        let revision = ResolvedRevision::GitCommit(GitCommit::new("a".repeat(40)).unwrap());
        let checkout = ResolvedSourceCheckout::new(
            AbsolutePath::new("/checkout").unwrap(),
            source.clone(),
            Some(revision.clone()),
        );
        assert_eq!(checkout.root().as_str(), "/checkout");
        assert_eq!(checkout.source(), &source);
        assert_eq!(checkout.revision(), Some(&revision));

        let request = NativeLifecycleRequest {
            action: NativeLifecycleAction::PluginInstall,
            scope: Scope::Project(project.clone()),
            name: NativeId::new("test").unwrap(),
            source: Some(source.locator().clone()),
        };
        let current_fingerprint = fingerprint('b');
        let desired_fingerprint = fingerprint('c');
        let manifest = vec![ManagedProjection::Skill {
            id: RelativeArtifactPath::new("test").unwrap(),
            fingerprint: desired_fingerprint.clone(),
        }];
        let apply_plan = ManagedProjectionPlan {
            files: vec![ManagedFileWrite {
                root: project.clone(),
                destination: RelativeArtifactPath::new(".agents/plugins/marketplace.json").unwrap(),
                expected: None,
                desired: Some(b"catalog".to_vec()),
            }],
            manifest: manifest.clone(),
            current_fingerprint: Some(current_fingerprint.clone()),
            desired_fingerprint: Some(desired_fingerprint.clone()),
            ..ManagedProjectionPlan::default()
        };
        let port = TestPort {
            checkout: checkout.clone(),
            apply_plan: apply_plan.clone(),
        };
        let port: &dyn ManagedProjectionPort = &port;
        let filesystem = SystemFileSystem;
        let json_limits = JsonLimits::new(1024, 8).unwrap();

        let planned_apply = port
            .plan(&ManagedProjectionContext {
                target: &target,
                scope: &Scope::Project(project.clone()),
                paths: &paths,
                resource_key: &resource_key,
                resource_kind: ResourceKind::Plugin,
                request: &request,
                kind: ManagedLifecycleKind::PluginInstall,
                input: ManagedProjectionInput::Apply {
                    checkout: &checkout,
                },
                prior: &[],
                filesystem: &filesystem,
                json_limits,
            })
            .unwrap();
        assert_eq!(planned_apply, apply_plan);
        assert_eq!(planned_apply.manifest, manifest);
        assert_eq!(
            planned_apply.current_fingerprint.as_ref(),
            Some(&current_fingerprint)
        );
        assert_eq!(
            planned_apply.desired_fingerprint.as_ref(),
            Some(&desired_fingerprint)
        );

        let planned_remove = port
            .plan(&ManagedProjectionContext {
                target: &target,
                scope: &Scope::Project(project.clone()),
                paths: &paths,
                resource_key: &resource_key,
                resource_kind: ResourceKind::Plugin,
                request: &request,
                kind: ManagedLifecycleKind::PluginRemove,
                input: ManagedProjectionInput::Remove,
                prior: &planned_apply.manifest,
                filesystem: &filesystem,
                json_limits,
            })
            .unwrap();
        assert_eq!(planned_remove, ManagedProjectionPlan::default());
    }
}
