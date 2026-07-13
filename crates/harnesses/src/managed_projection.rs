use skilltap_core::{
    domain::{AbsolutePath, HarnessId, ResourceKey, ResourceKind, Source},
    managed_projection::{AcquiredProjection, ManagedProjectionError, ManagedProjectionPlan},
    runtime::{ConfinedFileSystem, JsonLimits, PlatformPaths},
    storage::ManagedProjection,
    updates::SourceRevisionResolver,
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

/// Inputs required for target-specific source acquisition.
pub struct ManagedAcquisitionContext<'a> {
    pub target: &'a HarnessId,
    pub project: &'a AbsolutePath,
    pub paths: &'a PlatformPaths,
    pub resource_key: &'a ResourceKey,
    pub resource_kind: ResourceKind,
    pub request: &'a NativeLifecycleRequest,
    pub source: Option<&'a Source>,
    pub json_limits: JsonLimits,
    pub filesystem: &'a dyn ConfinedFileSystem,
    pub revision_resolver: &'a dyn SourceRevisionResolver,
}

/// Inputs required to map acquired content onto documented target surfaces.
pub struct ManagedProjectionContext<'a> {
    pub target: &'a HarnessId,
    pub project: &'a AbsolutePath,
    pub acquired: &'a AcquiredProjection,
    pub prior: &'a [ManagedProjection],
    pub kind: ManagedLifecycleKind,
    pub acknowledged: bool,
    pub filesystem: &'a dyn ConfinedFileSystem,
    pub json_limits: JsonLimits,
}

/// Target-specific acquisition and projection for managed fallback lifecycle.
///
/// The port owns native document codecs and target path rules. Shared
/// orchestration owns state, drift, acknowledgment, publication, and load
/// verification.
pub trait ManagedProjectionPort: Sync {
    fn acquire(
        &self,
        context: &ManagedAcquisitionContext<'_>,
    ) -> Result<AcquiredProjection, ManagedProjectionError>;

    fn project(
        &self,
        context: &ManagedProjectionContext<'_>,
    ) -> Result<ManagedProjectionPlan, ManagedProjectionError>;
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use skilltap_core::{
        domain::{
            Fingerprint, FingerprintAlgorithm, NativeId, RelativeArtifactPath, ResolvedRevision,
            ResourceId, Scope, SourceKind, SourceLocator,
        },
        managed_projection::ManagedFileWrite,
        runtime::{Environment, EnvironmentVariable, SupportedPlatform, SystemFileSystem},
        updates::ResolutionError,
    };

    use super::*;
    use crate::lifecycle::NativeLifecycleAction;

    struct TestEnvironment;

    impl Environment for TestEnvironment {
        fn value(&self, variable: EnvironmentVariable) -> Option<OsString> {
            (variable == EnvironmentVariable::Home).then(|| OsString::from("/test-home"))
        }
    }

    struct TestRevisionResolver;

    impl SourceRevisionResolver for TestRevisionResolver {
        fn resolve(&self, _source: &Source) -> Result<ResolvedRevision, ResolutionError> {
            Err(ResolutionError::UnreachableSource)
        }
    }

    struct TestPort;

    impl ManagedProjectionPort for TestPort {
        fn acquire(
            &self,
            context: &ManagedAcquisitionContext<'_>,
        ) -> Result<AcquiredProjection, ManagedProjectionError> {
            Ok(AcquiredProjection::MarketplaceCatalog {
                bytes: b"catalog".to_vec(),
                fingerprint: Fingerprint::new(FingerprintAlgorithm::Sha256, "a".repeat(64))
                    .expect("test fingerprint is valid"),
                source: context
                    .source
                    .cloned()
                    .ok_or(ManagedProjectionError::SourceMissing)?,
                installed_revision: None,
            })
        }

        fn project(
            &self,
            context: &ManagedProjectionContext<'_>,
        ) -> Result<ManagedProjectionPlan, ManagedProjectionError> {
            let AcquiredProjection::MarketplaceCatalog { bytes, .. } = context.acquired else {
                return Err(ManagedProjectionError::UnsupportedResourceKind);
            };
            Ok(ManagedProjectionPlan {
                files: vec![ManagedFileWrite {
                    root: context.project.clone(),
                    destination: RelativeArtifactPath::new(".agents/plugins/marketplace.json")
                        .expect("test path is valid"),
                    expected: None,
                    desired: Some(bytes.clone()),
                }],
                ..ManagedProjectionPlan::default()
            })
        }
    }

    #[test]
    fn managed_projection_port_is_object_safe_and_round_trips_pure_types() {
        let port: &dyn ManagedProjectionPort = &TestPort;
        let target = HarnessId::new("test").unwrap();
        let project = AbsolutePath::new("/project").unwrap();
        let paths = PlatformPaths::resolve_for(SupportedPlatform::Linux, &TestEnvironment).unwrap();
        let resource_key = ResourceKey::new(
            ResourceId::new("marketplace:test").unwrap(),
            Scope::Project(project.clone()),
        );
        let source = Source::new(
            SourceKind::Local,
            SourceLocator::new("/source").unwrap(),
            None,
        )
        .unwrap();
        let request = NativeLifecycleRequest {
            action: NativeLifecycleAction::MarketplaceAdd,
            scope: Scope::Project(project.clone()),
            name: NativeId::new("test").unwrap(),
            source: Some(SourceLocator::new("/source").unwrap()),
        };
        let filesystem = SystemFileSystem;
        let revision_resolver = TestRevisionResolver;
        let json_limits = JsonLimits::new(1024, 8).unwrap();

        let acquired = port
            .acquire(&ManagedAcquisitionContext {
                target: &target,
                project: &project,
                paths: &paths,
                resource_key: &resource_key,
                resource_kind: ResourceKind::Marketplace,
                request: &request,
                source: Some(&source),
                json_limits,
                filesystem: &filesystem,
                revision_resolver: &revision_resolver,
            })
            .unwrap();
        assert_eq!(acquired.source(), &source);

        let plan = port
            .project(&ManagedProjectionContext {
                target: &target,
                project: &project,
                acquired: &acquired,
                prior: &[],
                kind: ManagedLifecycleKind::MarketplaceAdd,
                acknowledged: false,
                filesystem: &filesystem,
                json_limits,
            })
            .unwrap();

        assert_eq!(
            plan,
            ManagedProjectionPlan {
                files: vec![ManagedFileWrite {
                    root: project,
                    destination: RelativeArtifactPath::new(".agents/plugins/marketplace.json")
                        .unwrap(),
                    expected: None,
                    desired: Some(b"catalog".to_vec()),
                }],
                ..ManagedProjectionPlan::default()
            }
        );
    }
}
