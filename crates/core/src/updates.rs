//! Pure update resolution and safety classification shared by foreground and
//! daemon paths.

use crate::domain::{
    DesiredResource, HarnessId, ResolvedRevision, Source, SourceKind, UpdateIntent,
};

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
    pub pinned: bool,
    pub drifted: bool,
    pub compatibility_changed: bool,
    pub requires_acknowledgment: bool,
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
        pinned: resource.update() == UpdateIntent::Pinned,
        drifted: request.drifted,
        compatibility_changed: request.compatibility_changed,
        requires_acknowledgment: request.requires_acknowledgment,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UpdateSafety {
    NoUpdate,
    Safe,
    NeedsDecision,
    Blocked,
}

pub fn classify_update(candidate: &UpdateCandidate) -> UpdateSafety {
    if candidate.available_revision.is_none()
        || candidate.current_revision == candidate.available_revision
    {
        return UpdateSafety::NoUpdate;
    }
    if candidate.drifted {
        return UpdateSafety::Blocked;
    }
    if candidate.pinned || candidate.compatibility_changed || candidate.requires_acknowledgment {
        return UpdateSafety::NeedsDecision;
    }
    UpdateSafety::Safe
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
            pinned: false,
            drifted: false,
            compatibility_changed: false,
            requires_acknowledgment: false,
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
