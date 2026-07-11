---
id: epic-rust-control-plane-domain-contracts-identity-scope-source
kind: story
stage: review
tags: []
parent: epic-rust-control-plane-domain-contracts
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Define Validated Identity, Scope, and Source Primitives

## Scope

Implement Unit 1 from the parent feature: serde-safe identity newtypes,
concrete and selected scopes, deterministic harness targets, source identity,
requested/resolved revisions, absolute/relative paths, and fingerprints.

## Acceptance criteria

- [x] Invalid raw and deserialized values are rejected by the same typed errors.
- [x] Target selections resolve to non-empty deterministic harness sets.
- [x] Paths, Git commits, and fingerprints enforce the parent invariants.
- [x] Public serialized forms are stable snake_case and round-trip through JSON.
- [x] Locked format, clippy, and workspace tests pass.

## Implementation notes

- Files changed: workspace and core manifests, `crates/core/src/lib.rs`, and the new `crates/core/src/domain/{mod,identity,scope,source}.rs` modules.
- Tests added: 10 focused unit tests covering raw and serde validation, bounded opaque native ids, stable string and snake-case JSON forms, lexical path constraints, deterministic non-empty harness sets, target resolution, Git SHA-1/SHA-256 object ids, requested/resolved source revisions, and parsed SHA-256/SHA-512 fingerprints.
- Discrepancies from design: none.
- Adjacent issues parked: none.
- Dispatch rationale: direct-read only; Unit 1 had a bounded new-module surface with no existing integration ambiguity.
- Verification: `cargo fmt --all -- --check`, `cargo check --locked --workspace`, `cargo clippy --locked --workspace --all-targets -- -D warnings`, and `cargo test --locked --workspace` all pass.
