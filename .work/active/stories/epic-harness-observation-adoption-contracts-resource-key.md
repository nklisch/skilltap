---
id: epic-harness-observation-adoption-contracts-resource-key
kind: story
stage: done
tags: [infra]
parent: epic-harness-observation-adoption-contracts
depends_on: []
release_binding: null
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Add Scope-Bearing Resource Keys

Add validated `ResourceKey { id, scope }`, canonical encoding, serde/order/hash
contracts, and a ResourceId-specific alphabet supporting documented qualified
plugin spelling without relaxing other identifiers. Prove equal IDs across
global and multiple projects remain distinct and malformed qualified IDs fail.

## Implementation notes

- Files changed: `crates/core/src/domain/identity.rs`,
  `crates/core/src/domain/scope.rs`, and `crates/core/src/domain/mod.rs`.
- Added `ResourceKey` as a strict nested serde value with exact `ResourceId`
  and `Scope` accessors, total Eq/Ord/Hash semantics, and a domain-separated,
  versioned, length-prefixed canonical byte encoding for stable hashing.
- Added a ResourceId-only qualified validator: unqualified IDs retain the
  existing grammar, while one nonempty `plugin@marketplace` pair is accepted
  only when both halves independently satisfy that grammar. Harness and
  operation identifiers remain unchanged.
- Tests added: qualified ResourceId constructor/serde acceptance and malformed
  rejection; strict nested key wire rejection; equal logical IDs across global
  and two project scopes in ordered and hashed sets; exact canonical global and
  project byte fixtures.
- Verification: locked format, workspace check, Clippy with warnings denied,
  196 workspace tests, rustdoc with warnings denied, optimized release build,
  and the compiled-binary wrapper all pass.
- Discrepancies from design: none.
- Adjacent issues parked: none.
- Dispatch rationale: the item named one bounded domain surface, so direct
  implementation was sufficient and no exploratory fanout was used.

## Review

Approved. Scope-bearing keys are strict, totally ordered/hashable, and use
domain-separated unambiguous bytes; only ResourceId gains one validated `@`
qualification, with every other identifier grammar unchanged.
