---
id: epic-rust-control-plane-domain-maintainability-validated-newtypes
kind: story
stage: done
tags: [refactor]
parent: epic-rust-control-plane-domain-maintainability
depends_on: [epic-rust-control-plane-domain-maintainability-resource-tests, epic-rust-control-plane-domain-maintainability-operation-tests]
release_binding: null
gate_origin: refactor-design
created: 2026-07-11
updated: 2026-07-11
---

# Consolidate Validated String Newtypes

Introduce crate-private support for repeated validated string-newtype
constructor/display/serde/accessor behavior and migrate only exact matches.
Preserve public APIs, trait sets, validation/normalization order, error text, and
wire forms. Leave custom paths, Git hashes, fingerprints, and leaky exceptions
bespoke. Full golden and locked workspace tests must remain unchanged.

## Implementation notes

- Files changed: added private `crates/core/src/domain/validated_newtype.rs`; migrated exact matches in `identity.rs`, `compatibility.rs`, `source.rs`, `capability.rs`, and `resource.rs`; registered only the private module in `domain/mod.rs`.
- Migrated types: `HarnessId`, `ResourceId`, `OperationId`, `NativeId`, `EvidenceCode`, `EvidenceDetail`, `ConsequenceCode`, `ConsequenceSummary`, `SourceLocator`, `RequestedRevision`, `CapabilityId`, and `ComponentId`.
- Preserved variations: the macro's sole trait switch retains `TryFrom<String>` only for the four identity types and `CapabilityId`; each invocation retains its original validator, kind, maximum length, derives, constructor/accessor/display/serde surface, errors, and string JSON wire form.
- Bespoke types retained: `AbsolutePath`, `RelativeArtifactPath`, `GitCommit`, and `Fingerprint`; capability dotted-segment validation remains a local private validator passed to the shared machinery.
- Tests added: none; the unchanged 56-test workspace inventory retains constructor/serde error equivalence and golden JSON coverage across the migrated types.
- Net boilerplate reduction: 134 production lines across the touched domain surface, including the 60-line private support module.
- Discrepancies from design: none.
- Adjacent issues parked: none.
- Dispatch rationale: direct-read only; the repeated implementations and their exact trait differences were locally enumerable.
- Verification: `cargo fmt --all -- --check`, `cargo check --locked --workspace`, `cargo clippy --locked --workspace --all-targets -- -D warnings`, `cargo test --locked --workspace`, and rustdoc with warnings denied all pass.

## Review (2026-07-11)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Crate-private support preserves exact public traits, validators,
errors, and wire forms across 12 types while reducing production code by 134
lines. Custom-normalized types remain bespoke. Locked tests/clippy/rustdoc pass;
fast-lane advance.
