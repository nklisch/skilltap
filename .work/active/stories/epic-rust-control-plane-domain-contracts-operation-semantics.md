---
id: epic-rust-control-plane-domain-contracts-operation-semantics
kind: story
stage: review
tags: []
parent: epic-rust-control-plane-domain-contracts
depends_on: [epic-rust-control-plane-domain-contracts-plan-results]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Complete Semantic Operations and Execution Results

## Scope

Correct the plan/result contracts found incomplete during feature review. Every
operation must state the semantic action, concrete scope, ordinary reason,
target-bound compatibility, provenance, and declarative affected file or native
command surfaces required for adapter resolution and human/JSON rendering,
without introducing adapter-internal actions or execution algorithms.

## Acceptance criteria

- [x] `Operation` exposes validated, strict-serde semantic action, concrete
  `Scope`, reason code/detail, `CompatibilityResult`, `Provenance`, and a
  deterministic affected-surface set in addition to its existing fields.
- [x] Native-command previews are target-bound and redactable; affected paths
  use `AbsolutePath`. Compatibility target must equal operation target.
- [x] Every material consequence is covered by the exact acknowledged resource
  or component selectors; cross-resource/cross-component consent fails through
  constructors and serde.
- [x] Apply validation forbids `Applied` for unsupported/conflict and requires
  `NoChange` for no-op operations.
- [x] A planned operation cannot report applied/no-change when any declared
  dependency failed, blocked, skipped, or remains pending; skips enumerate the
  actual blocking dependencies.
- [x] Operation-cycle errors report actual cycle members rather than downstream
  nodes blocked by a cycle.
- [x] All new fields and payloads have read-only accessors and deterministic JSON
  round trips; no planner, executor, adapter, storage, or renderer algorithm is
  added.
- [x] Locked format, clippy, and workspace tests pass.

## Implementation notes

- Files changed: `crates/core/src/domain/operation.rs`.
- Added contracts: `OperationAction`, `OperationReason`, `OperationSemantics`,
  `AffectedSurface`, and redactable `CommandArgument` previews. Operations now
  carry concrete scope, target-bound compatibility, provenance, and
  deterministic affected surfaces through strict wires and read-only accessors.
- Validation added: compatibility and command target binding; exact
  acknowledgment coverage for resource/component consequences; operation-class
  result matrices; exact dependency-blocker skip sets; exact cycle members.
- Tests added: constructor and serde negatives for semantic targets, redacted
  argument payloads, consequence coverage, class/outcome mismatches, dependency
  result propagation, and downstream cycle exclusion; representative semantic
  operations and apply results round-trip deterministically.
- Discrepancies from design: none.
- Adjacent issues parked: none.

## Review findings (2026-07-11)

- Blocker: executable safe/partial operations may carry an empty affected-surface
  set. Require at least one affected file or target-bound native command through
  constructor and serde paths.
- Blocker: operation class is not checked against transfer fidelity, and partial
  acknowledgment consequences may differ from the authoritative compatibility
  consequences. Enforce the class/fidelity matrix and exact consequence equality
  for partial operations. `NoOp` may represent already-satisfied faithful,
  materializable, or previously accepted partial state, but never blocked state.

## Review corrections (2026-07-11)

- Executable safe and partial operations now require at least one deterministic
  affected surface through both construction and deserialization.
- Every operation class now enforces its complete transfer-fidelity matrix;
  no-op accepts faithful, materializable, and partial state but rejects blocked.
- Partial acknowledgment and attention consequences must exactly equal the
  authoritative compatibility consequences, preventing both invented and
  omitted consent disclosures.
- Added exhaustive constructor and strict-wire coverage for all 28
  class/fidelity combinations, empty safe/partial surfaces, and both directions
  of partial consequence mismatch.
