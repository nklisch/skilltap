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

- [ ] `ObservationKey` includes `ResourceId`, `HarnessId`, and an explicit
  declared/effective layer; duplicate exact keys fail while the same resource
  across harnesses/layers is preserved.
- [ ] Representation-specific health, components, dependencies, native identity,
  revision, fingerprint, provenance, ownership, and metadata live on one
  observed instance rather than mixed per-harness maps.
- [ ] Desired resources preserve direct versus adopted origin (including source
  harness), explicit default/include/exclude component choices over the complete
  component graph, and accepted material consequences per target.
- [ ] Constructors and serde reject source-harness/target mismatches, selections
  for absent components, consequences for untargeted harnesses, and invalid
  dependency graphs within each observation context.
- [ ] Resource/component cycle errors report actual cycle members, not merely
  downstream nodes blocked by a cycle.
- [ ] Deterministic representative multi-harness, two-layer graphs round-trip.
- [ ] Locked format, clippy, and workspace tests pass.
