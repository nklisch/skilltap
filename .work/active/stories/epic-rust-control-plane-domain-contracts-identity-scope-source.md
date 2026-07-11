---
id: epic-rust-control-plane-domain-contracts-identity-scope-source
kind: story
stage: implementing
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

- [ ] Invalid raw and deserialized values are rejected by the same typed errors.
- [ ] Target selections resolve to non-empty deterministic harness sets.
- [ ] Paths, Git commits, and fingerprints enforce the parent invariants.
- [ ] Public serialized forms are stable snake_case and round-trip through JSON.
- [ ] Locked format, clippy, and workspace tests pass.
