---
id: epic-rust-control-plane-runtime-primitives-filesystem-lock
kind: story
stage: implementing
tags: [infra]
parent: epic-rust-control-plane-runtime-primitives
depends_on: [epic-rust-control-plane-runtime-primitives-errors-paths]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Atomic Filesystem and Configuration Lock

## Brief

Implement the shared filesystem adapter for owned files and the single
machine-wide, fail-fast mutation lock.

## Acceptance criteria

- Metadata inspection distinguishes regular files, directories, symlinks, and
  dangling links without silently following links.
- Directory creation, managed relative symlink creation, recoverable copies,
  reads, and removals use typed operations and validated paths.
- Atomic publication uses a same-directory temporary, complete write and sync,
  atomic rename, parent sync where supported, and failed-temporary cleanup;
  readers observe the old or new complete bytes.
- Existing user content is never followed or replaced through a symlink by an
  ownership-sensitive operation.
- The configuration lock is acquired non-blockingly, held for the guard
  lifetime, reports contention distinctly, and becomes available after drop.
- Temp-home tests cover successful replacement, injected failure cleanup,
  backups, relative and dangling links, contention, and release.
- Locked formatting, all-target check, Clippy, tests, and rustdoc pass.
