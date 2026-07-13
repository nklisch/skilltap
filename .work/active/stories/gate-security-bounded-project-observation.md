---
id: gate-security-bounded-project-observation
kind: story
stage: review
tags: [security]
parent: null
depends_on: []
release_binding: 3.0.2
gate_origin: security
created: 2026-07-12
updated: 2026-07-12
---

# Bound hostile managed project observation

## Severity
Medium

## Location
`crates/cli/src/application.rs:1801`; `crates/core/src/runtime/filesystem/directory_tree/tree_io.rs:91`

## Required fix
Apply entry, depth, per-file, total-byte, and document limits before loading
existing project catalogs, MCP configuration, or skill trees.

## Implementation notes
- Execution capability: inline; bounded observation extends the descriptor-confined filesystem primitive from the prerequisite security item.
- Review weight: standard (project default).
- Files changed: `crates/core/src/runtime/filesystem/directory_tree.rs`, `crates/core/src/runtime/filesystem/directory_tree/tree_io.rs`, `crates/core/src/runtime/filesystem/directory_tree/unix_support.rs`, `crates/core/src/runtime/filesystem/directory_tree/tests.rs`, `crates/cli/src/application.rs`.
- Tests added: one adversarial fixture independently exceeds document, entry, depth, per-file, and total-byte limits.
- Discrepancies from design: item began at drafting because Medium gate findings route there, but its required-fix brief was complete and the caller explicitly commissioned implementation.
- Adjacent issues parked: none.
- Verification: focused core limit test and `cargo check -p skilltap` pass; the latter reports a temporary dead-code warning in another worker's uncommitted test seam.

## Review findings (2026-07-12)

- **Blocker — managed skill-tree call paths are not consistently bounded or
  correctly typed.** Planning converts every
  `load_tree_bounded_no_follow` failure into absence with `.ok()`
  (`crates/cli/src/application.rs:1849`), so a byte/entry/depth violation or
  no-follow rejection does not reach the caller as an observation failure.
  Revalidation, post-apply verification, and rollback still call the unbounded
  `load_tree_no_follow` API (`crates/cli/src/application/execution.rs:276`,
  `:378`, `:461`, `:474`). A hostile tree introduced or enlarged after planning
  can therefore be fully allocated under the execution lock, contrary to the
  required pre-allocation bounds. Keep this item implementing until every
  managed-project skill-tree observation uses the bounded port and preserves a
  typed failure instead of treating errors as missing.

## Review (2026-07-12)

**Verdict**: Request changes

**Blockers**: managed skill-tree observation is unbounded on execution paths
and bounded planning errors are erased (`gate-security-bounded-project-observation`)
**Important**: none
**Nits**: none
**Rejected**: none

**Notes**: Substrate Deep review at effective review weight `standard` (explicit
caller selection), performed in fresh context because hostile-input allocation
is a security surface. The core bounded reader correctly checks directory
entry, depth, metadata length, per-file, total-byte, and document limits before
large allocation, and its focused adversarial tests pass. The blocker is in
application/execution composition, not the primitive. No foundation-doc drift
or public API break found; product/UX lenses were inapplicable.

## Review finding resolution (2026-07-12)

- Execution capability: inline; the finding was cohesive across one planner,
  one execution port, and their shared lifecycle fixture.
- Review weight: `standard` from the explicit caller selection.
- Files changed: `crates/cli/src/application.rs`,
  `crates/cli/src/application/execution.rs`, and
  `crates/cli/src/application/tests.rs`.
- Planning now distinguishes a genuinely missing tree from a bounded/no-follow
  observation failure and returns `managed_project_plugin_unreadable` for the
  latter.
- Revalidation, post-apply verification, and rollback now share the same
  root-confined depth, entry, per-file, total-byte, and path limits. Rollback
  treats an unreadable tree as an owned residual instead of guessing absence.
- Regression coverage creates an oversized sparse tree before planning and
  again between planning and locked revalidation; both fail without a
  publication attempt and retain the hostile surface for truthful observation.
- Simplification: one limit constructor and bounded observation helper replace
  duplicated/unbounded lifecycle reads.
- Discrepancies from design: none.
- Adjacent issues parked: none.
- Verification: focused managed-project security/recovery tests, CLI library
  tests, formatting, and strict CLI test Clippy.
