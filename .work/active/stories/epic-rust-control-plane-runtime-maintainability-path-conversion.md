---
id: epic-rust-control-plane-runtime-maintainability-path-conversion
kind: story
stage: implementing
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
