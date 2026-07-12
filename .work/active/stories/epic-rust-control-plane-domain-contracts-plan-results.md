---
id: epic-rust-control-plane-domain-contracts-plan-results
kind: story
stage: done
tags: []
parent: epic-rust-control-plane-domain-contracts
depends_on: [epic-rust-control-plane-domain-contracts-resource-graph, epic-rust-control-plane-domain-contracts-capability-compatibility]
release_binding: 3.0.0
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Define Plans, Operation Dependencies, and Results

## Scope

Implement Unit 4 from the parent feature: serializable operation graphs,
classifications, reversibility, exact piecewise acknowledgment requirements,
attention reasons, and validated per-operation and final apply outcomes.

## Acceptance criteria

- [x] Plan construction rejects duplicate ids, unknown dependencies, and cycles.
- [x] Partial operations enumerate exact selectors and non-empty consequences;
  no generic confirmation value erases compatibility evidence.
- [x] Apply results cannot claim success when an operation failed or remains
  blocked, and dependent skips remain distinguishable from failures.
- [x] Representative plans and partial results serialize deterministically and
  round-trip through JSON.
- [x] No planner, executor, adapter, persistence, or CLI rendering behavior is
  introduced.
- [x] Locked format, clippy, and workspace tests pass.

## Implementation notes

- Files changed: `crates/core/src/domain/operation.rs`,
  `crates/core/src/domain/mod.rs`.
- Tests added: constructor and deserialization rejection for duplicate,
  dangling, self, and cyclic plan edges; strict acknowledgment, attention,
  classification, reversibility, and result-summary combinations; stable
  snake_case JSON and deterministic round trips; failed, blocked, pending, and
  dependency-skipped outcomes.
- Dispatch: direct implementation in the assigned isolated domain lane; the
  integration surface was limited to completed domain APIs and stable module
  reexports.
- Discrepancies from design: none.
- Adjacent issues parked: none.

## Review findings (2026-07-11)

- Blocker: public `Operation` and `AttentionReason` values hide payloads needed
  by executors/renderers. Add read-only accessors for all operation fields and
  typed access to every attention payload.
- Blocker: `ApplyResult` is not bound to a `Plan`, so it can omit failed planned
  work or add unknown ids and still claim success. Make it own the exact plan,
  require one result per planned operation with no extras, persist that plan in
  its strict wire form, and expose it read-only.
- Blocker: dependency-skipped results must name declared dependencies of that
  exact plan operation, not merely any failed result.
- Blocker: partial acknowledgment selectors must remain within the operation's
  resource/component selector scope.
- Important: `Unsupported`, `Conflict`, and `NoOp` are non-actionable and must
  use `NotApplicable`; executable safe/partial classes must not.
- Important: failed outcomes require `OperationFailed` attention, while blocked
  outcomes accept only non-failure blocking reasons. Enforce at constructor and
  serde boundaries.

## Review (2026-07-11)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Initial inspectability, scope, reversibility, outcome, and plan/result
binding findings were resolved in `0f7c988`. Story verified by implement,
fresh-context adversarial contract review, and integrated locked workspace
checks; fast-lane advance.

## Review corrections (2026-07-11)

- Added read-only accessors for every `Operation` field and typed optional
  accessors for each `AttentionReason` payload.
- Bound `ApplyResult` to its complete serialized `Plan`; construction and
  deserialization now require exactly one result for every planned operation
  and reject extra result ids.
- Restricted dependency skips to dependencies declared by that exact operation
  and required each named dependency to have a failed, blocked, or skipped
  result.
- Restricted acknowledgment selectors to the operation's resource or exact
  component scope.
- Enforced `NotApplicable` only for unsupported, conflict, and no-op classes,
  and enforced the permitted attention kinds for failed and blocked results.
- Added constructor and serde regressions for all corrected boundaries. Locked
  format, check, clippy with warnings denied, and all workspace tests pass.
