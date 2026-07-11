---
id: epic-reconciliation-execution
kind: epic
stage: drafting
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

## Anticipated child features

- Pure desired/observed/last-applied planner
- Operation graph, selectors, and acknowledgment rules
- Ownership, drift, conflict, and no-op classification
- Locked executor with fingerprint revalidation and result journal
- Partial-failure recovery and idempotency enforcement
- `plan` and `sync` command surfaces with stable output

<!-- The design pass on each child feature will fill in real specifics. -->
