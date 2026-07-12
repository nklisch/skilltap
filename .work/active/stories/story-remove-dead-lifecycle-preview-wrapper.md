---
id: story-remove-dead-lifecycle-preview-wrapper
kind: story
stage: implementing
tags: [refactor]
parent: null
depends_on: []
release_binding: 3.0.0
research_refs: []
research_origin: null
gate_origin: refactor-design
created: 2026-07-12
updated: 2026-07-12
---

# Remove dead CLI lifecycle-preview wrapper

## Discovery finding

`crates/cli/src/entrypoint.rs:319-337` defines the private
`execute_system_lifecycle_preview` wrapper and suppresses the compiler's
dead-code lint. A repository-wide caller search finds no caller; the actual
preview behavior is reached through `StatusApplication::execute_lifecycle_preview`
inside reconciliation. The wrapper is leftover dispatch scaffolding and has no
public or test-facing entry point.

## Classification

Pure refactor / dead weight. Removing the unreachable private wrapper and its
`#[allow(dead_code)]` annotation changes no command, application, or output
behavior.

## Implementation

Delete the unused wrapper only. Do not remove
`StatusApplication::execute_lifecycle_preview` or its reconciliation callers;
those are live application behavior. Run the CLI and workspace test suites plus
strict clippy to prove no hidden caller or contract changes.

## Acceptance criteria

- [ ] `execute_system_lifecycle_preview` and its dead-code allowance are gone.
- [ ] No other lifecycle preview implementation or reconciliation path changes.
- [ ] `cargo fmt --all -- --check`, offline workspace tests, strict clippy, and
      `git diff --check` pass.

## Risk / rollback

Risk is low: the function is private and has no callers. Restore the deleted
wrapper if a compile or test search reveals a missed call site.
