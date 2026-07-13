---
id: gate-security-project-ancestor-symlink-escape
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

# Prevent managed project writes through symlink ancestors

## Severity
High

## Location
`crates/cli/src/application.rs:1313`; `crates/cli/src/application/execution.rs:343`

## Required fix
Bind project file mutation beneath a no-follow project root, reject symlink
ancestors, and prove a hostile `.agents` link cannot create or modify an
external catalog.

## Implementation notes
- Execution capability: inline; the change is one cohesive filesystem boundary plus its managed-project call sites.
- Review weight: standard (project default).
- Files changed: `crates/core/src/runtime/filesystem.rs`, `crates/core/src/runtime/mod.rs`, `crates/core/src/runtime/filesystem/directory_tree.rs`, `crates/core/src/runtime/filesystem/directory_tree/unix_support.rs`, `crates/core/src/runtime/filesystem/directory_tree/tests.rs`, `crates/cli/src/application.rs`, `crates/cli/src/application/execution.rs`.
- Tests added: hostile `.agents` ancestor symlink is rejected for confined read, write, and remove while the external target remains untouched.
- Discrepancies from design: none; shared bounded-read support landed here because the descriptor-confined primitive also underpins the bounded-observation item.
- Adjacent issues parked: none.
- Verification: focused core adversarial test and `cargo check -p skilltap` pass.
