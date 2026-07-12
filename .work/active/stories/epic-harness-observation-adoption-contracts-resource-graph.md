---
id: epic-harness-observation-adoption-contracts-resource-graph
kind: story
stage: done
tags: [infra]
parent: epic-harness-observation-adoption-contracts
depends_on: [epic-harness-observation-adoption-contracts-resource-key]
release_binding: 3.0.0
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Migrate Resource Graph Identity

Make desired and observed resources, observation keys, graph maps/errors, and
dependencies use exact `ResourceKey`. Remove redundant observed scope, add
typed optional source, and retain resolved/unresolved observed dependency
evidence without aborting healthy siblings. Preserve deterministic wires and
validate exact self/dangling/cycle contexts.

## Implementation

- Replaced desired resource `id`/`scope` storage and graph identity with one
  strict nested `ResourceKey`; derived convenience accessors do not create a
  second source of truth.
- Migrated observation keys, dependency sets, graph maps, and graph diagnostics
  to exact scope-bearing keys. Equal logical IDs coexist globally and in
  multiple projects, and desired dependencies can resolve across scopes.
- Removed redundant observed scope and arbitrary observation metadata. Added a
  typed optional `Source` and strict resolved/unresolved observed dependency
  evidence. Unresolved or absent observed siblings remain visible without
  aborting the rest of the snapshot; exact self-edges and cycles still fail.
- Added deterministic strict-serde coverage, rejection of legacy sibling
  `id`/`scope` wires, exact diagnostic coverage, cross-scope cycle coverage,
  source round trips, and scope-explicit `ResourceKey` display.

## Verification

- `cargo check -p skilltap-core --locked` — passed.
- `cargo clippy -p skilltap-core --lib --locked -- -D warnings` — passed.
- `cargo doc -p skilltap-core --no-deps --locked` — passed.
- The workspace test/full release ladder is intentionally deferred to the
  dependent storage-wire bridge: current storage unit/integration fixtures
  still call the pre-migration constructors and therefore cannot compile until
  that coordinated clean-break wire migration lands.

## Review

- Approved in the coordinated exact-key migration review after the storage
  bridge restored the full workspace ladder.
- Independent review found two observation diagnostic renderers that omitted
  project scope. Both now render the complete `ResourceKey`, with regression
  coverage for duplicate and self-dependent project observations.
- Locked workspace format, all-target check, warnings-denied Clippy, and all
  209 tests pass after the correction.
