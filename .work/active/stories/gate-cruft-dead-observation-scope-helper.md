---
id: gate-cruft-dead-observation-scope-helper
kind: story
stage: review
tags: [cleanup]
parent: null
depends_on: []
release_binding: 3.0.0
gate_origin: cruft
created: 2026-07-12
updated: 2026-07-12
---

# Remove dead observation scope helper

## Confidence

Medium

## Category

Dead function and warning suppression

## Location

`crates/core/src/reconciliation.rs:256-259`

## Evidence

`_scope_of_observation` is marked `#[allow(dead_code)]` and repository-wide
search found no call sites beyond its definition.

## Removal

Delete the helper and remove any import made unused by its removal while
preserving `ObservationKey` and its test usage.

## Autopilot implementation note

This is a bounded behavior-preserving cleanup with a complete removal scope;
no separate design expansion is required.

## Implementation Notes

- Removed the unused `_scope_of_observation` helper and its `dead_code`
  suppression from `crates/core/src/reconciliation.rs`.
- Removed the now-unused production imports while keeping `ObservationKey`
  explicitly imported by the reconciliation tests.
- Verification passed: `cargo test -p skilltap-core --offline` (292 unit tests,
  10 integration/doc tests), `cargo fmt --all -- --check`, and
  `cargo check -p skilltap-core --offline`.
