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
- Tests added: 12 focused unit tests covering raw and serde validation, bounded opaque native ids, stable string and snake-case JSON forms, canonical lexical path constraints, deterministic non-empty harness sets, target resolution, Git SHA-1/SHA-256 object ids, source-kind-specific requested/resolved revisions, unknown-field rejection, and parsed SHA-256/SHA-512 fingerprints.
- Discrepancies from design: none.
- Adjacent issues parked: none.
- Dispatch rationale: direct-read only; Unit 1 had a bounded new-module surface with no existing integration ambiguity.
- Verification: `cargo fmt --all -- --check`, `cargo check --locked --workspace`, `cargo clippy --locked --workspace --all-targets -- -D warnings`, and `cargo test --locked --workspace` all pass.

## Review findings (2026-07-11)

- Blocker: `AbsolutePath` and `RelativeArtifactPath` reject `.`/`..` but retain
  duplicate separators and trailing separators, allowing equivalent filesystem
  locations to compare as different identity values. Enforce one lexical form
  at both constructor and serde boundaries.
- Blocker: `Source` is infallible across fields, permits revisions on local or
  native sources, and lacks the explicit remote-catalog source kind required by
  the foundation. Use a fallible constructor/deserializer; permit requested
  revisions only for Git; require a validated absolute path for local sources;
  model remote catalogs explicitly; keep native identity in `NativeId` and
  `ResolvedRevision`, not as a source kind.
- Important: skilltap-owned tagged/wire values should reject unknown fields so
  nested persisted contracts cannot silently discard schema drift.

## Review fix notes

- Canonical path identity now rejects duplicate and trailing separators in both constructors and serde deserialization, in addition to `.` and `..` components.
- `SourceKind` is `Git`, `Local`, or `RemoteCatalog`; `Source::new` is fallible, its fields are private, and deserialization passes through the same constructor.
- Local source locators must satisfy `AbsolutePath`; only Git sources may carry `RequestedRevision`; opaque native versions remain represented by `ResolvedRevision::Native(NativeId)`.
- Skilltap-owned source and fingerprint wires plus scope, scope-selection, target-selection, and resolved-revision tagged values reject unknown fields.
- Regression coverage includes raw and serde forms for non-canonical paths and source-kind violations, plus unknown fields on owned object/tagged forms.
- Verification after fixes: `cargo fmt --all -- --check`, `cargo check --locked --workspace`, `cargo clippy --locked --workspace --all-targets -- -D warnings`, and `cargo test --locked --workspace` all pass.
