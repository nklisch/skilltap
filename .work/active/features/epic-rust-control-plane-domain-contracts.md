---
id: epic-rust-control-plane-domain-contracts
kind: feature
stage: drafting
tags: []
parent: epic-rust-control-plane
depends_on: [epic-rust-control-plane-workspace-reset]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Control-Plane Domain Contracts

## Brief

Define the validated `skilltap-core` vocabulary shared by every later
capability: harness and resource identities, scope and target selection, source
and revision identities, desired and observed resource graphs, provenance,
capabilities, compatibility, fingerprints, operation results, and attention
reasons. Boundary constructors reject invalid raw values so internal code does
not exchange unvalidated strings for identity-bearing concepts.

This feature establishes types and invariants, not reconciliation algorithms,
harness-specific interpretation, persistence implementations, or CLI rendering.
Harness-specific metadata remains namespaced and opaque to the general domain.

## Epic context

- Parent epic: `epic-rust-control-plane`
- Position in epic: shared-contract producer — storage and runtime primitives
  depend on its validated types

## Foundation references

- `docs/ARCH.md` — Domain Model, Core Types, Dependency Direction
- `docs/SPEC.md` — Terminology, Operating Model, Skill Compatibility
- `docs/HARNESS-CONTRACTS.md` — Common Capability Model, Identity Mapping

<!-- The feature design pass will fill in implementation units. -->
