//! Pure update safety classification shared by foreground and daemon paths.

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdateCandidate {
    pub current_revision: Option<String>,
    pub available_revision: Option<String>,
    pub pinned: bool,
    pub drifted: bool,
    pub compatibility_changed: bool,
    pub requires_acknowledgment: bool,
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

    fn candidate() -> UpdateCandidate {
        UpdateCandidate {
            current_revision: Some("a".into()),
            available_revision: Some("b".into()),
            pinned: false,
            drifted: false,
            compatibility_changed: false,
            requires_acknowledgment: false,
        }
    }

    #[test]
    fn safe_updates_require_no_new_user_decision() {
        assert_eq!(classify_update(&candidate()), UpdateSafety::Safe);
        let mut pinned = candidate();
        pinned.pinned = true;
        assert_eq!(classify_update(&pinned), UpdateSafety::NeedsDecision);
        let mut drift = candidate();
        drift.drifted = true;
        assert_eq!(classify_update(&drift), UpdateSafety::Blocked);
    }
}
