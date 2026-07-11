//! Fresh effective-load verification for managed target projections.

use skilltap_core::{
    domain::HarnessObservation,
    publication::{
        LoadVerificationError, LoadVerifier, PublicationEntry, PublishedArtifact, VerifiedTarget,
        verify_observed_load,
    },
};

/// Verifies a publication against one fresh normalized harness observation.
/// The observation must have been obtained from the adapter's effective load
/// path; this type never reads native caches or filesystem paths itself.
pub struct EffectiveObservationVerifier<'a> {
    observation: &'a HarnessObservation,
}

impl<'a> EffectiveObservationVerifier<'a> {
    pub const fn new(observation: &'a HarnessObservation) -> Self {
        Self { observation }
    }
}

impl LoadVerifier for EffectiveObservationVerifier<'_> {
    fn verify_loaded(
        &self,
        entry: &PublicationEntry,
        artifact: &PublishedArtifact,
    ) -> Result<VerifiedTarget, LoadVerificationError> {
        let observed = self.observation.resources().values().find(|observed| {
            observed.key().resource() == &entry.resource
                && observed.key().harness() == &entry.target
        });
        verify_observed_load(entry, artifact, observed)
    }
}
