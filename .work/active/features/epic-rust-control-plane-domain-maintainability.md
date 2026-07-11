---
id: epic-rust-control-plane-domain-maintainability
kind: feature
stage: implementing
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

## Refactor overview

The domain contracts are behaviorally complete but concentrated in two very
large production modules and repeat two kinds of boundary machinery. First move
the inline tests without changing them, then consolidate string-newtype support,
then extract graph traversal. Each step is independently buildable and
revertible; golden serialized-form/error tests protect black-box behavior.

## Refactor steps

### Step 1: Externalize resource tests

**Priority:** Medium
**Risk:** Low
**Source lens:** code smell
**Files:** `crates/core/src/domain/resource.rs`,
`crates/core/src/domain/resource/tests.rs`
**Story:** `epic-rust-control-plane-domain-maintainability-resource-tests`

**Current state:**

```rust
// resource.rs
#[cfg(test)]
mod layered_tests {
    // ~600 lines
}
```

**Target state:**

```rust
// resource.rs
#[cfg(test)]
mod tests;
```

Move the body mechanically to `resource/tests.rs`, retain access to private
items through the child-module relationship, and preserve every test name.

**Acceptance criteria:** format/clippy/tests pass; test inventory is unchanged;
no production code or public/wire behavior changes.

**Rollback:** revert this story commit; no other step depends on the file path.

### Step 2: Externalize operation tests

**Priority:** Medium
**Risk:** Low
**Source lens:** code smell
**Files:** `crates/core/src/domain/operation.rs`,
`crates/core/src/domain/operation/tests.rs`
**Story:** `epic-rust-control-plane-domain-maintainability-operation-tests`

Apply the same mechanical child-module move to the ~1,300-line operation test
body. Preserve private-item visibility, test names, assertions, and fixtures.

**Acceptance criteria:** format/clippy/tests pass; test inventory is unchanged;
no production code or public/wire behavior changes.

**Rollback:** revert this story commit independently of the resource move.

### Step 3: Consolidate validated string newtypes

**Priority:** High/Medium
**Risk:** Medium
**Source lens:** missing abstraction
**Files:** new private support under `crates/core/src/domain/`, plus
`identity.rs`, `compatibility.rs`, `source.rs`, `capability.rs`, `resource.rs`,
and `scope.rs` where an exact migration is possible
**Story:** `epic-rust-control-plane-domain-maintainability-validated-newtypes`

**Current state:** local macros/manual implementations each repeat constructor,
display, serde, and accessors.

**Target state:** one crate-private configurable macro/helper owns the common
shape while each type still supplies its existing validator, maximum length,
normalization, error kind, and trait set. Leave paths, Git commits, fingerprints,
and any other custom wire/normalization type bespoke when exact equivalence is
not obvious.

**Acceptance criteria:** all public methods/traits and JSON remain identical;
constructor and serde error text/order remain identical; golden tests and full
workspace checks pass; net boilerplate decreases without a more complex API.

**Rollback:** revert the single migration commit; serialized data is unchanged.

### Step 4: Share private dependency-graph traversal

**Priority:** High
**Risk:** High
**Source lens:** missing abstraction
**Files:** new private graph support plus `resource.rs` and `operation.rs`
**Story:** `epic-rust-control-plane-domain-maintainability-dependency-graphs`

Extract reusable private primitives for known-reference/self-edge checks and
exact cycle membership. Resource/component/observation and operation adapters
map those private results back to their existing public error variants and
messages. Preserve the intentional difference between one contextual resource
cycle and the operation set semantics; do not expose the generic machinery.

**Acceptance criteria:** every existing invalid-graph test returns the same
public error variant/member set/message; add equivalence tests around downstream
non-cycle nodes and multiple cycles; full locked checks pass; no public export.

**Rollback:** revert the extraction commit to restore local algorithms.

## Implementation order

1. Resource and operation test moves in parallel.
2. Validated-newtype consolidation after both moves.
3. Dependency-graph extraction last.

## Atomic steps acknowledged

None. Every step is behavior-preserving and independently revertible.
