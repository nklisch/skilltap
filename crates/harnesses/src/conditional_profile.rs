//! Optional compound-profile inspection for conditional harnesses.
//!
//! The port is intentionally separate from ordinary resource observation. A
//! companion package can provide capability evidence without becoming a
//! managed resource or an adoption candidate.

use skilltap_core::{
    domain::{
        ConditionalComponentReport, ConditionalProfileError, NativeVersion, ProfileComponentSet,
    },
    runtime::{ConfinedFileSystem, JsonLimits, PlatformPaths},
};

use skilltap_core::domain::Scope;

/// Inputs available to one bounded component inspection for one concrete scope.
pub struct ConditionalProfileContext<'a> {
    pub scope: &'a Scope,
    pub paths: &'a PlatformPaths,
    pub filesystem: &'a dyn ConfinedFileSystem,
    pub json_limits: JsonLimits,
    pub maximum_manifest_bytes: u64,
}

/// Optional adapter port for compound capability providers.
pub trait ConditionalProfilePort: Sync {
    fn inspect_components(
        &self,
        context: &ConditionalProfileContext<'_>,
    ) -> Result<ConditionalComponentReport, ConditionalProfileError>;

    fn select_compiled_profile(
        &self,
        runtime_version: &NativeVersion,
        components: &ProfileComponentSet,
    ) -> skilltap_core::domain::CapabilityProfileSelection;
}
