---
id: epic-native-marketplace-plugin-lifecycle-identity
kind: feature
stage: done
tags: []
parent: epic-native-marketplace-plugin-lifecycle
depends_on: []
release_binding: 3.0.0
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Normalize Explicit Marketplace and Plugin Identity

Represent one explicitly selected marketplace or plugin with scope, source,
logical identity, native identity, and source association kept separate.

## Design

- Accept only explicit locators and exact `plugin@marketplace` selectors.
- Names that happen to match across harnesses never imply association.
- Keep requested source/ref separate from observed native version and identity.
- Reject ambiguous scope or malformed selectors before native execution.

## Acceptance

Identity values round-trip deterministically and equal names in distinct scopes
remain distinct resources.

## Implementation notes

Added `skilltap_core::marketplace` with exact `plugin@marketplace` parsing and
scope-bearing marketplace identity values. Source, logical identity, and
native association remain separate.

## Review

### Verdict

Approve with comments.

### Findings

- Harness adapters must supply native lifecycle capability checks and preserve
  explicit source association; matching names alone remain insufficient.

### Verification

Focused identity tests and strict core clippy pass.
