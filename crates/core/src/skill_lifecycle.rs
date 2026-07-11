//! Pure decision model for standalone skill installation and updates.

use crate::domain::{Fingerprint, Ownership, ResourceKey, Source};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SkillLifecycleDecision {
    Install,
    NoOp,
    Update,
    Drift,
    OwnershipConflict,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SkillLifecycleRequest {
    pub resource: ResourceKey,
    pub source: Source,
    pub desired_fingerprint: Fingerprint,
    pub installed_fingerprint: Option<Fingerprint>,
    pub observed_fingerprint: Option<Fingerprint>,
    pub ownership: Option<Ownership>,
}

/// Decide whether an explicit source is a first install, repeat, SHA-backed
/// update, drift, or ownership conflict. This function never reads or writes
/// a source and never treats a changed source as an implicit confirmation.
pub fn decide(request: &SkillLifecycleRequest) -> SkillLifecycleDecision {
    if matches!(request.ownership, Some(Ownership::Unmanaged)) {
        return SkillLifecycleDecision::OwnershipConflict;
    }
    if let (Some(installed), Some(observed)) = (
        request.installed_fingerprint.as_ref(),
        request.observed_fingerprint.as_ref(),
    ) && installed != observed
    {
        return SkillLifecycleDecision::Drift;
    }
    match request.installed_fingerprint.as_ref() {
        None => SkillLifecycleDecision::Install,
        Some(installed) if installed == &request.desired_fingerprint => {
            SkillLifecycleDecision::NoOp
        }
        Some(_) => SkillLifecycleDecision::Update,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{FingerprintAlgorithm, ResourceId, Scope, SourceKind, SourceLocator};

    fn request(installed: Option<char>, observed: Option<char>) -> SkillLifecycleRequest {
        let fingerprint = |value: char| {
            Fingerprint::new(FingerprintAlgorithm::Sha256, value.to_string().repeat(64)).unwrap()
        };
        SkillLifecycleRequest {
            resource: ResourceKey::new(ResourceId::new("skill:demo").unwrap(), Scope::Global),
            source: Source::new(
                SourceKind::Local,
                SourceLocator::new("/tmp/demo").unwrap(),
                None,
            )
            .unwrap(),
            desired_fingerprint: fingerprint('b'),
            installed_fingerprint: installed.map(fingerprint),
            observed_fingerprint: observed.map(fingerprint),
            ownership: Some(Ownership::Skilltap),
        }
    }

    #[test]
    fn git_or_tree_fingerprint_changes_are_updates_and_repeat_is_noop() {
        assert_eq!(
            decide(&request(None, None)),
            SkillLifecycleDecision::Install
        );
        assert_eq!(
            decide(&request(Some('b'), Some('b'))),
            SkillLifecycleDecision::NoOp
        );
        assert_eq!(
            decide(&request(Some('a'), Some('a'))),
            SkillLifecycleDecision::Update
        );
        assert_eq!(
            decide(&request(Some('a'), Some('b'))),
            SkillLifecycleDecision::Drift
        );
    }

    #[test]
    fn unmanaged_content_blocks_before_comparing_source() {
        let mut request = request(None, None);
        request.ownership = Some(Ownership::Unmanaged);
        assert_eq!(decide(&request), SkillLifecycleDecision::OwnershipConflict);
    }
}
