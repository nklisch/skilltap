---
id: epic-rust-control-plane-runtime-primitives-scope-target
kind: story
stage: done
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

## Implementation notes

- Files changed: new `crates/core/src/runtime/scope.rs`, runtime exports in `runtime/mod.rs`, and path-specific additions to the shared typed error contract in `runtime/error.rs`.
- Public surface: mutually exclusive `ScopeRequest`, deterministic `ResolvedScopes`, `WorkingDirectory`/`SystemWorkingDirectory`, `GitRoot`/`CommandGitRoot`, composed `ScopeResolver`, and `resolve_targets` over enabled harness identifiers.
- Tests added: 7 tests covering current nested Git roots, explicit file-to-parent Git resolution, concrete non-Git fallback, missing/unsuitable inputs, global/all-scopes ordering and deduplication without scans, mutually exclusive request variants, and omitted/all/named/disabled/empty target selection.
- Git behavior: `CommandGitRoot` invokes `git -C <canonical-directory> rev-parse --show-toplevel` through `CommandRunner`; a non-zero Git result means no containing repository and falls back to the canonical directory, while spawn/wait and malformed-output failures remain typed.
- Scope behavior: project paths canonicalize through `FileSystem`; regular-file inputs use their containing directory; global/all-scopes avoid filesystem, working-directory, and Git access; all-scopes consumes only caller-supplied recorded project roots.
- Discrepancies from design: none.
- Adjacent issues parked: none.
- Dispatch rationale: direct-read only; the completed runtime ports and domain selection types fully defined the integration points.
- Verification: `cargo fmt --all -- --check`, `cargo check --locked --workspace --all-targets`, `cargo clippy --locked --workspace --all-targets -- -D warnings`, `cargo test --locked --workspace`, and `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps` all pass (85 workspace tests).

## Review

Approved. Scope requests make project and all-scopes mutually exclusive by
construction. Current and explicit inputs canonicalize, files resolve through
their parent, containing Git roots win, and ordinary non-repository results
fall back to the canonical directory. Global/all-scopes avoid working-directory,
filesystem, and Git discovery; recorded projects sort and deduplicate. Target
resolution preserves the domain contract for omitted, all, named, disabled,
and empty sets. All seven focused tests pass on review.
