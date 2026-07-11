---
id: epic-reconciliation-execution
kind: epic
stage: done
tags: []
parent: null
depends_on: [epic-harness-observation-adoption]
release_binding: null
gate_origin: null
created: 2026-07-10
updated: 2026-07-10
---

# Reconciliation Execution

## Brief

Deliver the engine that turns desired inventory and fresh observations into an
explainable dependency-ordered plan, then safely applies the operations that do
not require unresolved judgment. This includes ownership and drift analysis,
selectors, compatibility outcomes, operation dependencies, and stable human and
JSON representations for `plan` and `sync`.

Mutation must be serialized, revalidated against current fingerprints, journaled
as it proceeds, and recoverable through re-observation after partial failure.
This epic establishes generic reconciliation and execution; resource-specific
marketplace, plugin, skill, and instruction operations arrive in later epics.

## Foundation references

- `docs/VISION.md` — Plan Before Mutation, Explicit Loss, Idempotent Reconciliation
- `docs/SPEC.md` — Planning, Synchronization, Ownership and Removal, Mutation Safety
- `docs/ARCH.md` — Planning, Apply Flow, Concurrency, Error Model
- `docs/UX.md` — Planning and Synchronization, JSON Output, Errors

## Design decisions

- **What happens when another process holds the mutation lock?** Fail fast
  with an attention result, available lock-owner context, and an actionable
  retry instruction. The daemon skips a contended cycle and records the
  contention instead of waiting.
- **How is partial execution made crash-recoverable?** Atomically update
  `state.json` as each operation moves through planned, running, completed, or
  failed state. On interruption, re-observe native state and compute a fresh
  recovery plan; do not add a separate append-only journal file.
- **Does this epic require UI mockups?** No. Plans and apply results are
  non-interactive plain-text and JSON CLI surfaces.

## Decomposition

The epic is split by capability rather than crate layer. The planner owns the
pure desired/observed/state decision model; the graph feature owns dependency
and selector invariants; the executor owns lock/revalidation/journal/recovery;
and the CLI feature composes those ports into `plan` and `sync`. This keeps the
pure core independently testable while allowing later lifecycle epics to add
resource-specific adapters without changing execution safety.

### Child features

- `epic-reconciliation-execution-planner` — classify desired resources against
  fresh observations and last-applied state into explainable safe, partial,
  unsupported, conflict, drift, and no-op operations — depends on: `[]`.
- `epic-reconciliation-execution-graph` — enforce operation dependencies,
  exact scope-bearing selectors, piecewise include/exclude selection, and
  acknowledgment semantics — depends on:
  `[epic-reconciliation-execution-planner]`.
- `epic-reconciliation-execution-executor` — safely apply dependency-ordered
  operations under the process lock, revalidate fingerprints, journal state,
  and recover after partial failure — depends on:
  `[epic-reconciliation-execution-graph]`.
- `epic-reconciliation-execution-cli` — expose deterministic `plan` and `sync`
  commands with shared plain/JSON output, exit classes, and idempotency
  coverage — depends on: `[epic-reconciliation-execution-executor]`.

### Design decisions

- **Plan representation**: use the existing validated `Plan`, `Operation`,
  selector, compatibility, and attention contracts as the single wire model;
  planner helpers return those types rather than introducing a parallel
  application-only plan shape.
- **Execution policy**: safe independent operations may proceed when another
  operation is blocked, but dependent operations are skipped with explicit
  dependency blockers. No operation is applied after its affected observation
  or executable identity fails revalidation.
- **State journaling**: update `state.json` atomically at operation boundaries
  through the existing repository; a fresh observation and plan is the recovery
  mechanism, not a second journal format.
- **Acknowledgment**: only exact partial consequences shown by the plan may be
  acknowledged; there is no generic bypass. Include/exclude selectors narrow
  operations by exact scope-bearing resource or component identity.
- **Native boundary**: this epic supplies generic operation ports and safety;
  marketplace, plugin, standalone skill, instruction, and materialization
  adapters remain in their later epics.

### Decomposition risks

- The current operation contracts are extensive; planner helpers must preserve
  their constructor invariants instead of bypassing them with ad hoc structs.
- State journaling after each operation can expose a publication failure after
  native success; recovery must report uncertainty and never claim completion.
- Resource-specific adapters are not yet present, so initial integration tests
  use deterministic fake operation ports and must not imply native lifecycle
  support prematurely.

## UI alignment deferred

This epic has no visual surface. `plan` and `sync` are non-interactive CLI
commands rendered through the existing plain/JSON output contract.

## Implementation summary

The reconciliation execution foundation is complete. Core now provides a pure
planner for drift, ownership, and evidence findings; deterministic operation
graph traversal with exact selectors and acknowledgment validation; and a
locked executor with revalidation, pending/terminal journaling, independent
failure continuation, and dependency skips. The CLI exposes deterministic
`plan` and `sync` command envelopes with scope/target resolution and stable
plain/JSON exit classes. Resource-specific lifecycle adapters intentionally
remain in the dependent marketplace, plugin, skill, instruction, and
materialization epics.

## Completion review

### Verdict

Approve with comments.

### Findings

- The generic executor is fully tested, but the current CLI composition uses an
  empty candidate bridge and remains read-only for populated inventory until
  resource lifecycle adapters arrive.
- Exact consequence acknowledgments still need to replace the legacy `--yes`
  spelling before any partial operation can be applied.

### Verification

Full workspace tests, compiled-binary contracts, formatting, and strict clippy
pass.
