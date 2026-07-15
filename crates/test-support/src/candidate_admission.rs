use std::{collections::BTreeSet, fmt};

/// Candidates admitted to the registry with read-only authority only.
pub const OBSERVE_ONLY_CANDIDATES: [&str; 3] = ["cursor", "zoo", "zcode"];

/// Evidence required before a candidate can be admitted as a mutable target.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CandidateAdmissionCheck {
    /// A reliable target identity exists, even when an installed executable or
    /// extension version is not available.
    ReliableTargetIdentity,
    /// At least one safe, source-documented read surface is available without
    /// using a cache, editor database, auth state, or guessed command.
    SafeDocumentedReadSurface,
    /// Exact installed identity remains a mutation prerequisite.
    ExactInstallationIdentity,
    DocumentedGlobalSkillRoot,
    DocumentedProjectSkillRoot,
    CompleteSkillSiblings,
    SkillPrecedenceAndReload,
    DocumentedGlobalMcpFile,
    DocumentedProjectMcpFile,
    McpSchemaAndPrecedence,
    EffectiveReloadObservation,
    UnknownFieldAndSiblingPreservation,
    OwnershipSafeUpdateAndRemoval,
    CacheIndependentBoundary,
    SharedAdapterAcceptance,
    ImmediateRepeatNoChange,
}

impl CandidateAdmissionCheck {
    /// The complete, stable matrix order used by every candidate runner.
    pub const ALL: [Self; 16] = [
        Self::ReliableTargetIdentity,
        Self::SafeDocumentedReadSurface,
        Self::ExactInstallationIdentity,
        Self::DocumentedGlobalSkillRoot,
        Self::DocumentedProjectSkillRoot,
        Self::CompleteSkillSiblings,
        Self::SkillPrecedenceAndReload,
        Self::DocumentedGlobalMcpFile,
        Self::DocumentedProjectMcpFile,
        Self::McpSchemaAndPrecedence,
        Self::EffectiveReloadObservation,
        Self::UnknownFieldAndSiblingPreservation,
        Self::OwnershipSafeUpdateAndRemoval,
        Self::CacheIndependentBoundary,
        Self::SharedAdapterAcceptance,
        Self::ImmediateRepeatNoChange,
    ];

    /// Checks that establish deterministic, cache-independent observation.
    ///
    /// Mutation checks intentionally do not appear here: passing the read-only
    /// surface is enough for an observe-only disposition, never for admission.
    const READ_ONLY: [Self; 2] = [
        Self::ReliableTargetIdentity,
        Self::SafeDocumentedReadSurface,
    ];

    fn read_only_proven(checks: &BTreeSet<Self>) -> bool {
        Self::READ_ONLY
            .iter()
            .all(|required| checks.contains(required))
    }
}

/// One candidate's concrete evidence labels after its runner's assertions pass.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateAdmissionEvidence {
    checks: BTreeSet<CandidateAdmissionCheck>,
}

impl CandidateAdmissionEvidence {
    pub fn new(checks: impl IntoIterator<Item = CandidateAdmissionCheck>) -> Self {
        Self {
            checks: checks.into_iter().collect(),
        }
    }

    pub fn checks(&self) -> impl Iterator<Item = CandidateAdmissionCheck> + '_ {
        self.checks.iter().copied()
    }

    pub fn contains(&self, check: CandidateAdmissionCheck) -> bool {
        self.checks.contains(&check)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateDisposition {
    Admitted,
    ObserveOnly,
    Blocked,
}

/// The disposition and missing evidence for one independently evaluated candidate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateAdmissionReport {
    candidate: &'static str,
    disposition: CandidateDisposition,
    checks: BTreeSet<CandidateAdmissionCheck>,
    missing: Vec<CandidateAdmissionCheck>,
}

impl CandidateAdmissionReport {
    pub const fn candidate(&self) -> &'static str {
        self.candidate
    }

    pub const fn disposition(&self) -> CandidateDisposition {
        self.disposition
    }

    pub fn checks(&self) -> impl Iterator<Item = CandidateAdmissionCheck> + '_ {
        self.checks.iter().copied()
    }

    pub fn missing(&self) -> &[CandidateAdmissionCheck] {
        &self.missing
    }

    pub const fn is_admitted(&self) -> bool {
        matches!(self.disposition, CandidateDisposition::Admitted)
    }

    pub const fn is_observe_only(&self) -> bool {
        matches!(self.disposition, CandidateDisposition::ObserveOnly)
    }

    pub const fn is_blocked(&self) -> bool {
        matches!(self.disposition, CandidateDisposition::Blocked)
    }
}

impl fmt::Display for CandidateDisposition {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Admitted => "admitted",
            Self::ObserveOnly => "observe_only",
            Self::Blocked => "blocked",
        })
    }
}

/// Evaluate the dependency-neutral candidate admission contract.
///
/// The callback is deliberately a check predicate rather than an evidence
/// constructor. A production-aware runner must perform its source, path,
/// version, reload, ownership, and idempotency assertions before returning
/// `true`; labels alone cannot satisfy this gate.
pub fn candidate_admission_gate(
    candidate: &'static str,
    mut exercise: impl FnMut(CandidateAdmissionCheck) -> bool,
) -> CandidateAdmissionReport {
    let checks = CandidateAdmissionCheck::ALL
        .into_iter()
        .filter(|check| exercise(*check))
        .collect::<BTreeSet<_>>();
    let missing = CandidateAdmissionCheck::ALL
        .into_iter()
        .filter(|check| !checks.contains(check))
        .collect::<Vec<_>>();
    let disposition = if missing.is_empty() {
        CandidateDisposition::Admitted
    } else if CandidateAdmissionCheck::read_only_proven(&checks) {
        CandidateDisposition::ObserveOnly
    } else {
        CandidateDisposition::Blocked
    };

    CandidateAdmissionReport {
        candidate,
        disposition,
        checks,
        missing,
    }
}

/// Produce the final aggregate reports for the three narrow observe-only
/// candidates. The reports intentionally omit exact installed identity and all
/// mutation/effective checks; those missing checks remain visible evidence, not
/// latent authority.
pub fn observe_only_candidate_admission_reports() -> [CandidateAdmissionReport; 3] {
    OBSERVE_ONLY_CANDIDATES.map(|candidate| {
        candidate_admission_gate(candidate, |check| match candidate {
            "cursor" => matches!(
                check,
                CandidateAdmissionCheck::ReliableTargetIdentity
                    | CandidateAdmissionCheck::SafeDocumentedReadSurface
                    | CandidateAdmissionCheck::DocumentedGlobalSkillRoot
                    | CandidateAdmissionCheck::DocumentedProjectSkillRoot
                    | CandidateAdmissionCheck::DocumentedGlobalMcpFile
                    | CandidateAdmissionCheck::DocumentedProjectMcpFile
                    | CandidateAdmissionCheck::McpSchemaAndPrecedence
            ),
            "zoo" => matches!(
                check,
                CandidateAdmissionCheck::ReliableTargetIdentity
                    | CandidateAdmissionCheck::SafeDocumentedReadSurface
                    | CandidateAdmissionCheck::DocumentedGlobalSkillRoot
                    | CandidateAdmissionCheck::DocumentedProjectSkillRoot
                    | CandidateAdmissionCheck::DocumentedProjectMcpFile
            ),
            "zcode" => matches!(
                check,
                CandidateAdmissionCheck::ReliableTargetIdentity
                    | CandidateAdmissionCheck::SafeDocumentedReadSurface
                    | CandidateAdmissionCheck::DocumentedGlobalSkillRoot
                    | CandidateAdmissionCheck::DocumentedGlobalMcpFile
                    | CandidateAdmissionCheck::DocumentedProjectMcpFile
            ),
            _ => false,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn complete(check: CandidateAdmissionCheck) -> bool {
        check != CandidateAdmissionCheck::OwnershipSafeUpdateAndRemoval
    }

    #[test]
    fn complete_evidence_is_admitted() {
        let report = candidate_admission_gate("cursor", |_| true);

        assert_eq!(report.candidate(), "cursor");
        assert_eq!(report.disposition(), CandidateDisposition::Admitted);
        assert!(report.is_admitted());
        assert!(report.missing().is_empty());
        assert_eq!(report.checks().count(), CandidateAdmissionCheck::ALL.len());
    }

    #[test]
    fn one_missing_mutation_check_is_observe_only() {
        let report = candidate_admission_gate("zoo", complete);

        assert_eq!(report.disposition(), CandidateDisposition::ObserveOnly);
        assert!(report.is_observe_only());
        assert_eq!(
            report.missing(),
            &[CandidateAdmissionCheck::OwnershipSafeUpdateAndRemoval]
        );
    }

    #[test]
    fn missing_deterministic_observation_blocks_even_with_other_evidence() {
        let report = candidate_admission_gate("zcode", |check| {
            check != CandidateAdmissionCheck::SafeDocumentedReadSurface
        });

        assert_eq!(report.disposition(), CandidateDisposition::Blocked);
        assert!(report.is_blocked());
        assert_eq!(
            report.missing(),
            &[CandidateAdmissionCheck::SafeDocumentedReadSurface]
        );
    }

    #[test]
    fn missing_reliable_identity_blocks_observation() {
        let report = candidate_admission_gate("cursor", |check| {
            check != CandidateAdmissionCheck::ReliableTargetIdentity
        });

        assert_eq!(report.disposition(), CandidateDisposition::Blocked);
        assert!(
            report
                .missing()
                .contains(&CandidateAdmissionCheck::ReliableTargetIdentity)
        );
    }

    #[test]
    fn evidence_deduplicates_checks_without_granting_disposition() {
        let evidence = CandidateAdmissionEvidence::new([
            CandidateAdmissionCheck::ExactInstallationIdentity,
            CandidateAdmissionCheck::ExactInstallationIdentity,
        ]);

        assert!(evidence.contains(CandidateAdmissionCheck::ExactInstallationIdentity));
        assert_eq!(evidence.checks().count(), 1);
    }

    #[test]
    fn observe_only_candidate_reports_match_the_relaxed_disposition_matrix() {
        let reports = observe_only_candidate_admission_reports();

        assert_eq!(
            reports
                .iter()
                .map(CandidateAdmissionReport::candidate)
                .collect::<Vec<_>>(),
            OBSERVE_ONLY_CANDIDATES.to_vec()
        );
        assert!(
            reports
                .iter()
                .all(CandidateAdmissionReport::is_observe_only)
        );
        assert!(reports.iter().all(|report| {
            report
                .missing()
                .contains(&CandidateAdmissionCheck::ExactInstallationIdentity)
        }));
    }
}
