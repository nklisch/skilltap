---
id: epic-rust-control-plane-runtime-primitives-publication-residuals
kind: story
stage: review
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

## Implementation notes

- Files changed: `crates/core/src/runtime/error.rs`, `crates/core/src/runtime/filesystem.rs`, and
  runtime exports in `crates/core/src/runtime/mod.rs`.
- Public contract: replaced the mutually exclusive publication-state enum with
  `PublicationResiduals`, a deterministic set of `PublicationResidual` values carrying exact
  `Temporary` or `Destination` roles and absolute paths, plus independent `DirectorySyncState`
  (`NotRequired`, `Synced`, or `Uncertain`). Read-only accessors expose structured recovery data.
- Cleanup behavior: post-publication recovery now attempts exact-inode destination rollback,
  exact-inode temporary cleanup, and parent-directory sync independently, then returns an ordinary
  filesystem error only when no residual remains and rollback durability is confirmed.
- Tests updated: deterministic injected seams cover pre-publication temporary residue,
  destination-only residue, temporary-only residue after destination rollback, both residues, no
  residual paths with uncertain sync, exact on-disk residual paths, and safe deterministic display.
- Discrepancies from design: none. Successful publication, atomic no-clobber behavior, and fully
  cleaned ordinary failures retain their prior behavior.
- Verification: locked format, all-target check, warnings-denied Clippy, workspace tests (93 core
  tests), and warnings-denied rustdoc pass.
- Adjacent issues parked: none.
