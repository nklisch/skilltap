---
id: feature-daemon-marketplace-refresh-task-graph
kind: story
stage: done
tags: [infra]
parent: feature-daemon-marketplace-refresh
depends_on: []
release_binding: 3.1.0
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-14
updated: 2026-07-15
---

# Build the Daemon Native-Update Task Graph

## Checkpoint

Introduce the pure, deterministic daemon task graph described by Unit 1 of the
parent feature. Build one marketplace-refresh task per exact marketplace
`ResourceKey` and target, attach tracked plugin tasks to that exact prerequisite,
and retain malformed or unavailable relationships as typed per-plugin blockers
without invalidating unrelated branches.

The graph must derive from desired inventory only, preserve scope and target in
identity, deduplicate shared marketplace prerequisites deterministically, and
attach new dependencies through validated `Operation` construction rather than
mutating operation internals.

## Implementation notes

Implemented the pure planner in `crates/core/src/daemon.rs`. Marketplace
refreshes are keyed by exact marketplace `ResourceKey` plus `HarnessId` and are
built in stable map order; tracked plugins resolve only the marketplace with
their selector and scope. Missing, target-mismatched, pinned, disabled, and
malformed relationships are retained as typed per-target blockers. Added
`Operation::with_added_dependencies` rebuilds through `Operation::new`, so
adapter and daemon edges remain subject to the existing operation contract.

## Verification

- `cargo test -p skilltap-core daemon::tests --lib` — 8 passed.
- `cargo test -p skilltap-core daemon_native_plan --lib` — 4 passed.
- `cargo test -p skilltap-core operation_dependency_addition_revalidates_the_union --lib` — passed, including plan wire round-trip.
- `cargo fmt --all` — passed.

The inventory relationship assumptions matched the approved design; no design
bounce was required.

## Expected implementation surface

- `crates/core/src/daemon.rs`
- `crates/core/src/marketplace.rs`
- `crates/core/src/domain/operation.rs`

## Acceptance evidence

- Two plugins sharing one marketplace, target, and scope produce one refresh and
  two dependent plugin tasks.
- Equal marketplace names in different targets or project scopes remain
  distinct.
- Missing, mismatched-target, pinned, disabled, and malformed prerequisites
  block only the affected plugin task.
- Planning is stable regardless of inventory serialization order.
- Added operation dependencies preserve operation-contract, graph, and wire
  round-trip invariants.

## Ordering

This checkpoint establishes the task identities and dependency graph consumed by
`feature-daemon-marketplace-refresh-execution`.
