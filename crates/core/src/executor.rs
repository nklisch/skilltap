//! Locking, dependency-aware execution of validated reconciliation plans.
//!
//! The executor deliberately knows nothing about marketplaces, plugins, skills, or
//! instruction files.  Resource adapters implement [`ExecutionPort`], while the
//! caller supplies the state publication boundary through [`ExecutionJournal`].
//! This keeps the safety protocol in the core and leaves native mutation at the
//! harness boundary.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use crate::{
    domain::{
        AcknowledgmentRequirement, ApplyOutcome, ApplyResult, AttentionReason, EvidenceCode,
        EvidenceDetail, Operation, OperationClass, OperationId, OperationOutcome, OperationResult,
        Plan,
    },
    operation_graph::{GraphError, dependency_waves},
    runtime::{ConfigurationLock, ConfigurationLockGuard, RuntimeError},
};

/// A synchronous port for revalidation and one resolved operation mutation.
///
/// Revalidation runs while the cooperative configuration lock is held and must
/// verify every identity/fingerprint that can be affected by `plan`.  The port
/// may return [`ExecutionError::Apply`] for a native action that failed; the
/// executor turns that failure into a journaled failed operation and continues
/// with independent work.
pub trait ExecutionPort {
    fn revalidate(&self, plan: &Plan) -> Result<(), ExecutionError>;
    fn apply(&self, operation: &Operation) -> Result<OperationOutcome, ExecutionError>;
}

/// Publication boundary for operation results.
///
/// Implementations are expected to atomically publish the result into the
/// machine state document.  The executor records a `Pending` result before an
/// executable action and the terminal result immediately after it, which makes
/// an interrupted process observable without introducing a second journal file.
pub trait ExecutionJournal {
    fn record(&self, result: &OperationResult) -> Result<(), ExecutionError>;
}

/// The result of a completed execution pass.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionReport {
    pub result: ApplyResult,
    pub changed: bool,
}

/// Exact foreground authorization for partial operations.
///
/// The map is intentionally keyed by operation id and stores the complete
/// requirement from the current plan. This prevents a caller from approving a
/// selector or consequence reconstructed from stale or broader context.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ExecutionAcknowledgments {
    accepted: BTreeMap<OperationId, AcknowledgmentRequirement>,
}

impl ExecutionAcknowledgments {
    /// Accept every partial operation in this already-built plan. This is the
    /// only convenience used by a foreground `--yes` path; it cannot add or
    /// widen an operation.
    pub fn foreground_all(plan: &Plan) -> Self {
        Self {
            accepted: plan
                .iter()
                .filter(|(_, operation)| operation.class() == OperationClass::Partial)
                .map(|(id, operation)| (id.clone(), operation.acknowledgment().clone()))
                .collect(),
        }
    }

    /// Validate exact operation ids and requirements against the current plan.
    pub fn new(
        plan: &Plan,
        accepted: impl IntoIterator<Item = (OperationId, AcknowledgmentRequirement)>,
    ) -> Result<Self, GraphError> {
        let mut values = BTreeMap::new();
        for (id, requirement) in accepted {
            if values.insert(id.clone(), requirement.clone()).is_some() {
                return Err(GraphError::InvalidAcknowledgment);
            }
            let Some(operation) = plan.get(&id) else {
                return Err(GraphError::UnknownOperation { operation: id });
            };
            if operation.class() != OperationClass::Partial
                || operation.acknowledgment() != &requirement
            {
                return Err(GraphError::InvalidAcknowledgment);
            }
        }
        Ok(Self { accepted: values })
    }

    pub fn accepts(&self, operation: &Operation) -> bool {
        operation.class() != OperationClass::Partial
            || self
                .accepted
                .get(operation.id())
                .is_some_and(|requirement| requirement == operation.acknowledgment())
    }
}

/// Errors raised while acquiring the lock, validating the plan boundary, or
/// publishing state.  An [`ExecutionError::Apply`] is intentionally recoverable:
/// it is converted into a failed [`OperationResult`] and does not abort
/// independent operations.
#[derive(Debug)]
pub enum ExecutionError {
    Lock(RuntimeError),
    Release(RuntimeError),
    Revalidation {
        code: EvidenceCode,
        detail: EvidenceDetail,
    },
    /// A resolved native/filesystem action failed.  The reason must be an
    /// `OperationFailed` attention reason so that it can be represented by the
    /// validated operation-result contract.
    Apply {
        reason: AttentionReason,
    },
    /// A journal boundary failed.  `after_apply` is true when native work may
    /// already have happened and recovery must begin with fresh observation.
    Journal {
        operation: OperationId,
        after_apply: bool,
        source: Box<ExecutionError>,
    },
    JournalBoundary {
        code: EvidenceCode,
        detail: EvidenceDetail,
    },
    InvalidOutcome {
        operation: OperationId,
    },
    Graph(GraphError),
    Contract(crate::domain::OperationContractError),
}

impl ExecutionError {
    /// Construct an apply failure suitable for returning from an
    /// [`ExecutionPort`].
    pub fn apply_failure(reason: AttentionReason) -> Self {
        Self::Apply { reason }
    }

    /// Construct a redacted revalidation error for an adapter boundary.
    pub fn revalidation(code: EvidenceCode, detail: EvidenceDetail) -> Self {
        Self::Revalidation { code, detail }
    }

    /// Construct a redacted persistence-boundary failure from an adapter.
    pub fn journal_failure(code: EvidenceCode, detail: EvidenceDetail) -> Self {
        Self::JournalBoundary { code, detail }
    }
}

impl fmt::Display for ExecutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lock(error) => {
                write!(formatter, "configuration lock acquisition failed: {error}")
            }
            Self::Release(error) => write!(formatter, "configuration lock release failed: {error}"),
            Self::Revalidation { code, detail } => {
                write!(formatter, "revalidation failed ({code}): {detail}")
            }
            Self::Apply { reason } => {
                write!(formatter, "operation apply failed")?;
                if let Some(code) = reason.code() {
                    write!(formatter, " ({code})")?;
                }
                Ok(())
            }
            Self::Journal {
                operation,
                after_apply,
                source,
            } => write!(
                formatter,
                "state journal failed for operation `{operation}`{}: {source}",
                if *after_apply {
                    " after native action"
                } else {
                    ""
                }
            ),
            Self::JournalBoundary { code, detail } => {
                write!(
                    formatter,
                    "state journal boundary failed ({code}): {detail}"
                )
            }
            Self::InvalidOutcome { operation } => {
                write!(
                    formatter,
                    "operation `{operation}` returned an invalid execution outcome"
                )
            }
            Self::Graph(error) => error.fmt(formatter),
            Self::Contract(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for ExecutionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Lock(error) | Self::Release(error) => Some(error),
            Self::Journal { source, .. } => Some(source),
            Self::Graph(error) => Some(error),
            Self::Contract(error) => Some(error),
            Self::Revalidation { .. }
            | Self::Apply { .. }
            | Self::JournalBoundary { .. }
            | Self::InvalidOutcome { .. } => None,
        }
    }
}

impl From<GraphError> for ExecutionError {
    fn from(error: GraphError) -> Self {
        Self::Graph(error)
    }
}

impl From<crate::domain::OperationContractError> for ExecutionError {
    fn from(error: crate::domain::OperationContractError) -> Self {
        Self::Contract(error)
    }
}

/// Execute a validated plan while holding the process-wide configuration lock.
///
/// Plan graph validation is pure and happens before lock acquisition.  Once the
/// lock is held, revalidation is performed before any journal or native call.
/// Safe operations proceed in stable dependency waves; attention operations are
/// blocked, dependents of every non-success result are skipped, and independent
/// operations continue.  A journal error after an action is surfaced as an
/// explicit `Journal { after_apply: true, .. }` error rather than a false
/// success report.
pub fn execute_plan<L, P, J>(
    lock: &L,
    lock_path: &crate::domain::AbsolutePath,
    port: &P,
    journal: &J,
    plan: &Plan,
) -> Result<ExecutionReport, ExecutionError>
where
    L: ConfigurationLock,
    P: ExecutionPort + ?Sized,
    J: ExecutionJournal + ?Sized,
{
    execute_plan_with_acknowledgments(
        lock,
        lock_path,
        port,
        journal,
        plan,
        &ExecutionAcknowledgments::default(),
    )
}

/// Execute a plan with exact foreground acknowledgment entries.
pub fn execute_plan_with_acknowledgments<L, P, J>(
    lock: &L,
    lock_path: &crate::domain::AbsolutePath,
    port: &P,
    journal: &J,
    plan: &Plan,
    acknowledgments: &ExecutionAcknowledgments,
) -> Result<ExecutionReport, ExecutionError>
where
    L: ConfigurationLock,
    P: ExecutionPort + ?Sized,
    J: ExecutionJournal + ?Sized,
{
    let waves = dependency_waves(plan)?;
    let guard = lock.try_acquire(lock_path).map_err(ExecutionError::Lock)?;

    let execution = execute_locked(port, journal, plan, &waves, acknowledgments);
    let release = guard.release().map_err(ExecutionError::Release);
    match (execution, release) {
        (Err(error), _) => Err(error),
        (Ok(report), Ok(())) => Ok(report),
        (Ok(_), Err(error)) => Err(error),
    }
}

fn execute_locked<P, J>(
    port: &P,
    journal: &J,
    plan: &Plan,
    waves: &[crate::operation_graph::OperationWave],
    acknowledgments: &ExecutionAcknowledgments,
) -> Result<ExecutionReport, ExecutionError>
where
    P: ExecutionPort + ?Sized,
    J: ExecutionJournal + ?Sized,
{
    // No state boundary is touched until the affected observations have been
    // revalidated under the lock.
    port.revalidate(plan)?;

    let mut results = BTreeMap::<OperationId, OperationResult>::new();
    let mut changed = false;

    for wave in waves {
        for operation_id in &wave.operations {
            let operation = plan
                .get(operation_id)
                .expect("dependency waves only contain plan operations");

            let blockers = operation
                .dependencies()
                .iter()
                .filter_map(|dependency| {
                    let dependency_id = dependency.operation_id();
                    let result = results
                        .get(dependency_id)
                        .expect("dependencies are completed in an earlier wave");
                    (!matches!(
                        result.outcome(),
                        OperationOutcome::Applied | OperationOutcome::NoChange
                    ))
                    .then(|| dependency_id.clone())
                })
                .collect::<BTreeSet<_>>();

            let result = if !blockers.is_empty() {
                OperationResult::new(
                    operation_id.clone(),
                    OperationOutcome::SkippedDependency {
                        dependencies: blockers,
                    },
                )?
            } else {
                execute_operation(port, journal, operation, acknowledgments, &mut changed)?
            };

            // `execute_operation` journals terminal outcomes itself for
            // executable and attention operations.  Dependency skips are
            // created here, so publish them at the exact point they become
            // known.
            if matches!(result.outcome(), OperationOutcome::SkippedDependency { .. }) {
                record(journal, &result, false)?;
            }
            results.insert(operation_id.clone(), result);
        }
    }

    let outcome = if results
        .values()
        .any(|result| matches!(result.outcome(), OperationOutcome::Failed { .. }))
    {
        ApplyOutcome::PartialFailure
    } else if results.values().any(|result| {
        matches!(
            result.outcome(),
            OperationOutcome::Blocked { .. }
                | OperationOutcome::SkippedDependency { .. }
                | OperationOutcome::Pending
        )
    }) {
        ApplyOutcome::AttentionRequired
    } else {
        ApplyOutcome::Succeeded
    };
    let result = ApplyResult::new(plan.clone(), outcome, results.into_values())?;
    Ok(ExecutionReport { result, changed })
}

fn execute_operation<P, J>(
    port: &P,
    journal: &J,
    operation: &Operation,
    acknowledgments: &ExecutionAcknowledgments,
    changed: &mut bool,
) -> Result<OperationResult, ExecutionError>
where
    P: ExecutionPort + ?Sized,
    J: ExecutionJournal + ?Sized,
{
    let terminal = match operation.class() {
        OperationClass::NoOp => OperationOutcome::NoChange,
        OperationClass::Unsupported | OperationClass::Conflict => {
            let reason = operation
                .attention()
                .cloned()
                .expect("validated attention operation carries an attention reason");
            OperationOutcome::Blocked { reason }
        }
        OperationClass::Partial if !acknowledgments.accepts(operation) => {
            let reason = operation
                .attention()
                .cloned()
                .expect("validated partial operation carries an attention reason");
            OperationOutcome::Blocked { reason }
        }
        OperationClass::Partial
        | OperationClass::SafeNative
        | OperationClass::SafeFaithfulEquivalent
        | OperationClass::SafeMaterialization => {
            let pending = OperationResult::new(operation.id().clone(), OperationOutcome::Pending)?;
            record(journal, &pending, false)?;

            let outcome = match port.apply(operation) {
                Ok(OperationOutcome::Applied) => {
                    *changed = true;
                    OperationOutcome::Applied
                }
                Ok(OperationOutcome::NoChange) => OperationOutcome::NoChange,
                Ok(OperationOutcome::Failed { reason }) => OperationOutcome::Failed { reason },
                Ok(OperationOutcome::Blocked { reason }) => OperationOutcome::Blocked { reason },
                Ok(OperationOutcome::Pending) | Ok(OperationOutcome::SkippedDependency { .. }) => {
                    return Err(ExecutionError::InvalidOutcome {
                        operation: operation.id().clone(),
                    });
                }
                Err(ExecutionError::Apply { reason }) => OperationOutcome::Failed { reason },
                Err(error) => return Err(error),
            };
            let result = OperationResult::new(operation.id().clone(), outcome)?;
            // The adapter has completed its action boundary regardless of
            // whether the native command reported success, no-op, or failure.
            // A publication failure here therefore always needs recovery from
            // fresh observation.
            record(journal, &result, true)?;
            return Ok(result);
        }
    };

    let result = OperationResult::new(operation.id().clone(), terminal)?;
    record(journal, &result, false)?;
    Ok(result)
}

fn record<J>(journal: &J, result: &OperationResult, after_apply: bool) -> Result<(), ExecutionError>
where
    J: ExecutionJournal + ?Sized,
{
    journal
        .record(result)
        .map_err(|source| ExecutionError::Journal {
            operation: result.operation_id().clone(),
            after_apply,
            source: Box::new(source),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{
            AffectedSurface, CompatibilityClass, CompatibilityEvidence, CompatibilityResult,
            ConsequenceCode, ConsequenceSummary, HarnessId, MaterialConsequence, OperationAction,
            OperationReason, OperationSelector, OperationSemantics, Provenance, ResourceId,
            ResourceKey, Reversibility, Scope, TransferFidelity,
        },
        runtime::{ConfigurationLockGuard, RuntimeError},
    };
    use std::cell::RefCell;

    struct FakeGuard {
        path: crate::domain::AbsolutePath,
    }

    impl ConfigurationLockGuard for FakeGuard {
        fn path(&self) -> &crate::domain::AbsolutePath {
            &self.path
        }

        fn release(self) -> Result<(), RuntimeError> {
            Ok(())
        }
    }

    struct FakeLock {
        acquired: RefCell<bool>,
        contended: bool,
    }

    impl ConfigurationLock for FakeLock {
        type Guard = FakeGuard;

        fn try_acquire(
            &self,
            path: &crate::domain::AbsolutePath,
        ) -> Result<Self::Guard, RuntimeError> {
            *self.acquired.borrow_mut() = true;
            if self.contended {
                return Err(RuntimeError::LockContended { path: path.clone() });
            }
            Ok(FakeGuard { path: path.clone() })
        }
    }

    #[derive(Default)]
    struct FakePort {
        revalidated: RefCell<bool>,
        fail_revalidation: bool,
        calls: RefCell<Vec<OperationId>>,
        outcomes: RefCell<BTreeMap<OperationId, Result<OperationOutcome, ExecutionError>>>,
    }

    impl ExecutionPort for FakePort {
        fn revalidate(&self, _plan: &Plan) -> Result<(), ExecutionError> {
            *self.revalidated.borrow_mut() = true;
            if self.fail_revalidation {
                return Err(ExecutionError::revalidation(
                    EvidenceCode::new("revalidation.changed").unwrap(),
                    EvidenceDetail::new("affected evidence changed").unwrap(),
                ));
            }
            Ok(())
        }

        fn apply(&self, operation: &Operation) -> Result<OperationOutcome, ExecutionError> {
            assert!(*self.revalidated.borrow());
            self.calls.borrow_mut().push(operation.id().clone());
            self.outcomes
                .borrow_mut()
                .remove(operation.id())
                .unwrap_or(Ok(OperationOutcome::Applied))
        }
    }

    #[derive(Default)]
    struct FakeJournal {
        records: RefCell<Vec<OperationResult>>,
        fail_after: Option<usize>,
    }

    impl ExecutionJournal for FakeJournal {
        fn record(&self, result: &OperationResult) -> Result<(), ExecutionError> {
            if self
                .fail_after
                .is_some_and(|limit| self.records.borrow().len() >= limit)
            {
                return Err(ExecutionError::Journal {
                    operation: result.operation_id().clone(),
                    after_apply: false,
                    source: Box::new(ExecutionError::InvalidOutcome {
                        operation: result.operation_id().clone(),
                    }),
                });
            }
            self.records.borrow_mut().push(result.clone());
            Ok(())
        }
    }

    fn path() -> crate::domain::AbsolutePath {
        crate::domain::AbsolutePath::new("/tmp/skilltap-executor.lock").unwrap()
    }

    fn operation(id: &str, class: OperationClass, dependencies: &[&str]) -> Operation {
        let target = HarnessId::new("codex").unwrap();
        let resource = ResourceKey::new(ResourceId::new("plugin:demo").unwrap(), Scope::Global);
        let compatibility = CompatibilityResult::new(
            target.clone(),
            CompatibilityClass::Compatible,
            match class {
                OperationClass::SafeNative | OperationClass::NoOp => TransferFidelity::Faithful,
                _ => TransferFidelity::Blocked,
            },
            [],
            [],
        )
        .unwrap();
        Operation::new(
            OperationId::new(id).unwrap(),
            target,
            OperationSelector::Resource {
                resource: resource.clone(),
            },
            OperationSemantics::new(
                OperationAction::PluginInstall,
                Scope::Global,
                OperationReason::new(
                    EvidenceCode::new("test.operation").unwrap(),
                    EvidenceDetail::new("test operation").unwrap(),
                ),
                compatibility,
                Provenance::Native,
                [AffectedSurface::file(
                    crate::domain::AbsolutePath::new("/tmp/skilltap-resource").unwrap(),
                )],
            ),
            class,
            if matches!(class, OperationClass::NoOp) {
                Reversibility::NotApplicable
            } else {
                Reversibility::Reversible
            },
            dependencies
                .iter()
                .map(|id| crate::domain::OperationDependency::new(OperationId::new(*id).unwrap())),
            crate::domain::AcknowledgmentRequirement::not_required(),
            None,
        )
        .unwrap()
    }

    fn plan(operations: impl IntoIterator<Item = Operation>) -> Plan {
        Plan::new(operations).unwrap()
    }

    #[test]
    fn executes_in_waves_and_journals_pending_then_terminal() {
        let lock = FakeLock {
            acquired: RefCell::new(false),
            contended: false,
        };
        let port = FakePort::default();
        let journal = FakeJournal::default();
        let plan = plan([
            operation("dependent", OperationClass::SafeNative, &["base"]),
            operation("base", OperationClass::SafeNative, &[]),
        ]);
        let report = execute_plan(&lock, &path(), &port, &journal, &plan).unwrap();
        assert_eq!(report.result.outcome(), ApplyOutcome::Succeeded);
        assert!(report.changed);
        assert_eq!(
            port.calls.borrow().as_slice(),
            &[
                OperationId::new("base").unwrap(),
                OperationId::new("dependent").unwrap()
            ]
        );
        assert_eq!(journal.records.borrow().len(), 4);
        assert!(matches!(
            journal.records.borrow()[0].outcome(),
            OperationOutcome::Pending
        ));
    }

    #[test]
    fn failed_operation_skips_dependents_but_keeps_independent_success() {
        let lock = FakeLock {
            acquired: RefCell::new(false),
            contended: false,
        };
        let port = FakePort::default();
        port.outcomes.borrow_mut().insert(
            OperationId::new("failed").unwrap(),
            Ok(OperationOutcome::Failed {
                reason: AttentionReason::operation_failed(
                    EvidenceCode::new("native.failed").unwrap(),
                    EvidenceDetail::new("native action failed").unwrap(),
                ),
            }),
        );
        let journal = FakeJournal::default();
        let plan = plan([
            operation("failed", OperationClass::SafeNative, &[]),
            operation("dependent", OperationClass::SafeNative, &["failed"]),
            operation("independent", OperationClass::SafeNative, &[]),
        ]);
        let report = execute_plan(&lock, &path(), &port, &journal, &plan).unwrap();
        assert_eq!(report.result.outcome(), ApplyOutcome::PartialFailure);
        assert!(report.changed);
        assert!(
            !port
                .calls
                .borrow()
                .contains(&OperationId::new("dependent").unwrap())
        );
        assert!(matches!(
            report
                .result
                .operations()
                .get(&OperationId::new("dependent").unwrap())
                .unwrap()
                .outcome(),
            OperationOutcome::SkippedDependency { .. }
        ));
    }

    #[test]
    fn lock_contention_prevents_revalidation_and_mutation() {
        let lock = FakeLock {
            acquired: RefCell::new(false),
            contended: true,
        };
        let port = FakePort::default();
        let journal = FakeJournal::default();
        let plan = plan([operation("one", OperationClass::SafeNative, &[])]);
        assert!(matches!(
            execute_plan(&lock, &path(), &port, &journal, &plan),
            Err(ExecutionError::Lock(RuntimeError::LockContended { .. }))
        ));
        assert!(!*port.revalidated.borrow());
        assert!(port.calls.borrow().is_empty());
        assert!(journal.records.borrow().is_empty());
    }

    #[test]
    fn revalidation_failure_aborts_before_any_journal_or_native_action() {
        let lock = FakeLock {
            acquired: RefCell::new(false),
            contended: false,
        };
        let port = FakePort {
            fail_revalidation: true,
            ..FakePort::default()
        };
        let journal = FakeJournal::default();
        let plan = plan([operation("one", OperationClass::SafeNative, &[])]);
        assert!(matches!(
            execute_plan(&lock, &path(), &port, &journal, &plan),
            Err(ExecutionError::Revalidation { .. })
        ));
        assert!(port.calls.borrow().is_empty());
        assert!(journal.records.borrow().is_empty());
    }

    #[test]
    fn journal_failure_after_action_is_explicitly_partial() {
        let lock = FakeLock {
            acquired: RefCell::new(false),
            contended: false,
        };
        let port = FakePort::default();
        // The first call publishes Pending; the terminal publication fails
        // after the adapter has already returned Applied.
        let journal = FakeJournal {
            fail_after: Some(1),
            ..FakeJournal::default()
        };
        let plan = plan([operation("one", OperationClass::SafeNative, &[])]);
        let error = execute_plan(&lock, &path(), &port, &journal, &plan).unwrap_err();
        assert!(matches!(
            error,
            ExecutionError::Journal {
                after_apply: true,
                operation,
                ..
            } if operation == OperationId::new("one").unwrap()
        ));
        assert_eq!(
            port.calls.borrow().as_slice(),
            &[OperationId::new("one").unwrap()]
        );
    }

    #[test]
    fn repeated_no_op_execution_does_not_call_the_adapter() {
        let lock = FakeLock {
            acquired: RefCell::new(false),
            contended: false,
        };
        let port = FakePort::default();
        let journal = FakeJournal::default();
        let plan = plan([operation("noop", OperationClass::NoOp, &[])]);
        let first = execute_plan(&lock, &path(), &port, &journal, &plan).unwrap();
        let second = execute_plan(&lock, &path(), &port, &journal, &plan).unwrap();
        assert!(!first.changed);
        assert!(!second.changed);
        assert_eq!(first.result, second.result);
        assert!(port.calls.borrow().is_empty());
        assert_eq!(journal.records.borrow().len(), 2);
    }

    #[test]
    fn attention_operations_are_blocked_without_native_calls() {
        let lock = FakeLock {
            acquired: RefCell::new(false),
            contended: false,
        };
        let port = FakePort::default();
        let journal = FakeJournal::default();
        let target = HarnessId::new("codex").unwrap();
        let resource = ResourceKey::new(ResourceId::new("plugin:demo").unwrap(), Scope::Global);
        let consequence = MaterialConsequence::new(
            ConsequenceCode::new("native.unsupported").unwrap(),
            [],
            ConsequenceSummary::new("The native operation is unavailable").unwrap(),
        );
        let operation = Operation::new(
            OperationId::new("unsupported").unwrap(),
            target.clone(),
            OperationSelector::Resource {
                resource: resource.clone(),
            },
            OperationSemantics::new(
                OperationAction::PluginInstall,
                Scope::Global,
                OperationReason::new(
                    EvidenceCode::new("test.unsupported").unwrap(),
                    EvidenceDetail::new("no native operation").unwrap(),
                ),
                CompatibilityResult::new(
                    target.clone(),
                    CompatibilityClass::TargetSpecific,
                    TransferFidelity::Blocked,
                    [CompatibilityEvidence::new(
                        EvidenceCode::new("native.unsupported").unwrap(),
                        target,
                        [],
                        EvidenceDetail::new("native operation is unavailable").unwrap(),
                    )],
                    [consequence],
                )
                .unwrap(),
                Provenance::Native,
                [],
            ),
            OperationClass::Unsupported,
            Reversibility::NotApplicable,
            [],
            crate::domain::AcknowledgmentRequirement::not_required(),
            Some(AttentionReason::unsupported(
                EvidenceCode::new("native.unsupported").unwrap(),
                EvidenceDetail::new("native operation is unavailable").unwrap(),
            )),
        )
        .unwrap();
        let plan = plan([operation]);
        let report = execute_plan(&lock, &path(), &port, &journal, &plan).unwrap();
        assert_eq!(report.result.outcome(), ApplyOutcome::AttentionRequired);
        assert!(!report.changed);
        assert!(port.calls.borrow().is_empty());
        assert!(matches!(
            report
                .result
                .operations()
                .values()
                .next()
                .unwrap()
                .outcome(),
            OperationOutcome::Blocked { .. }
        ));
    }

    #[test]
    fn exact_foreground_acknowledgment_allows_the_same_partial_operation() {
        let lock = FakeLock {
            acquired: RefCell::new(false),
            contended: false,
        };
        let port = FakePort::default();
        let journal = FakeJournal::default();
        let target = HarnessId::new("codex").unwrap();
        let resource = ResourceKey::new(ResourceId::new("plugin:demo").unwrap(), Scope::Global);
        let component = crate::domain::ComponentId::new("mcp:demo").unwrap();
        let operation = crate::lifecycle_operation::managed_partial_materialization_operation(
            OperationId::new("partial").unwrap(),
            target.clone(),
            resource,
            OperationAction::PluginInstall,
            [crate::domain::AbsolutePath::new("/tmp/skilltap-partial").unwrap()],
            [CompatibilityEvidence::new(
                EvidenceCode::new("managed.effective_unverified").unwrap(),
                target,
                [component.clone()],
                EvidenceDetail::new("The declaration cannot prove effective loading.").unwrap(),
            )],
            [MaterialConsequence::new(
                ConsequenceCode::new("managed.effective_unverified").unwrap(),
                [component],
                ConsequenceSummary::new("Effective loading remains unverified.").unwrap(),
            )],
        )
        .unwrap();
        let plan = plan([operation]);

        let blocked = execute_plan(&lock, &path(), &port, &journal, &plan).unwrap();
        assert_eq!(blocked.result.outcome(), ApplyOutcome::AttentionRequired);
        assert!(!blocked.changed);
        assert!(port.calls.borrow().is_empty());

        let acknowledgments = ExecutionAcknowledgments::foreground_all(&plan);
        assert!(acknowledgments.accepts(plan.get(&OperationId::new("partial").unwrap()).unwrap()));
        let accepted = execute_plan_with_acknowledgments(
            &lock,
            &path(),
            &port,
            &journal,
            &plan,
            &acknowledgments,
        )
        .unwrap();
        assert_eq!(accepted.result.outcome(), ApplyOutcome::Succeeded);
        assert!(accepted.changed);
        assert_eq!(
            port.calls.borrow().as_slice(),
            &[OperationId::new("partial").unwrap()]
        );
    }
}
