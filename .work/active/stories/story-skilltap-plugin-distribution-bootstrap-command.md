---
id: story-skilltap-plugin-distribution-bootstrap-command
kind: story
stage: implementing
tags: [infra, content, testing]
parent: epic-skilltap-plugin-distribution-bootstrap
depends_on: [story-skilltap-plugin-distribution-bootstrap-artifacts, story-skilltap-plugin-distribution-bootstrap-harness]
release_binding: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# First-class bootstrap command and result contract

Expose the bootstrap application through the Rust CLI and compose the pure
release policy, artifact installer, and harness setup adapters. Add the
non-interactive `skilltap bootstrap` leaf with `--target`, `--allow-major`, and
`--json`; the command must report binary state and each target's plugin state
separately.

Scope:

- `crates/cli/src/command.rs`, `dispatch.rs`, `entrypoint.rs`.
- `crates/cli/src/application/bootstrap.rs` and composition exports.
- Command/entrypoint tests and compiled-binary coverage.

Acceptance criteria:

- Help describes global scope, target narrowing, major-version acknowledgment,
  JSON representation, no prompts, and exit classes.
- Plain and schema-1 JSON output distinguish install/update/no-op from each
  harness result and include safe next actions for missing/unsupported targets.
- Fresh install takes latest release; existing major upgrades require
  `--allow-major`; same-major repeats are idempotent.
- Failures before verified publish leave binary/config/native state unchanged;
  no `--yes` or arbitrary source argument is introduced.
- Compiled tests cover target narrowing, absent harnesses, mixed success and
  attention, blocked major upgrade, and healthy repeat in isolated roots.

Keep command grammar authoritative in clap metadata and preserve existing
output schema/result classes. Do not duplicate the full grammar in plugin or
website prose.

## Implementation notes
- Execution capability: highest available local capability; this changes the public agent-facing CLI and composes security-sensitive adapters.
- Review weight: standard (autopilot project default).
