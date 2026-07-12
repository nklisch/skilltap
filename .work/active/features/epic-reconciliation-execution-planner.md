---
id: epic-reconciliation-execution-planner
kind: feature
stage: done
tags: []
parent: epic-reconciliation-execution
depends_on: []
release_binding: 3.0.0
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Build Pure Reconciliation Planner

Classify desired inventory against one fresh normalized environment and
last-applied state into the existing validated operation model. Cover ownership,
drift, conflicts, unsupported/partial outcomes, safe native outcomes, and
idempotent no-op classification without external I/O.

## Architectural choice

Keep the planner pure and adapter-neutral. Harness/resource adapters provide
validated operation candidates and observation evidence; the planner decides
whether each candidate is safe, blocked, drifted, conflicting, or a no-op and
assembles the existing `Plan` contract. This preserves native behavior at the
adapter boundary and avoids a second operation wire model. A planner that
directly shells out to harnesses would make status/plan/sync disagree and would
be impossible to test without mutation-capable fixtures.

## Design decisions

- **Freshness**: only the supplied normalized snapshot and state document are
  inputs; the planner never reads repositories, files, or native commands.
- **Ownership**: desired provenance/ownership and observed ownership are
  compared before proposing mutation. Unmanaged or user-owned drift becomes a
  conflict, never an implicit overwrite.
- **No-op**: an observed resource matching desired semantics and the last
  applied evidence emits a validated `OperationClass::NoOp` candidate or no
  operation, deterministically; it never emits a safe mutation.
- **Unknowns**: unknown compatibility, missing evidence, unresolved
  dependencies, and unverified harness capability remain explicit blocked or
  attention classifications.
- **Selectors**: selector filtering and dependency expansion belong to the
  following graph feature; this planner receives the already selected
  candidate set and preserves exact scope-bearing keys.

## Implementation Units

### Unit 1: Adapter-neutral assessment contract

**File**: `crates/core/src/reconciliation.rs` (new, exported from
`crates/core/src/lib.rs`)

```rust
pub struct ReconciliationCandidate {
    pub operation: Operation,
    pub resource: ResourceKey,
    pub expected_identity: Option<NativeId>,
    pub expected_fingerprint: Option<Fingerprint>,
    pub observed: Option<ObservedResource>,
    pub prior_state: Option<ResourceState>,
}

pub struct ReconciliationRequest {
    pub candidates: Vec<ReconciliationCandidate>,
}

pub enum ReconciliationFinding {
    Drift { resource: ResourceKey },
    OwnershipConflict { resource: ResourceKey },
    MissingEvidence { resource: ResourceKey },
}

pub struct ReconciliationPlan {
    pub plan: Plan,
    pub findings: Vec<ReconciliationFinding>,
}

pub fn plan_reconciliation(
    request: ReconciliationRequest,
) -> Result<ReconciliationPlan, ReconciliationError>;
```

Validate that candidate resource/selector/scope identity agrees with the
operation, that expected evidence matches the operation's affected target, and
that duplicate exact operation/resource identities are rejected. Findings are
sorted by scope-bearing key and never contain raw native payloads.

### Unit 2: Ownership, drift, and no-op classification

**File**: `crates/core/src/reconciliation.rs`

```rust
pub fn classify_candidate(
    candidate: &ReconciliationCandidate,
) -> Result<ReconciliationDisposition, ReconciliationError>;

pub enum ReconciliationDisposition {
    Keep(Operation),
    NoOp(Operation),
    Attention { operation: Operation, finding: ReconciliationFinding },
}
```

Use exact native identity and fingerprint evidence when present. A mismatched
prior fingerprint, user-owned unmanaged observation, or drifted managed
artifact creates an attention/conflict disposition instead of silently
rewriting. A semantically equal healthy observation preserves the candidate's
validated no-op/safe classification and remains idempotent.

### Unit 3: Planner contract tests

**Files**: `crates/core/src/reconciliation_tests.rs` (or module tests) and
`crates/core/tests/foundation_integration.rs`

Cover safe native, faithful, materialized, partial, unsupported, conflict,
no-op, ownership drift, duplicate identity, exact scope mismatch, missing
evidence, deterministic ordering, and secret-safe findings. Every test builds
validated `Operation` values through constructors and asserts no external I/O.

## Implementation Order

1. Add the candidate/disposition contract and error taxonomy.
2. Implement pure ownership, drift, evidence, and no-op classification.
3. Assemble validated plans and deterministic findings.
4. Add unit and foundation integration coverage, then run workspace checks.

## Testing

The planner tests use in-memory candidates and existing operation/resource
fixtures. They must assert that equal inputs produce equal plans, that all
findings are scope-bearing and redactable, and that a repeated planning pass
does not mutate inventory, state, or native fixtures.

## Risks

- The existing operation constructors reject incomplete semantics by design;
  planner tests must construct complete evidence rather than weakening those
  invariants.
- Native resource-specific adapters are not yet implemented. Candidate fixtures
  must remain explicit and must not claim marketplace/plugin/skill lifecycle
  support before their later epics land.

## Implementation notes

- Added `skilltap_core::reconciliation` with adapter-neutral candidate,
  disposition, finding, and plan types.
- Classification validates exact selector/scope identity, detects native
  identity/fingerprint drift, unmanaged ownership conflicts, degraded/unknown
  health, missing evidence, and preserves validated no-op operations.
- Plan assembly is deterministic, duplicate-resource rejecting, and delegates
  operation graph validation to the existing `Plan` contract; no external I/O
  is performed.

## Verification

- `cargo fmt --all`
- `cargo test -p skilltap-core reconciliation --offline`
- `cargo clippy -p skilltap-core --all-targets --offline -- -D warnings`

## Review

Verdict: Approve with comments - pure planner contract is implemented and
verified; richer candidate fixtures and graph/executor integration remain in
the dependent features.

## Review

### Summary

The planner is pure and adapter-neutral, validates exact selector/scope
identity, detects identity/fingerprint and ownership drift, preserves no-op
classification, and delegates dependency graph validation to the existing
operation contract.

### Verdict

Approve with comments.

### Findings

- Important follow-up: resource-specific adapters still need to provide rich
  validated candidates; this module deliberately does not invent native
  lifecycle commands.
- Important follow-up: the graph and executor features must treat planner
  findings as apply blockers so an attention disposition cannot be executed as
  a safe operation accidentally.

### Notes

Inline deep review completed because no different model-class reviewer was
available. Four focused tests and strict clippy pass; downstream features own
the remaining integration behavior.
