---
id: epic-cross-harness-materialization-compatibility-aggregate
kind: story
stage: done
tags: []
parent: epic-cross-harness-materialization-compatibility
depends_on: [epic-cross-harness-materialization-compatibility-policy]
release_binding: 3.0.0
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Aggregate Dependency-Aware Compatibility

Add dependency propagation, aggregate resource classification, and exact
component acknowledgment selectors in `crates/core/src/compatibility.rs`.

Acceptance criteria:

- Required dependency loss blocks affected dependents; optional loss remains
  visible as partial.
- Aggregate evidence/consequences and selector sets are deterministic.
- A faithful aggregate is impossible when any material consequence exists.

## Implementation notes

- Files changed: `crates/core/src/compatibility.rs` (implemented with the
  policy unit because dependency propagation and aggregate construction share
  the same validated decisions).
- Tests added: dependency loss propagation, deterministic aggregate fidelity,
  and exact partial acknowledgment selector assertions.
- Discrepancies from design: no separate module split was introduced; the
  cohesive analyzer keeps component decisions and aggregation in one pure core
  boundary.
- Adjacent issues parked: none.

## Verification

- `cargo test -p skilltap-core compatibility::tests --offline` — passed.
- `cargo clippy -p skilltap-core --all-targets --offline -- -D warnings` —
  passed.

## Review

Verdict: Approve — story verified by implement; fast-lane advance.
