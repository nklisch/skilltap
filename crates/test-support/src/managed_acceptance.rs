use std::{collections::BTreeSet, fmt};

/// One behavior every managed-projection adapter must prove through lifecycle
/// dispatch. The production-aware runner lives in the consuming crate so this
/// fixture crate remains dependency-neutral.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ManagedAcceptanceScenario {
    ApplyProjection,
    ProjectionEvidence,
    RemovalWithoutCheckout,
    Compatibility,
    Ownership,
    PendingRecovery,
    FreshLoadVerification,
    ImmediateRepeat,
}

impl ManagedAcceptanceScenario {
    pub const ALL: [Self; 8] = [
        Self::ApplyProjection,
        Self::ProjectionEvidence,
        Self::RemovalWithoutCheckout,
        Self::Compatibility,
        Self::Ownership,
        Self::PendingRecovery,
        Self::FreshLoadVerification,
        Self::ImmediateRepeat,
    ];

    const fn required_evidence(self) -> &'static [ManagedAcceptanceCheck] {
        use ManagedAcceptanceCheck as Check;

        match self {
            Self::ApplyProjection => &[Check::OneApplyCheckout, Check::CompleteSkillTree],
            Self::ProjectionEvidence => &[
                Check::Manifest,
                Check::CurrentFingerprint,
                Check::DesiredFingerprint,
            ],
            Self::RemovalWithoutCheckout => &[Check::RemoveWithoutCheckout],
            Self::Compatibility => &[Check::OmissionAcknowledgment, Check::RequiredUnsupported],
            Self::Ownership => &[
                Check::OwnedDestination,
                Check::DriftRejected,
                Check::UnownedRejected,
                Check::UpdateRequired,
                Check::TargetStateIsolated,
            ],
            Self::PendingRecovery => &[Check::PendingRetry, Check::RetryNoChange],
            Self::FreshLoadVerification => &[Check::FreshLoadObserved],
            Self::ImmediateRepeat => &[
                Check::ImmediateRepeatNoChange,
                Check::NoDuplicateArtifacts,
                Check::NoDuplicateState,
            ],
        }
    }
}

/// Dependency-neutral evidence labels returned by a production-aware matrix
/// runner after its concrete assertions pass.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ManagedAcceptanceCheck {
    OneApplyCheckout,
    CompleteSkillTree,
    Manifest,
    CurrentFingerprint,
    DesiredFingerprint,
    RemoveWithoutCheckout,
    OmissionAcknowledgment,
    RequiredUnsupported,
    OwnedDestination,
    DriftRejected,
    UnownedRejected,
    UpdateRequired,
    TargetStateIsolated,
    PendingRetry,
    RetryNoChange,
    FreshLoadObserved,
    ImmediateRepeatNoChange,
    NoDuplicateArtifacts,
    NoDuplicateState,
}

/// Evidence produced for one matrix scenario.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedAcceptanceEvidence {
    checks: BTreeSet<ManagedAcceptanceCheck>,
}

impl ManagedAcceptanceEvidence {
    pub fn new(checks: impl IntoIterator<Item = ManagedAcceptanceCheck>) -> Self {
        Self {
            checks: checks.into_iter().collect(),
        }
    }

    pub fn checks(&self) -> impl Iterator<Item = ManagedAcceptanceCheck> + '_ {
        self.checks.iter().copied()
    }
}

/// Target load-surface descriptors shared by managed-projection acceptance
/// runners. Production crates translate `id` and paths into their validated
/// domain types rather than making test-support depend on them.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedProjectionProfile {
    id: &'static str,
    catalog_destinations: &'static [&'static str],
    mcp_destination: Option<&'static str>,
    skill_destination: &'static str,
}

impl ManagedProjectionProfile {
    pub const fn codex() -> Self {
        Self::new(
            "codex",
            &[
                ".agents/plugins/marketplace.json",
                ".claude-plugin/marketplace.json",
            ],
            Some(".codex/config.toml"),
            ".agents/skills",
        )
    }

    /// Construct a dependency-neutral profile for a test adapter or a future
    /// concrete adapter acceptance suite.
    pub const fn new(
        id: &'static str,
        catalog_destinations: &'static [&'static str],
        mcp_destination: Option<&'static str>,
        skill_destination: &'static str,
    ) -> Self {
        Self {
            id,
            catalog_destinations,
            mcp_destination,
            skill_destination,
        }
    }

    pub const fn id(&self) -> &'static str {
        self.id
    }

    pub const fn catalog_destinations(&self) -> &'static [&'static str] {
        self.catalog_destinations
    }

    pub const fn mcp_destination(&self) -> Option<&'static str> {
        self.mcp_destination
    }

    pub const fn skill_destination(&self) -> &'static str {
        self.skill_destination
    }
}

/// Complete evidence returned by a successful managed-projection matrix run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedAcceptanceReport {
    profile_id: &'static str,
    scenarios: BTreeSet<ManagedAcceptanceScenario>,
    checks: BTreeSet<ManagedAcceptanceCheck>,
}

impl ManagedAcceptanceReport {
    pub const fn profile_id(&self) -> &'static str {
        self.profile_id
    }

    pub fn scenarios(&self) -> impl Iterator<Item = ManagedAcceptanceScenario> + '_ {
        self.scenarios.iter().copied()
    }

    pub fn checks(&self) -> impl Iterator<Item = ManagedAcceptanceCheck> + '_ {
        self.checks.iter().copied()
    }

    pub fn passed(&self) -> bool {
        self.scenarios.len() == ManagedAcceptanceScenario::ALL.len()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedAcceptanceError {
    profile_id: &'static str,
    scenario: ManagedAcceptanceScenario,
    missing: Vec<ManagedAcceptanceCheck>,
}

impl ManagedAcceptanceError {
    pub const fn profile_id(&self) -> &'static str {
        self.profile_id
    }

    pub const fn scenario(&self) -> ManagedAcceptanceScenario {
        self.scenario
    }

    pub fn missing(&self) -> &[ManagedAcceptanceCheck] {
        &self.missing
    }
}

impl fmt::Display for ManagedAcceptanceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "managed-projection profile `{}` omitted {:?} evidence for {:?}",
            self.profile_id, self.missing, self.scenario
        )
    }
}

impl std::error::Error for ManagedAcceptanceError {}

/// Run the shared managed-projection contract through a caller-supplied,
/// production-aware lifecycle runner.
///
/// The callback must execute real dispatch and assertions for the supplied
/// scenario, then return only the evidence labels it proved. Keeping the
/// callback in the consuming crate avoids a core/harnesses/CLI dependency
/// cycle while this function enforces one complete scenario set for every
/// opted-in adapter.
pub fn managed_acceptance_matrix(
    profile: &ManagedProjectionProfile,
    mut exercise: impl FnMut(
        &ManagedProjectionProfile,
        ManagedAcceptanceScenario,
    ) -> ManagedAcceptanceEvidence,
) -> Result<ManagedAcceptanceReport, ManagedAcceptanceError> {
    let mut scenarios = BTreeSet::new();
    let mut checks = BTreeSet::new();

    for scenario in ManagedAcceptanceScenario::ALL {
        let evidence = exercise(profile, scenario);
        let missing = scenario
            .required_evidence()
            .iter()
            .copied()
            .filter(|required| !evidence.checks.contains(required))
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            return Err(ManagedAcceptanceError {
                profile_id: profile.id,
                scenario,
                missing,
            });
        }
        scenarios.insert(scenario);
        checks.extend(evidence.checks());
    }

    Ok(ManagedAcceptanceReport {
        profile_id: profile.id,
        scenarios,
        checks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrix_requires_each_scenarios_declared_evidence() {
        let error = managed_acceptance_matrix(&ManagedProjectionProfile::codex(), |_, scenario| {
            if scenario == ManagedAcceptanceScenario::ApplyProjection {
                ManagedAcceptanceEvidence::new([ManagedAcceptanceCheck::OneApplyCheckout])
            } else {
                ManagedAcceptanceEvidence::new(scenario.required_evidence().iter().copied())
            }
        })
        .unwrap_err();

        assert_eq!(error.profile_id(), "codex");
        assert_eq!(error.scenario(), ManagedAcceptanceScenario::ApplyProjection);
        assert_eq!(
            error.missing(),
            &[ManagedAcceptanceCheck::CompleteSkillTree]
        );
    }

    #[test]
    fn complete_matrix_reports_every_scenario_and_check() {
        let report =
            managed_acceptance_matrix(&ManagedProjectionProfile::codex(), |_, scenario| {
                ManagedAcceptanceEvidence::new(scenario.required_evidence().iter().copied())
            })
            .unwrap();

        assert!(report.passed());
        assert_eq!(report.profile_id(), "codex");
        assert_eq!(report.scenarios().count(), 8);
        assert_eq!(report.checks().count(), 19);
    }
}
