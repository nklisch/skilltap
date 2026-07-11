//! Pure update resolution and safety classification shared by foreground and
//! daemon paths.

use crate::compatibility::CompatibilityAnalysis;
use crate::domain::{
    DesiredResource, HarnessId, ResolvedRevision, Source, SourceKind, TransferFidelity,
    UpdateIntent,
};
use crate::storage::UpdateMode;

/// A typed failure at the revision-resolution boundary. Native output and
/// process details never cross this contract.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResolutionError {
    UnreachableSource,
    InvalidRequestedRevision,
    UnsupportedSourceKind(SourceKind),
    NativeObservationUnavailable,
    TargetDisagreement,
}

/// An explicit update-resolution request. Drift and compatibility evidence are
/// supplied by the observation/planning layer rather than inferred here.
#[derive(Clone, Copy)]
pub struct UpdateResolutionRequest<'a> {
    pub resource: &'a DesiredResource,
    pub installed: Option<&'a ResolvedRevision>,
    pub drifted: bool,
    pub compatibility_changed: bool,
    pub requires_acknowledgment: bool,
}

/// Port for resolving an explicitly selected source without mutating the
/// installed resource or native harness.
pub trait SourceRevisionResolver {
    fn resolve(&self, source: &Source) -> Result<ResolvedRevision, ResolutionError>;
}

/// Port backed by fresh native observations for resources without a portable
/// source locator.
pub trait NativeRevisionResolver {
    fn resolve(
        &self,
        resource: &DesiredResource,
        target: &HarnessId,
    ) -> Result<Option<ResolvedRevision>, ResolutionError>;
}

/// The result of a resolution attempt. `error` is deliberately separate from
/// `available` so a failed check can never be mistaken for a no-update result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedUpdate {
    pub current: Option<ResolvedRevision>,
    pub available: Option<ResolvedRevision>,
    pub error: Option<ResolutionError>,
}

/// A candidate consumed by the existing safety policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdateCandidate {
    pub current_revision: Option<ResolvedRevision>,
    pub available_revision: Option<ResolvedRevision>,
    pub resolution_error: Option<ResolutionError>,
    pub pinned: bool,
    pub drifted: bool,
    pub compatibility_changed: bool,
    pub requires_acknowledgment: bool,
    pub intent: UpdateIntent,
}

/// Resolve one desired resource. Source-backed resources resolve once; native
/// resources resolve once per target and require all concrete revisions to
/// agree before returning an available revision.
pub fn resolve_candidate<R, N>(
    source_resolver: &R,
    native_resolver: &N,
    request: UpdateResolutionRequest<'_>,
) -> ResolvedUpdate
where
    R: ?Sized + SourceRevisionResolver,
    N: ?Sized + NativeRevisionResolver,
{
    let current = request.installed.cloned();
    let available: Result<Option<ResolvedRevision>, ResolutionError> =
        if let Some(source) = request.resource.source() {
            source_resolver.resolve(source).map(Some)
        } else {
            let mut resolved = None;
            for target in request.resource.targets().iter() {
                match native_resolver.resolve(request.resource, target) {
                    Ok(Some(revision)) => {
                        if resolved
                            .as_ref()
                            .is_some_and(|existing| existing != &revision)
                        {
                            return ResolvedUpdate {
                                current,
                                available: None,
                                error: Some(ResolutionError::TargetDisagreement),
                            };
                        }
                        resolved = Some(revision);
                    }
                    Ok(None) => {
                        return ResolvedUpdate {
                            current,
                            available: None,
                            error: Some(ResolutionError::NativeObservationUnavailable),
                        };
                    }
                    Err(error) => {
                        return ResolvedUpdate {
                            current,
                            available: None,
                            error: Some(error),
                        };
                    }
                }
            }
            resolved
                .map(Some)
                .ok_or(ResolutionError::NativeObservationUnavailable)
        };
    match available {
        Ok(Some(revision)) => ResolvedUpdate {
            current,
            available: Some(revision),
            error: None,
        },
        Ok(None) => ResolvedUpdate {
            current,
            available: None,
            error: Some(ResolutionError::NativeObservationUnavailable),
        },
        Err(error) => ResolvedUpdate {
            current,
            available: None,
            error: Some(error),
        },
    }
}

/// Resolve a deterministic batch without mutating any external resource.
pub fn check_updates<'a, R, N>(
    requests: impl IntoIterator<Item = UpdateResolutionRequest<'a>>,
    source_resolver: &R,
    native_resolver: &N,
) -> Vec<ResolvedUpdate>
where
    R: ?Sized + SourceRevisionResolver,
    N: ?Sized + NativeRevisionResolver,
{
    requests
        .into_iter()
        .map(|request| resolve_candidate(source_resolver, native_resolver, request))
        .collect()
}

/// Build the safety-policy candidate from a successful or failed resolution.
/// Callers must check `ResolvedUpdate::error` before treating the candidate as
/// actionable; unresolved candidates intentionally carry no available value.
pub fn candidate_for(
    resource: &DesiredResource,
    request: &UpdateResolutionRequest<'_>,
    resolved: &ResolvedUpdate,
) -> UpdateCandidate {
    UpdateCandidate {
        current_revision: resolved.current.clone(),
        available_revision: resolved.available.clone(),
        resolution_error: resolved.error.clone(),
        pinned: resource.update() == UpdateIntent::Pinned,
        drifted: request.drifted,
        compatibility_changed: request.compatibility_changed,
        requires_acknowledgment: request.requires_acknowledgment,
        intent: resource.update(),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UpdateSafety {
    NoUpdate,
    Safe,
    NeedsDecision,
    Blocked,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UpdateDecisionReason {
    DisabledResource,
    GlobalModeOff,
    CheckOnly,
    PinnedResource,
    Drifted,
    CompatibilityChanged,
    AcknowledgmentRequired,
    ResolutionFailed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UpdateDecision {
    pub safety: UpdateSafety,
    pub reason: Option<UpdateDecisionReason>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UpdateChangeSummary {
    pub compatibility_changed: bool,
    pub added_required_components: usize,
    pub partial_components: usize,
    pub requires_acknowledgment: bool,
}

/// Compare two target-bound analyses without using revision text as a proxy
/// for behavior. The resulting summary is safe to feed into an update
/// candidate before policy classification.
pub fn update_change_summary(
    before: &CompatibilityAnalysis,
    after: &CompatibilityAnalysis,
) -> UpdateChangeSummary {
    let mut added_required_components = 0;
    let mut partial_components = 0;
    for (component, next) in &after.components {
        let previous = before.components.get(component);
        let previous_fidelity = previous
            .map(|decision| decision.result.fidelity())
            .unwrap_or(TransferFidelity::Faithful);
        if next.requiredness == crate::domain::ComponentRequiredness::Required
            && next.result.fidelity() == TransferFidelity::Blocked
            && previous_fidelity != TransferFidelity::Blocked
        {
            added_required_components += 1;
        }
        if next.result.fidelity() == TransferFidelity::Partial
            && previous_fidelity != TransferFidelity::Partial
        {
            partial_components += 1;
        }
    }
    let new_selectors = after
        .acknowledgment_selectors
        .difference(&before.acknowledgment_selectors)
        .count();
    let compatibility_changed =
        before.aggregate != after.aggregate || before.components != after.components;
    UpdateChangeSummary {
        compatibility_changed,
        added_required_components,
        partial_components,
        requires_acknowledgment: new_selectors > 0,
    }
}

impl UpdateDecision {
    const fn new(safety: UpdateSafety, reason: Option<UpdateDecisionReason>) -> Self {
        Self { safety, reason }
    }
}

pub fn classify_update(candidate: &UpdateCandidate) -> UpdateSafety {
    classify_update_with_mode(candidate, UpdateMode::ApplySafe).safety
}

/// Classify whether an update may be applied automatically under the global
/// policy. This is deliberately independent of revision distance or semver.
pub fn classify_update_with_mode(candidate: &UpdateCandidate, mode: UpdateMode) -> UpdateDecision {
    if candidate.resolution_error.is_some() {
        return UpdateDecision::new(
            UpdateSafety::Blocked,
            Some(UpdateDecisionReason::ResolutionFailed),
        );
    }
    if candidate.intent == UpdateIntent::Disabled {
        return UpdateDecision::new(
            UpdateSafety::NoUpdate,
            Some(UpdateDecisionReason::DisabledResource),
        );
    }
    if candidate.available_revision.is_none()
        || candidate.current_revision == candidate.available_revision
    {
        return UpdateDecision::new(UpdateSafety::NoUpdate, None);
    }
    if mode == UpdateMode::Off {
        return UpdateDecision::new(
            UpdateSafety::Blocked,
            Some(UpdateDecisionReason::GlobalModeOff),
        );
    }
    if candidate.drifted {
        return UpdateDecision::new(UpdateSafety::Blocked, Some(UpdateDecisionReason::Drifted));
    }
    if candidate.pinned || candidate.intent == UpdateIntent::Pinned {
        return UpdateDecision::new(
            UpdateSafety::NeedsDecision,
            Some(UpdateDecisionReason::PinnedResource),
        );
    }
    if candidate.compatibility_changed {
        return UpdateDecision::new(
            UpdateSafety::NeedsDecision,
            Some(UpdateDecisionReason::CompatibilityChanged),
        );
    }
    if candidate.requires_acknowledgment {
        return UpdateDecision::new(
            UpdateSafety::NeedsDecision,
            Some(UpdateDecisionReason::AcknowledgmentRequired),
        );
    }
    if mode == UpdateMode::Check {
        return UpdateDecision::new(
            UpdateSafety::NeedsDecision,
            Some(UpdateDecisionReason::CheckOnly),
        );
    }
    UpdateDecision::new(UpdateSafety::Safe, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    use crate::domain::{
        ComponentGraph, DesiredOrigin, ResourceId, ResourceKey, ResourceKind, Scope, SourceLocator,
    };

    fn commit(value: char) -> ResolvedRevision {
        ResolvedRevision::GitCommit(
            crate::domain::GitCommit::new(value.to_string().repeat(40)).unwrap(),
        )
    }

    fn candidate() -> UpdateCandidate {
        UpdateCandidate {
            current_revision: Some(commit('a')),
            available_revision: Some(commit('b')),
            resolution_error: None,
            pinned: false,
            drifted: false,
            compatibility_changed: false,
            requires_acknowledgment: false,
            intent: UpdateIntent::Track,
        }
    }

    fn resource(source: Option<Source>, targets: &[&str]) -> DesiredResource {
        let targets = crate::domain::HarnessSet::new(
            targets.iter().map(|value| HarnessId::new(*value).unwrap()),
        )
        .unwrap();
        DesiredResource::new(
            ResourceKey::new(ResourceId::new("plugin:test").unwrap(), Scope::Global),
            ResourceKind::Plugin,
            targets,
            DesiredOrigin::Direct,
            source,
            UpdateIntent::Track,
            ComponentGraph::new([]).unwrap(),
            BTreeMap::new(),
            BTreeMap::new(),
            BTreeSet::new(),
        )
        .unwrap()
    }

    struct SourceResolver;
    impl SourceRevisionResolver for SourceResolver {
        fn resolve(&self, _source: &Source) -> Result<ResolvedRevision, ResolutionError> {
            Ok(commit('b'))
        }
    }

    struct NativeResolver {
        revisions: BTreeMap<String, Option<ResolvedRevision>>,
    }
    impl NativeRevisionResolver for NativeResolver {
        fn resolve(
            &self,
            _resource: &DesiredResource,
            target: &HarnessId,
        ) -> Result<Option<ResolvedRevision>, ResolutionError> {
            Ok(self.revisions.get(target.as_str()).cloned().flatten())
        }
    }

    #[test]
    fn typed_revisions_drive_safe_update_classification() {
        assert_eq!(classify_update(&candidate()), UpdateSafety::Safe);
        let mut pinned = candidate();
        pinned.pinned = true;
        assert_eq!(classify_update(&pinned), UpdateSafety::NeedsDecision);
        let mut drift = candidate();
        drift.drifted = true;
        assert_eq!(classify_update(&drift), UpdateSafety::Blocked);
        let mut unresolved = candidate();
        unresolved.available_revision = None;
        unresolved.resolution_error = Some(ResolutionError::UnreachableSource);
        assert_eq!(classify_update(&unresolved), UpdateSafety::Blocked);
    }

    #[test]
    fn policy_modes_and_intents_never_upgrade_a_decision_to_safe() {
        let mut disabled = candidate();
        disabled.intent = UpdateIntent::Disabled;
        assert_eq!(
            classify_update_with_mode(&disabled, UpdateMode::ApplySafe),
            UpdateDecision::new(
                UpdateSafety::NoUpdate,
                Some(UpdateDecisionReason::DisabledResource)
            )
        );

        let mut pinned = candidate();
        pinned.intent = UpdateIntent::Pinned;
        assert_eq!(
            classify_update_with_mode(&pinned, UpdateMode::ApplySafe),
            UpdateDecision::new(
                UpdateSafety::NeedsDecision,
                Some(UpdateDecisionReason::PinnedResource)
            )
        );
        assert_eq!(
            classify_update_with_mode(&candidate(), UpdateMode::Check),
            UpdateDecision::new(
                UpdateSafety::NeedsDecision,
                Some(UpdateDecisionReason::CheckOnly)
            )
        );
        assert_eq!(
            classify_update_with_mode(&candidate(), UpdateMode::Off),
            UpdateDecision::new(
                UpdateSafety::Blocked,
                Some(UpdateDecisionReason::GlobalModeOff)
            )
        );
    }

    #[test]
    fn policy_reason_precedence_preserves_drift_and_resolution_failures() {
        let mut drifted = candidate();
        drifted.drifted = true;
        drifted.compatibility_changed = true;
        assert_eq!(
            classify_update_with_mode(&drifted, UpdateMode::ApplySafe),
            UpdateDecision::new(UpdateSafety::Blocked, Some(UpdateDecisionReason::Drifted))
        );
        let mut unresolved = candidate();
        unresolved.resolution_error = Some(ResolutionError::TargetDisagreement);
        assert_eq!(
            classify_update_with_mode(&unresolved, UpdateMode::ApplySafe),
            UpdateDecision::new(
                UpdateSafety::Blocked,
                Some(UpdateDecisionReason::ResolutionFailed)
            )
        );
    }

    #[test]
    fn source_resolution_returns_a_concrete_typed_revision() {
        let source = Source::new(
            SourceKind::Git,
            SourceLocator::new("https://example.test/plugin.git").unwrap(),
            None,
        )
        .unwrap();
        let desired = resource(Some(source), &["codex"]);
        let request = UpdateResolutionRequest {
            resource: &desired,
            installed: Some(&commit('a')),
            drifted: false,
            compatibility_changed: false,
            requires_acknowledgment: false,
        };
        let resolved = resolve_candidate(
            &SourceResolver,
            &NativeResolver {
                revisions: BTreeMap::new(),
            },
            request,
        );
        assert_eq!(resolved.available, Some(commit('b')));
        assert!(resolved.error.is_none());
    }

    #[test]
    fn native_target_disagreement_is_not_collapsed() {
        let desired = resource(None, &["codex", "claude"]);
        let revisions = [
            ("codex".to_owned(), Some(commit('a'))),
            ("claude".to_owned(), Some(commit('b'))),
        ]
        .into_iter()
        .collect();
        let request = UpdateResolutionRequest {
            resource: &desired,
            installed: None,
            drifted: false,
            compatibility_changed: false,
            requires_acknowledgment: false,
        };
        let resolved = resolve_candidate(&SourceResolver, &NativeResolver { revisions }, request);
        assert_eq!(resolved.error, Some(ResolutionError::TargetDisagreement));
        assert!(resolved.available.is_none());
    }
}
