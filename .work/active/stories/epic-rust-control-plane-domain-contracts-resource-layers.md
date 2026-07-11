---
id: epic-rust-control-plane-domain-contracts-resource-layers
kind: story
stage: implementing
tags: []
parent: epic-rust-control-plane-domain-contracts
depends_on: [epic-rust-control-plane-domain-contracts-resource-graph, epic-rust-control-plane-domain-contracts-capability-compatibility]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Preserve Desired Origin and Layered Harness Observations

## Scope

Correct the resource contracts found incomplete during feature review. Desired
state must preserve adoption origin, the complete component graph plus explicit
include/exclude choices, and accepted consequences per target. Observations must
be keyed by resource, harness, and native-declared/effective-installed layer so
different representations never overwrite one another.

## Acceptance criteria

- [x] `ObservationKey` includes `ResourceId`, `HarnessId`, and an explicit
  declared/effective layer; duplicate exact keys fail while the same resource
  across harnesses/layers is preserved.
- [x] Representation-specific health, components, dependencies, native identity,
  revision, fingerprint, provenance, ownership, and metadata live on one
  observed instance rather than mixed per-harness maps.
- [x] Desired resources preserve direct versus adopted origin (including source
  harness), explicit default/include/exclude component choices over the complete
  component graph, and accepted material consequences per target.
- [x] Constructors and serde reject source-harness/target mismatches, selections
  for absent components, consequences for untargeted harnesses, and invalid
  dependency graphs within each observation context.
- [x] Resource/component cycle errors report actual cycle members, not merely
  downstream nodes blocked by a cycle.
- [x] Deterministic representative multi-harness, two-layer graphs round-trip.
- [x] Locked format, clippy, and workspace tests pass.

## Implementation notes

- Files changed: `crates/core/src/domain/resource.rs` and this story.
- Added `ObservationKey` and `ObservationLayer`; observed records now contain
  scalar state for exactly one resource/harness/declared-or-effective context,
  and graph identity uses the complete key.
- Added `DesiredOrigin`, `ComponentChoice`, target-keyed accepted
  `MaterialConsequence` sets, private desired/observed fields, strict serde
  wires, accessors, and fallible constructors that enforce cross-field context.
- Observed dependencies resolve only within their harness/layer. Desired,
  observed, and component cycle errors use deterministic DFS back-edges to
  report exact cycle members rather than downstream blocked nodes.
- Tests added: 15 focused raw/serde tests covering adoption source targets,
  complete component choice maps, accepted consequence targets/components,
  exact observation duplicates, cross-context dependency rejection,
  multi-harness/two-layer deterministic round trips, exact cycle diagnostics,
  malformed findings, opaque metadata, strict wires, and stable enum forms.
- Discrepancies from design: retained the existing `OpaqueHarnessMetadata` public
  name for source compatibility, but layered `ObservedResource` uses one opaque
  JSON value because its harness namespace is now carried by `ObservationKey`.
- Adjacent issues parked: none.

## Review findings (2026-07-11)

- Important: adopted source harness is historical provenance, independent of
  the current target set. Remove the requirement that an adopted source harness
  remain targeted and add a round-trip case for “adopt from Claude, manage only
  Codex.”
- Verification: `cargo fmt --all -- --check`, `cargo check --workspace --locked`,
  `cargo clippy --workspace --all-targets --locked -- -D warnings`, and
  `cargo test --workspace --locked` pass with 52 core tests.
