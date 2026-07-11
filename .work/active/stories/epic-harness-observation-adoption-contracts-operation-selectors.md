---
id: epic-harness-observation-adoption-contracts-operation-selectors
kind: story
stage: review
tags: [infra]
parent: epic-harness-observation-adoption-contracts
depends_on: [epic-harness-observation-adoption-contracts-resource-graph]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Migrate Exact Operation Selectors

Move resource/component operation selectors, acknowledgment sets, consequence
coverage, and errors/wires to exact `ResourceKey`. Enforce selector scope equals
operation semantic scope and preserve all partial/blocked dependency behavior.

## Implementation

- Migrated resource and component selectors from logical `ResourceId` fields to
  one strict nested `ResourceKey`. The same exact selector type continues
  through acknowledgment requirements, attention reasons, operation wires,
  errors, containment, and consequence coverage.
- Added constructor and deserialization enforcement that the selector key's
  scope equals `OperationSemantics::scope`; mismatches return the exact
  operation, resource key, and semantic scope.
- Preserved operation dependency validation, partial acknowledgment matching,
  blocked dependency propagation, result validation, and deterministic plan
  ordering without adding scope inference.
- Added same-ID global/project coexistence, cross-scope acknowledgment
  rejection, semantic-scope mismatch, deterministic exact-key wire, and legacy
  `resource_id` rejection coverage.

## Verification

- `cargo test -p skilltap-core domain::operation --locked` — 21 passed.
- `cargo fmt --all -- --check` — passed.
- `cargo check --workspace --all-targets --locked` — passed.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` — passed.
- `cargo test --workspace --locked` — 211 tests passed across unit,
  integration, and compiled-binary suites.
- `cargo doc --workspace --no-deps --locked` — passed.
- `cargo build --workspace --release --locked` — passed.
