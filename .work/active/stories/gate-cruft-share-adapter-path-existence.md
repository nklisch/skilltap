---
id: gate-cruft-share-adapter-path-existence
kind: story
stage: drafting
tags: [cleanup]
parent: null
depends_on: []
release_binding: 3.1.0
gate_origin: cruft
created: 2026-04-02
updated: 2026-07-15
---

# Share adapter path-existence helpers

## Confidence
Medium

## Category
duplicated helper

## Location
`crates/harnesses/src/adapters/cursor.rs:162` and ten sibling adapter copies; canonical helper at `crates/harnesses/src/adapter_helpers.rs:324`

## Evidence
Eleven private adapter helpers repeat one of two identical `symlink_metadata(...).is_ok()` shapes, while `adapter_helpers` already owns the child-path form.

## Removal
Expose cohesive path-existence helpers from `adapter_helpers`, migrate adapter callers, and delete private copies. Preserve the current behavior that a dangling symlink counts as present.
