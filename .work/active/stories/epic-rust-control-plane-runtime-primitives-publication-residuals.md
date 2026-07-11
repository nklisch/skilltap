---
id: epic-rust-control-plane-runtime-primitives-publication-residuals
kind: story
stage: implementing
tags: [infra, correctness]
parent: epic-rust-control-plane-runtime-primitives
depends_on: [epic-rust-control-plane-runtime-primitives-filesystem-hardening]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Report Exact Publication Residuals

## Brief

Make recoverable-copy partial errors sufficient for deterministic recovery
without scanning by separating residual owned paths from parent-directory
durability.

## Acceptance criteria

- Partial publication context identifies every generated temporary or published
  destination path whose removal could not be proven, with its role.
- Directory-sync state is represented independently as not-required, synced,
  or uncertain, so it can coexist with any residual-path combination.
- Post-publication cleanup attempts destination rollback, temporary cleanup,
  and parent sync independently; one failure does not suppress the others.
- Tests cover pre-publication temp residue; destination-only residue;
  temp-only residue after successful destination rollback; both residues; no
  residual paths with uncertain sync; and safe structured/display output.
- Successful publication and cleaned ordinary failures retain existing public
  behavior; full locked format/check/Clippy/test/rustdoc ladder passes.
