---
id: epic-rust-control-plane-runtime-primitives-scope-target
kind: story
stage: implementing
tags: [infra]
parent: epic-rust-control-plane-runtime-primitives
depends_on:
  - epic-rust-control-plane-runtime-primitives-command-clock
  - epic-rust-control-plane-runtime-primitives-filesystem-lock
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Scope and Target Resolution

## Brief

Resolve CLI scope and target selections into deterministic domain values using
the shared runtime boundaries.

## Acceptance criteria

- No scope flag resolves global; project selection canonicalizes the current or
  supplied location and uses its containing Git root when present.
- Outside Git, project selection uses the canonical directory itself; file
  inputs resolve through their containing directory or fail explicitly when
  unsuitable.
- All-scopes combines global with explicit recorded canonical projects in
  deterministic deduplicated order and never scans for projects.
- Project and all-scopes remain mutually exclusive at the request boundary.
- Omitted/all targets resolve to every enabled harness; a named target must be
  enabled and an empty enabled set fails.
- Fake-port and temporary-Git tests cover nested roots, non-Git directories,
  explicit paths, failures, ordering, and target selection.
- Locked formatting, all-target check, Clippy, tests, and rustdoc pass.
