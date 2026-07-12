---
id: epic-real-harness-recovery-runtime-boundary-process-context
kind: story
stage: review
tags: [correctness, security, testing]
parent: epic-real-harness-recovery-runtime-boundary
depends_on: []
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Build the explicit native process context

## Scope

Extend the validated platform-path contract with XDG cache and Claude config
roots, construct the exact six-variable native child environment, and expand
isolated test machines so every harness process is contained by explicit roots.

## Acceptance

- Default/custom roots and global instructions resolve independently.
- Native children receive only HOME, XDG config/cache, Codex home, Claude
  config, and PATH values supplied by the request.
- Missing or invalid required values fail before spawn.
- Test support snapshots prove host Codex/Claude/config trees are untouched.

## Implementation

- `PlatformPaths` now resolves independent cache, Codex, Claude, and global
  instruction roots, including `XDG_CACHE_HOME` and `CLAUDE_CONFIG_DIR`.
- Detection, observation, bootstrap, and lifecycle execution receive one
  explicit six-variable environment; the native runner's existing clear-env
  boundary prevents ambient variables from crossing into harness processes.
- Isolated-machine fixtures provide every root and an isolated PATH variant.
  Harness integration coverage captures the child environment and proves an
  unlisted variable is absent.
- The first-use compiled-binary test now uses an isolated executable path, so
  a developer's real Codex or Claude installation cannot influence the result.

## Verification

- `cargo test --workspace`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
