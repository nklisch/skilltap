---
id: epic-native-marketplace-plugin-lifecycle-identity
kind: feature
stage: drafting
tags: []
parent: epic-native-marketplace-plugin-lifecycle
depends_on: []
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
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
