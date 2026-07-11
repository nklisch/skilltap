---
id: epic-cross-harness-materialization-compatibility-aggregate
kind: story
stage: implementing
tags: []
parent: epic-cross-harness-materialization-compatibility
depends_on: [epic-cross-harness-materialization-compatibility-policy]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Aggregate Dependency-Aware Compatibility

Add dependency propagation, aggregate resource classification, and exact
component acknowledgment selectors in `crates/core/src/compatibility.rs`.

Acceptance criteria:

- Required dependency loss blocks affected dependents; optional loss remains
  visible as partial.
- Aggregate evidence/consequences and selector sets are deterministic.
- A faithful aggregate is impossible when any material consequence exists.
