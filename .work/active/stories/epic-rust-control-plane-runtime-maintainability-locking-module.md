---
id: epic-rust-control-plane-runtime-maintainability-locking-module
kind: story
stage: implementing
tags: [refactor]
parent: epic-rust-control-plane-runtime-maintainability
depends_on:
  - epic-rust-control-plane-runtime-maintainability-unix-identity
  - epic-rust-control-plane-runtime-maintainability-publication-module
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Extract Configuration Locking

Move the configuration lock port, system adapter, guard, acquisition helpers,
and release behavior to `runtime/filesystem/locking.rs`. Preserve public names,
nonblocking/RAII semantics, lock ordering, identity checks, error precedence,
and all adversarial identities. Remove only now-dead parameters/imports and run
the full locked ladder.
