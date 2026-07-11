---
id: epic-rust-control-plane-domain-maintainability
kind: feature
stage: drafting
tags: [refactor]
parent: epic-rust-control-plane
depends_on: [epic-rust-control-plane-domain-contracts]
release_binding: null
gate_origin: refactor-design
created: 2026-07-11
updated: 2026-07-11
---

# Consolidate Domain Contract Internals

## Brief

Reduce the maintenance and review cost of the completed domain-contract surface
without changing public types, serialized forms, validation behavior, errors, or
test coverage.

## Discovery findings

### Shared dependency-graph primitives

**Classification:** pure refactor  
**Value:** high  
**Source lens:** missing abstraction

`crates/core/src/domain/resource.rs` contains component, desired-resource, and
contextual observed-resource graph validation beginning near lines 199, 1033,
and 1065. `crates/core/src/domain/operation.rs` independently validates the
operation graph beginning near line 916. Extract only private traversal and
reference-validation machinery while preserving each public error type/message
and its exact cycle-member semantics.

### Validated string-newtype support

**Classification:** pure refactor  
**Value:** high/medium  
**Source lens:** missing abstraction

Constructor/display/serialize/deserialize boilerplate recurs across
`identity.rs`, `compatibility.rs`, `source.rs`, `capability.rs`, `resource.rs`,
and `scope.rs` (representative macros begin at `identity.rs:7` and
`compatibility.rs:35`). Introduce private configurable support and migrate only
types whose existing validators, normalization, trait set, errors, and wire form
can remain byte-for-byte equivalent. Do not force paths, Git hashes, or other
custom-normalized types into a leaky macro.

### Externalize large inline tests

**Classification:** pure refactor  
**Value:** medium  
**Source lens:** code smell

The inline operation tests occupy roughly 1,302 of 3,023 lines and resource
tests roughly 627 of 1,877 lines. Move them into dedicated submodule files while
preserving test names, module visibility, coverage, and production module paths.
This reduces the primary production working sets by about 43% and 33%.

## Constraints

- Black-box behavior, public API, serde JSON, validation ordering, error variants
  and messages, deterministic ordering, and all current tests remain unchanged.
- `GraphCollection::Observed` is not part of this refactor; removing a public
  variant is an API change.
- No new capability, schema migration, or compatibility behavior belongs here.
