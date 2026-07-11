---
id: epic-rust-control-plane-runtime-primitives
kind: feature
stage: review
tags: [infra]
parent: epic-rust-control-plane
depends_on: [epic-rust-control-plane-domain-contracts]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Runtime Boundary Primitives

## Brief

Provide reusable ports and platform adapters for global/project scope
resolution, target resolution, canonical paths, atomic filesystem operations,
process-wide fail-fast configuration locking, time, and direct executable-plus-
argument-vector command invocation. Return typed boundary errors and captured
command evidence without writing to the terminal or leaking secrets.

These primitives support later harness adapters and reconciliation execution,
but do not encode Codex or Claude commands, semantic planning, or resource
lifecycle behavior. Synchronous operation remains the default unless measured
behavior later justifies concurrency.

## Epic context

- Parent epic: `epic-rust-control-plane`
- Position in epic: independent infrastructure consumer of the shared domain
  contracts; can proceed in parallel with storage

## Foundation references

- `docs/SPEC.md` — Operating Model, Mutation Safety, Platform Contract
- `docs/ARCH.md` — Native Command Execution, Concurrency, Error Model,
  Technology
- `AGENTS.md` — Architecture, Development

## Design

### Boundary and ownership

Runtime primitives live in `skilltap-core::runtime`. They are infrastructure
ports plus macOS/Linux adapters, not application services: they do not know
Codex or Claude commands, storage schemas, reconciliation semantics, or CLI
rendering. Domain values such as `AbsolutePath`, `ScopeSelection`,
`TargetSelection`, and `HarnessSet` remain the vocabulary at the boundary.

The module owns four cohesive surfaces:

1. `RuntimeError` and platform-path resolution for the machine configuration
   root and home-relative canonical locations.
2. A synchronous `CommandRunner` port and process adapter accepting an
   executable and argument vector, plus a `Clock` port and system clock.
3. A `FileSystem` port and platform adapter for metadata without implicit link
   following, directory creation, same-directory atomic publication, relative
   links, and recoverable copies; plus a fail-fast configuration lock whose
   guard owns the lock lifetime.
4. A pure scope resolver backed by working-directory, canonicalization, and Git
   root ports. No scope flag resolves global; `--project` resolves the current
   or supplied containing Git root, falling back to the canonical directory;
   all-scopes expansion consumes explicit recorded project roots. Target
   resolution continues to use the domain contract and fails on an empty or
   disabled selection.

Concrete adapters support Linux and macOS only. No shell command strings,
async runtime, global mutable process state, terminal writes, or harness
behavior enter this feature. Command results capture status, stdout, stderr,
and elapsed duration but never log, persist, or include inherited environment
values in structured errors. Atomic publication writes a complete temporary
file in the destination directory, flushes it, renames it over the destination,
and cleans up failed temporaries; callers never observe a partially written
replacement. Lock acquisition is non-blocking and reports contention as a
typed error.

### Dependency direction

`runtime` may depend on `domain`; storage and later harness/application layers
depend on `runtime`. This makes the runtime filesystem adapter the single owner
of atomic publication rather than duplicating an atomic writer in storage.
Consequently the sibling storage feature must follow this feature even though
their schema and repository design remain independent.

### Failure model

Errors name the boundary operation and relevant safe path or executable while
retaining a typed category and source. Invalid environment/path input, missing
home, unsupported platform, non-UTF-8 canonical paths, command spawn/wait
failure, atomic publication failure, and lock contention are distinguishable.
Captured native stdout/stderr remain ordinary command output for the adapter to
validate; they are not interpolated into generic error text.

### Pre-mortem

- **Atomic writes are only rename-shaped, not durable.** Require a temporary in
  the destination directory, file flush/sync, rename, parent sync where the
  platform supports it, cleanup tests, and old-or-new reader assertions.
- **Symlink handling overwrites user content through a followed link.** Expose
  link metadata separately, never infer ownership from a resolved target, and
  test links and dangling links explicitly.
- **A second CLI or daemon silently waits forever.** Acquire the one machine
  configuration lock non-blockingly, hold it by RAII, and test contention and
  release.
- **Project scope changes with spelling or nested working directories.**
  canonicalize first, prefer the containing Git root, and test nested,
  non-repository, explicit-path, and global defaults.
- **Command evidence leaks credentials.** Keep inherited environment and argv
  out of errors and persisted evidence; capture output only in the returned
  value and make redaction an explicit later adapter responsibility.

## Implementation units

1. `epic-rust-control-plane-runtime-primitives-errors-paths` — typed runtime
   errors and deterministic platform/home/config path resolution — depends on
   `[]`.
2. `epic-rust-control-plane-runtime-primitives-command-clock` — synchronous
   direct-argv command and time ports with system adapters — depends on
   `[epic-rust-control-plane-runtime-primitives-errors-paths]`.
3. `epic-rust-control-plane-runtime-primitives-filesystem-lock` — link-aware
   filesystem operations, atomic publication, backups, and fail-fast locking —
   depends on `[epic-rust-control-plane-runtime-primitives-errors-paths]`.
4. `epic-rust-control-plane-runtime-primitives-scope-target` — deterministic
   global/project/all-scopes and enabled-target resolution — depends on
   `[epic-rust-control-plane-runtime-primitives-command-clock,
   epic-rust-control-plane-runtime-primitives-filesystem-lock]`.

## Acceptance criteria

- Public runtime ports and concrete adapters are synchronous, terminal-free,
  and warnings-clean on supported Linux and macOS builds.
- Configuration and home paths follow the documented XDG/HOME contract and
  produce validated canonical domain paths.
- Commands are executed without a shell and return status, output streams, and
  duration; typed failures do not echo argument vectors or environment values.
- Atomic replacement and backup operations demonstrate old-or-new visibility,
  cleanup on failure, and explicit symlink treatment in isolated tests.
- Exactly one writer holds the process-wide configuration lock; contention
  fails immediately and releasing the guard permits the next writer.
- Scope tests cover the global default, current and explicit nested projects,
  Git-root preference, non-Git fallback, all-scopes ordering/deduplication, and
  path failures. Target tests cover all enabled and one enabled/disabled target.
- `cargo fmt --all -- --check`, locked all-target check, Clippy with warnings
  denied, workspace tests/doctests, and warnings-clean rustdoc pass.

## Implementation summary

All four children are complete. `skilltap-core::runtime` now exposes safe typed
boundary errors and XDG/home paths; direct-argv synchronous command execution
and deterministic/system clocks; link-aware filesystem operations, durable
same-directory atomic file publication, recoverable copies, relative symlinks,
and a fail-fast RAII configuration lock; plus global/project/all-scopes and
enabled-target resolution. The implementation is terminal-free, performs no
resource discovery, and retains storage, harness, and reconciliation semantics
outside the runtime layer. The locked workspace passes 85 tests plus doctests
and warnings-clean rustdoc.
