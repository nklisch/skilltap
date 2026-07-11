---
id: epic-rust-control-plane-domain-contracts-capability-compatibility
kind: story
stage: implementing
tags: []
parent: epic-rust-control-plane-domain-contracts
depends_on: [epic-rust-control-plane-domain-contracts-identity-scope-source]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Define Capability and Compatibility Evidence

## Scope

Implement Unit 3 from the parent feature: extensible capability identifiers and
support states plus separate behavioral-compatibility and transfer-fidelity
classifications carrying target-specific evidence and consequences.

## Acceptance criteria

- [ ] Dotted capability ids are validated without closing the future namespace.
- [ ] Capability sets distinguish supported, unsupported, and unverified.
- [ ] Behavioral compatibility and transfer fidelity cannot be conflated.
- [ ] Every non-faithful result requires machine-readable evidence and a material
  consequence suitable for agent output.
- [ ] Representative classifications serialize deterministically and round-trip.
- [ ] Locked format, clippy, and workspace tests pass.
