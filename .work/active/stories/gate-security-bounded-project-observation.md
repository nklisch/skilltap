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
