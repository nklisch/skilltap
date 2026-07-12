---
id: gate-cruft-dead-observation-scope-helper
kind: story
stage: drafting
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
