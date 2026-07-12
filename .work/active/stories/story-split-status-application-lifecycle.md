---
id: story-split-status-application-lifecycle
kind: story
stage: review
tags: [refactor]
parent: feature-split-status-application
depends_on: [story-split-status-application-execution-ports]
release_binding: 3.0.0
gate_origin: refactor-design
created: 2026-07-12
updated: 2026-07-12
---

# Extract native lifecycle and skill flows

## Brief

Move daemon/update orchestration, native marketplace/plugin lifecycle, lifecycle
preview, and standalone-skill install/update/remove flows into
`crates/cli/src/application/lifecycle.rs` after the execution-port extraction.
Keep the application entrypoint signatures and all native, Git, compatibility,
state, and acknowledgment behavior unchanged.

## Current / target

Current methods are `execute_daemon_cycle`, `execute_lifecycle_preview`,
`execute_native_lifecycle`, `execute_skill_install`, `execute_skill_update`,
and `execute_skill_remove` in `application.rs:633-3030`. Their supporting
`NativeLifecycleSpec`, `SkillDestination`, Git source resolution, lifecycle and
skill operation IDs, native presence/state seed helpers, and daemon result
helpers are top-level through `application.rs:5560`.

Target `lifecycle.rs` contains `impl StatusApplication<'_>` blocks with the
same `pub(crate)` methods and private helpers. `NativeLifecycleKind` and
`SkillInstallRequest` remain parent-facing types because `entrypoint.rs`
constructs them. Import execution ports and shared document/scope helpers via
`super`; do not introduce a second adapter or executor.

## Acceptance criteria

- Native marketplace/plugin lifecycle, complete skill-tree validation,
  Git revision/sha tracking, compatibility warnings, state journaling, daemon
  records, and generic `acknowledged: bool` behavior remain identical.
- Existing operation IDs, error/warning codes, output fields, and idempotent
  no-change results are unchanged.
- Native lifecycle, skill, daemon, and Git-source tests plus workspace fmt,
  tests, and clippy pass.

## Risk / rollback

This is the broadest extraction and has many helper references. A changed
import or call target could alter which adapter runs. Revert this commit only;
execution ports remain safely extracted and lifecycle behavior returns to the
pre-step layout.

## Implementation Notes

- Moved daemon/update orchestration, lifecycle preview, native lifecycle, and
  standalone skill install/update/remove methods into the private
  `application/lifecycle.rs` module.
- Kept all supporting helpers, adapter construction, operation IDs, state
  journal wiring, and entrypoint signatures unchanged; the child module uses
  the parent support surface via `super`.
- Verification: `cargo fmt --all`, `cargo check -p skilltap --offline`, and
  `cargo test -p skilltap --offline` passed (40 unit tests and 41 compiled-
  binary tests).
