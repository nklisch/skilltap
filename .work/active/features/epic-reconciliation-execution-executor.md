---
id: epic-reconciliation-execution-executor
kind: feature
stage: review
tags: []
parent: epic-reconciliation-execution
depends_on: [epic-reconciliation-execution-graph]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Safely Execute Reconciliation Plans

Apply safe operations through generic native/filesystem ports under the
configuration lock. Revalidate affected identities and fingerprints, journal
planned/running/completed/failed results atomically in state, stop dependent
work after failure, preserve independent successes, and return a fresh recovery
plan after partial execution.

## Architectural choice

Implement one synchronous core executor over two explicit ports: an operation
port for revalidation/native or filesystem application, and a journal port for
atomic state publication at each operation boundary. The executor owns lock
acquisition, dependency waves, blocked/failed result construction, and final
outcome classification. Concrete harness adapters and file repositories are
composed later by the CLI and lifecycle epics.

## Design decisions

- Lock acquisition is fail-fast and happens before revalidation or any apply.
- Revalidation failure aborts before the first operation; an apply failure is
  journaled, independent operations may continue, and dependents are skipped.
- Journal failure after a native action returns an explicit partial-failure
  error; the executor never claims the state is synchronized.
- Unsupported, conflict, and partial operations are blocked unless the
  operation already carries an exact accepted acknowledgment; no generic
  bypass is added here.
- A final `ApplyResult` is validated through existing operation contracts and
  contains one result for every operation in the plan.

## Implementation Units

### Unit 1: Execution ports and errors

**File**: `crates/core/src/executor.rs` (new, exported from
`crates/core/src/lib.rs`)

```rust
pub trait ExecutionPort {
    fn revalidate(&self, plan: &Plan) -> Result<(), ExecutionError>;
    fn apply(&self, operation: &Operation) -> Result<OperationOutcome, ExecutionError>;
}

pub trait ExecutionJournal {
    fn record(&self, result: &OperationResult) -> Result<(), ExecutionError>;
}

pub fn execute_plan<L, P, J>(
    lock: &L,
    lock_path: &AbsolutePath,
    port: &P,
    journal: &J,
    plan: &Plan,
) -> Result<ExecutionReport, ExecutionError>;
```

The traits are synchronous, object-safe, and core-only; they never write
terminal output or expose raw native payloads.

### Unit 2: Dependency-ordered apply and journaling

**File**: `crates/core/src/executor.rs`

```rust
pub struct ExecutionReport {
    pub result: ApplyResult,
    pub changed: bool,
}
```

Walk stable dependency waves, produce `NoChange` for no-op operations, block
attention operations, journal each result immediately, skip dependents with
typed dependency blockers, and continue independent operations. Revalidation
and lock release cover all success and error paths.

### Unit 3: Executor contract tests

**Files**: `crates/core/src/executor.rs` tests and
`crates/core/tests/storage_integration.rs`

Use in-memory lock/port/journal fakes to cover lock contention, revalidation
failure, successful and repeated no-op execution, independent failure with
dependent skip, journal failure, blocked partial/conflict operations, stable
result ordering, and native-operation no-mutation boundaries.

## Implementation Order

1. Define ports, typed errors, and report shape.
2. Implement lock/revalidation and dependency-wave execution.
3. Add per-result journaling, failure/skip classification, and contract tests.

## Testing

Every mutating fake records calls and journal writes so tests can assert exact
ordering and that no operation runs before revalidation. Re-running an already
successful no-op plan must produce no additional apply calls.

## Risks

- Journaling after a native success can fail; preserve the explicit partial
  failure result and leave recovery to fresh observation rather than guessing.
- The executor must not reinterpret compatibility or selectors; those are
  validated by planner/graph contracts and carried through unchanged.

## Implementation notes

- Files changed: `crates/core/src/executor.rs`, `crates/core/src/lib.rs`.
- Added synchronous `ExecutionPort` and `ExecutionJournal` ports plus
  `execute_plan`, `ExecutionReport`, and typed lock/revalidation/apply/journal
  errors.
- Execution acquires the cooperative configuration lock before revalidation,
  records `Pending` before executable actions, journals terminal outcomes at
  each boundary, and marks post-action journal failures explicitly for fresh
  recovery observation.
- Stable dependency waves continue independent operations, skip dependents of
  failed/blocked/skipped results, and classify the complete result through the
  existing `ApplyResult` contract. No resource-specific native mutation was
  added.
- Partial, unsupported, and conflict classes remain blocked in this core entry
  point; the existing operation contract has no separate accepted-acknowledgment
  bit, so a later CLI composition layer must pass only an explicitly accepted
  plan into execution.
- Tests added: lock contention, revalidation-before-mutation, stable waves,
  pending/terminal journal ordering, independent failure with dependent skip,
  attention blocking, post-action journal failure, and repeated no-op execution.
- Discrepancies from design: no concrete native adapter or state repository was
  added; those remain composition work for later lifecycle and CLI features.
- Adjacent issues parked: none.

## Verification

- `cargo fmt --all`
- `cargo test -p skilltap-core executor --offline`
- `cargo clippy -p skilltap-core --all-targets --offline -- -D warnings`
