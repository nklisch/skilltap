---
id: gate-tests-mutating-scope-matrix
kind: story
stage: done
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

The SPEC-backed regression initially exposed a production defect in
scope-omitting lifecycle operation IDs; that fix landed in `9bc22a7` and the
test now passes. The inventory assertion was also corrected to accept the
documented multiline TOML rendering while still checking both targets in the
specific resource section.

Verification: the focused scope-matrix test passes against the fixed
implementation.

Extended the scope matrix with equal `same@team` global/project resources,
all-scopes removal, state/inventory assertions, unchanged native-tree
snapshots, and a project sentinel byte check. The existing target-subset and
unrelated-project assertions remain intact.

Verification: the focused scope-matrix test passes against the fixed
implementation.

## Review (2026-07-12)

**Verdict**: Request changes

**Blockers**: end-to-end scope-isolation matrix remains incomplete (this item)
**Important**: none
**Nits**: none

**Notes**: Standard fresh-context substrate review with correctness, tests,
scope-isolation, and state-boundary lenses. The regression covers a global
resource, one project resource, all-scopes removal, target-subset retention,
and inventory isolation. It does not exercise equal resource IDs in global and
project scopes, nor assert native trees, state records, and untouched bytes
remain isolated across project/all-scopes mutations as required by the gate
brief. Add those same-name and native/state assertions while retaining the
existing coverage.

## Follow-up Resolution

Extended the scope matrix with equal `same@team` global/project resources,
all-scopes removal, state/inventory assertions, unchanged native-tree
snapshots, and a project sentinel byte check. The focused scope-matrix test
passes against the fixed implementation.

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Standard fresh-context substrate review at review weight standard.
The focused compiled-binary test passes. Equal logical ids are exercised in
global and project scopes, all-scopes removal preserves the unrelated project
resource, target-subset removal preserves the remaining target, inventory and
state records are asserted, the native tree is snapshotted, and an untouched
project sentinel is preserved. The scope-aware operation-id fix is covered by
the matrix and no test weakening or foundation drift was found.
