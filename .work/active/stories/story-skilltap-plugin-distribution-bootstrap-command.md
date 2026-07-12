---
id: story-skilltap-plugin-distribution-bootstrap-command
kind: story
stage: review
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
- Review weight: standard (source: autopilot project default).
- Files changed: `crates/cli/src/command.rs`, `crates/cli/src/dispatch.rs`, `crates/cli/src/entrypoint.rs`, command/compiled-binary tests, `crates/core/src/runtime/artifact.rs`.
- Tests added: bootstrap grammar/help, isolated fixture install, same-major no-op, blocked major upgrade, unknown-version safety, harness attention separation, and failure-preserving artifact paths.
- Discrepancies from design: release resolution accepts a strict local manifest override for isolated environments and otherwise uses the bounded canonical GitHub latest endpoint; existing executable versions are probed directly and unknown versions block replacement.
- Adjacent issues parked: none.

## Review resolution
- Reconnected the binary path to `FileReleaseResolver`/`SystemReleaseResolver`, `ArtifactFetcher`, and `BinaryInstaller`.
- Added GitHub latest-release and checksum parsing, direct version probing, major-safe decisioning, and isolated fixture coverage for install, no-op, and blocked major paths.

## Review findings (2026-07-12)

- **Blocker**: The public command does not compose or execute the binary
  bootstrap boundary. `execute_system_bootstrap` only checks whether
  `SKILLTAP_RELEASE_MANIFEST` is present, reports `planned` when it is set,
  and never resolves a release manifest, selects the platform artifact,
  fetches it, verifies its checksum, installs it atomically, or probes the
  installed identity. The environment variable is not even read as a
  manifest path or transport input. Consequently `--allow-major` has no
  effect, fresh installs and same-major updates cannot succeed, major updates
  cannot be blocked based on an observed version, and the command can report
  an apparently planned binary operation without any corresponding mutation
  or verified result. Wire the existing `ReleaseResolver`, artifact fetcher,
  and `BinaryInstaller` ports (or an equivalent application composition) into
  the command, add isolated first-install/update/no-op/blocked-major and
  failure-preservation coverage, and report the actual binary result before
  advancing this story.

## Review (2026-07-12)

**Verdict**: Request changes

**Blockers**: binary bootstrap boundary is disconnected from the command (this item)
**Important**: none
**Nits**: none

**Notes**: Substrate review at standard weight, escalated to a focused
correctness/public-contract pass because this is the agent-facing bootstrap
command and a security-sensitive release boundary. Reviewed commit `176e812`,
the bootstrap feature design, `docs/SPEC.md`, `docs/ARCH.md`, the core artifact
ports, harness adapter, CLI implementation, help output, and the compiled
attention test. The CLI grammar and separate per-harness result shape are
present, but the only binary-path test proves the deliberate unavailable
fallback; setting `SKILLTAP_RELEASE_MANIFEST` merely changes the label to
`planned` and still performs no binary operation. The item remains at
`stage: implementing` until the command is wired to verified artifact
resolution/installation and its acceptance matrix is covered.
