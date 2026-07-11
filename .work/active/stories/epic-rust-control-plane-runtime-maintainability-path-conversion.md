---
id: epic-rust-control-plane-runtime-maintainability-path-conversion
kind: story
stage: review
tags: [refactor]
parent: epic-rust-control-plane-runtime-maintainability
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Centralize Runtime Path Conversion

Extract one private helper for the five identical UTF-8 `Path`/`PathBuf` to
`AbsolutePath` conversions with role-aware `RuntimeError` mapping. Keep
environment parsing separate and preserve exact variants, messages, and error
ordering. Run all path/scope/filesystem tests and the full locked ladder.

## Implementation notes

- Files changed: new private `crates/core/src/runtime/path_value.rs`, private module registration,
  and the five matching call sites in `paths.rs`, `scope.rs`, and `filesystem.rs`.
- Extraction: `absolute_path(&Path, PathRole)` performs the existing UTF-8 check followed by
  `AbsolutePath` validation and maps failures to the same `NonUtf8Path` then `InvalidPath` variants.
- Call sites migrated: platform child join, current working directory, Git metadata marker, project
  parent, and filesystem canonicalization.
- Tests added: none; existing path (6), scope (10), and filesystem (12) suites cover the exact
  behavior and all pass unchanged.
- Discrepancies from design: none. Environment-variable parsing, Git stdout decoding, publication
  invariant conversion, and lock-specific conversion remain separate because their errors or
  invariants differ.
- Verification: focused suites, locked format/all-target check, warnings-denied Clippy, full
  workspace tests, and warnings-denied rustdoc pass.
- Adjacent issues parked: none.
