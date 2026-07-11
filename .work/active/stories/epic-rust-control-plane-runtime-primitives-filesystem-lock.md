---
id: epic-rust-control-plane-runtime-primitives-filesystem-lock
kind: story
stage: review
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

## Implementation notes

- Files changed: new `crates/core/src/runtime/filesystem.rs`, runtime exports in
  `crates/core/src/runtime/mod.rs`, and typed unsafe-symlink/canonical-path context in
  `crates/core/src/runtime/error.rs`.
- Public surface: synchronous `FileSystem` and `ConfigurationLock` ports; link-aware
  `FileMetadata`; validated parent-capable `RelativeSymlinkTarget`; system filesystem and
  nonblocking RAII lock adapters.
- Tests added: seven isolated temp-directory tests covering regular/directory/live-link/dangling-
  link inspection, whole-file visibility under a concurrent reader, injected atomic-write failure
  cleanup, non-overwriting recoverable copies, parent-relative link validation, symlink-safe writes
  and removal, and explicit/drop lock release after fail-fast contention.
- Discrepancies from design: no external locking or temporary-file dependency was necessary because
  pinned Rust 1.96 provides nonblocking file locks; the adapter uses exclusive same-directory files
  and the standard filesystem APIs directly.
- Verification: locked workspace format, all-target check, Clippy with warnings denied, tests (78
  core tests), and rustdoc with warnings denied all pass.
- Adjacent issues parked: none.
