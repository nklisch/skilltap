---
id: gate-tests-mutating-scope-matrix
kind: story
stage: implementing
tags: [testing]
parent: null
depends_on: []
release_binding: 3.0.0
gate_origin: tests
created: 2026-07-12
updated: 2026-07-12
---

# Cover project and all-scopes mutation boundaries

## Priority

High

## Spec reference

Scope and target contract in `docs/SPEC.md` and the
`epic-harness-observation-adoption-integration` scope matrix.

## Gap type

Missing mutating project and `--all-scopes` coverage proving exact resource
keys and isolation of unrelated scopes.

## Suggested test

Create two isolated Git project roots plus global resources; run project-scoped
and all-scopes install/update/remove with target subsets, asserting inventory,
state, native trees, and untouched global/project bytes.

## Test location (suggested)

`crates/cli/tests/compiled_binary.rs`

## Implementation Notes

Added `native_mutations_keep_project_and_all_scope_boundaries`, covering
global and Claude project installs, all-scopes removal, target-subset removal,
inventory scope retention, and project isolation.

The SPEC-backed test currently exposes a production defect: all-scopes removal
with same-named global/project resources returns `operation_plan_invalid`
because lifecycle operation IDs omit scope and collide. Parked as
`idea-scope-aware-lifecycle-operation-ids`; do not weaken or skip this test.
The story remains blocked on that fix and review of the resulting regression.

## Blocker

The regression cannot pass until lifecycle operation identities include the
concrete scope. See parked item `idea-scope-aware-lifecycle-operation-ids`.
