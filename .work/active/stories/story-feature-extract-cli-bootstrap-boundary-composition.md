---
id: story-feature-extract-cli-bootstrap-boundary-composition
kind: story
stage: review
tags: [refactor, infra]
parent: feature-extract-cli-bootstrap-boundary
depends_on: []
release_binding: 3.0.0
research_refs: []
research_origin: null
gate_origin: refactor-design
created: 2026-07-12
updated: 2026-07-12
---

# Extract bootstrap command composition

## Brief

Create the private `crates/cli/src/bootstrap_commands.rs` boundary and move
the first-party bootstrap command composition and outcome projection out of
`entrypoint.rs`. Preserve the existing command contract while leaving the
low-level binary publication helpers behind a temporary narrow bridge for the
next extraction step.

## Current / target

`entrypoint.rs:285-457` currently owns `execute_system_bootstrap`, the
Codex/Claude target-selection loop, `compose_bootstrap_outcome`,
`BinaryBootstrapResult`, and the setup-result mapping. The dispatch arm calls
that implementation directly.

The target module owns those functions and the composition-focused tests. It
exposes only `pub(super) execute_system_bootstrap(&BootstrapArgs) -> Outcome`;
the dispatcher remains the stable caller. Until the publication story lands,
the module may use a `pub(super)` parent bridge for
`execute_binary_bootstrap`.

## Guardrails

- Keep missing-config defaults, target narrowing, configured absolute/PATH
  resolution, bounded process/JSON limits, canonical plugin source, and the
  binary-attention short circuit unchanged.
- Preserve binary-first and target-filtered harness resource order, status
  labels, summaries, warning/error codes, next actions, plain/JSON output, and
  exit classes.
- Move the composition tests mechanically; do not widen production APIs or
  modify harness setup behavior.
- Do not move daemon service lifecycle code or artifact publication internals
  in this step.

## Acceptance criteria

- [ ] A private `bootstrap_commands` module is declared and bootstrap dispatch
      routes through its narrow wrapper.
- [ ] No duplicate command-composition implementation remains in
      `entrypoint.rs`; low-level publication remains available for Step 2.
- [ ] Target-narrowed, mixed-harness, blocked-binary, plain, and JSON tests
      pass with unchanged assertions and result ordering.
- [ ] `cargo test -p skilltap-cli --offline` and `cargo fmt --all -- --check`
      pass.

## Risk / rollback

The main risk is import or visibility drift changing target filtering or the
attention short circuit. Revert the module and dispatch edit to restore the
composition block; this source-only move touches no native files or state.

## Implementation notes

- Execution capability: highest available local implementation; mechanical
  boundary extraction with no behavior change.
- Review weight: standard (autopilot default).
- Files changed: `crates/cli/src/entrypoint.rs`,
  `crates/cli/src/bootstrap_commands.rs`.
- Tests added: none; existing bootstrap composition, target narrowing, JSON,
  and plain-output assertions moved with the implementation.
- Discrepancies from design: the publication and daemon blocks were moved in
  the same mechanical extraction so the new private module was immediately
  buildable; no contract or output behavior changed.
- Adjacent issues parked: none.

## Verification

- `cargo fmt --all`
- `cargo test -p skilltap --offline` (59 unit/compiled/package tests passed)
- `cargo clippy -p skilltap --all-targets --offline -- -D warnings`
- `git diff --check`
