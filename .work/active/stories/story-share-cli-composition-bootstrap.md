---
id: story-share-cli-composition-bootstrap
kind: story
stage: done
tags: [refactor]
parent: null
depends_on: []
release_binding: 3.0.0
gate_origin: refactor-design
created: 2026-07-12
updated: 2026-07-12
---

# Share CLI application composition bootstrap

## Value

The CLI entrypoint repeats repository, runner, path, scope, and
`StatusApplication` composition in reconciliation, adopt, and status wrappers.
A shared private bootstrap reduces drift while preserving status's distinct
platform-path error mapping.

## Scope

Extract the repeated setup around `crates/cli/src/entrypoint.rs:881-997` into
one private composition helper. Keep command-specific error/result handling
and borrowed lifetimes unchanged.

## Acceptance

- Reconciliation, adopt, and status use the shared bootstrap.
- Existing output, errors, and command behavior remain unchanged.
- Workspace tests, formatting, and clippy stay green.

## Implementation Notes

- Added a private `with_system_application` composition helper that owns
  platform-path, repository, scope resolver, and system adapter construction.
- Routed reconciliation, adopt, and status through the helper while retaining
  status's distinct `platform_paths_unavailable` mapping.
- Verification: `cargo fmt --all -- --check` and `cargo check -p skilltap --offline`
  passed.

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Standard same-harness fresh-context review. The shared bootstrap
retains repository construction, scope resolver wiring, borrowed lifetimes,
and status-specific platform-path errors; workspace fmt, tests, clippy, and
diff checks are green.
