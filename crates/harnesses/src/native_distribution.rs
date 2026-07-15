//! Adapter-owned native distribution assessment.
//!
//! This port reports target-native component evidence for one already-resolved
//! source checkout. It does not execute a native lifecycle command, acquire a
//! source, or define a universal plugin manifest.

use std::fmt;

use skilltap_core::{
    domain::{HarnessId, RequestedRevision, Scope},
    managed_projection::ResolvedSourceCheckout,
    materialization::MaterializationPlan,
    plugin_graph::SourceComponentGraph,
    runtime::{ConfinedFileSystem, JsonLimits},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeDistributionAssessment {
    pub graph: SourceComponentGraph,
    pub plan: MaterializationPlan,
}

pub struct NativeDistributionContext<'a> {
    pub target: &'a HarnessId,
    pub scope: &'a Scope,
    pub checkout: &'a ResolvedSourceCheckout,
    pub requested_revision: Option<&'a RequestedRevision>,
    pub filesystem: &'a dyn ConfinedFileSystem,
    pub json_limits: JsonLimits,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NativeDistributionError {
    UnsupportedSource,
    SourceUnavailable,
    MalformedSource,
    InvalidAssessment,
    Other {
        code: &'static str,
        summary: &'static str,
    },
}

impl NativeDistributionError {
    pub const fn code(&self) -> &'static str {
        match self {
            Self::UnsupportedSource => "native_distribution_unsupported_source",
            Self::SourceUnavailable => "native_distribution_source_unavailable",
            Self::MalformedSource => "native_distribution_source_malformed",
            Self::InvalidAssessment => "native_distribution_assessment_invalid",
            Self::Other { code, .. } => code,
        }
    }

    pub const fn summary(&self) -> &'static str {
        match self {
            Self::UnsupportedSource => {
                "The selected source has no verified native distribution for this target."
            }
            Self::SourceUnavailable => {
                "The resolved source checkout could not be assessed by the native adapter."
            }
            Self::MalformedSource => {
                "The selected source's native distribution metadata is malformed."
            }
            Self::InvalidAssessment => "The native adapter returned invalid distribution evidence.",
            Self::Other { summary, .. } => summary,
        }
    }
}

impl fmt::Display for NativeDistributionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.summary())
    }
}

impl std::error::Error for NativeDistributionError {}

/// Assess a source for one target and concrete scope without granting
/// lifecycle mutation authority. The caller owns checkout resolution and later
/// selects a native profile before executing any operation.
pub trait NativeDistributionPort: Sync {
    fn assess(
        &self,
        context: &NativeDistributionContext<'_>,
    ) -> Result<Option<NativeDistributionAssessment>, NativeDistributionError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use skilltap_core::{
        domain::{AbsolutePath, Scope, Source, SourceKind, SourceLocator},
        runtime::SystemFileSystem,
    };

    struct TestPort;

    impl NativeDistributionPort for TestPort {
        fn assess(
            &self,
            context: &NativeDistributionContext<'_>,
        ) -> Result<Option<NativeDistributionAssessment>, NativeDistributionError> {
            assert_eq!(context.target.as_str(), "fixture");
            assert_eq!(context.scope, &Scope::Global);
            assert_eq!(context.checkout.root().as_str(), "/checkout");
            assert!(context.requested_revision.is_none());
            assert_eq!(context.json_limits, JsonLimits::new(1024, 8).unwrap());
            let _ = context.filesystem;
            Ok(None)
        }
    }

    #[test]
    fn assessment_port_receives_one_resolved_checkout_and_concrete_scope() {
        let target = HarnessId::new("fixture").unwrap();
        let source = Source::new(
            SourceKind::Local,
            SourceLocator::new("/checkout").unwrap(),
            None,
        )
        .unwrap();
        let checkout =
            ResolvedSourceCheckout::new(AbsolutePath::new("/checkout").unwrap(), source, None);
        let limits = JsonLimits::new(1024, 8).unwrap();
        let port: &dyn NativeDistributionPort = &TestPort;
        assert_eq!(
            port.assess(&NativeDistributionContext {
                target: &target,
                scope: &Scope::Global,
                checkout: &checkout,
                requested_revision: None,
                filesystem: &SystemFileSystem,
                json_limits: limits,
            }),
            Ok(None)
        );
    }
}
