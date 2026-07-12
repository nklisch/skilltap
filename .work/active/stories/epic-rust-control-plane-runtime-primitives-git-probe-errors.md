---
id: epic-rust-control-plane-runtime-primitives-git-probe-errors
kind: story
stage: done
tags: [infra, correctness]
parent: epic-rust-control-plane-runtime-primitives
depends_on: [epic-rust-control-plane-runtime-primitives-scope-target]
release_binding: 3.0.0
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Classify Git Root Probe Failures

## Brief

Preserve non-repository project fallback while refusing to silently reinterpret
an existing broken, inaccessible, or rejected Git repository as a nested
standalone project.

## Acceptance criteria

- A directory with no containing Git metadata and an ordinary nonzero probe
  still returns `None` so project scope falls back to the canonical directory.
- When a nonzero probe occurs and `.git` metadata exists at the candidate or an
  ancestor, return a typed safe Git-root error containing directory/status but
  not stderr or environment content.
- Ancestor inspection is limited to the supplied canonical directory chain; it
  does not discover projects or resources elsewhere.
- Tests cover ordinary non-Git fallback, corrupt `.git` file/directory, a
  rejected probe fixture, nested metadata, and safe error rendering.
- Full locked format/check/Clippy/test/rustdoc ladder passes.

## Implementation notes

- Added `RuntimeError::GitRootProbe`, classified at the path boundary and
  rendered with only the canonical directory and optional exit status.
- On a nonzero Git probe, `CommandGitRoot` now inspects only `.git` entries on
  the supplied canonical directory's ancestor chain. Missing entries preserve
  ordinary non-repository fallback; existing metadata or an inspection failure
  returns the typed probe error without exposing stderr or source-error text.
- Reused the `FileSystem` and `CommandRunner` ports. The existing constructor
  retains its call shape, while an injected filesystem constructor supports
  deterministic boundary tests.
- Added tests for corrupt `.git` files and directories, nested ancestor
  metadata, rejected probes, inaccessible metadata, bounded inspection, and
  safe rendering. The existing ordinary non-Git fallback test remains green.
- Files changed: `crates/core/src/runtime/command.rs`,
  `crates/core/src/runtime/error.rs`, and
  `crates/core/src/runtime/scope.rs`.
- Verification passed: `cargo fmt --all -- --check`,
  `cargo check --locked --workspace --all-targets`,
  `cargo clippy --locked --workspace --all-targets -- -D warnings`,
  `cargo test --locked --workspace` (93 tests), and
  `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`.
- No design discrepancies or adjacent issues were found.

## Review

Approved. Nonzero probes now inspect only `.git` markers on the supplied
canonical ancestor chain. A metadata-free chain preserves ordinary non-Git
fallback, while existing or inaccessible metadata produces a typed error with
only safe directory/status context. Corrupt file/directory, nested, rejected,
inaccessible, bounded-inspection, and secrecy cases are covered. Ten focused
scope tests and warnings-denied workspace Clippy pass on review.
