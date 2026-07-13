---
id: story-skilltap-plugin-distribution-bootstrap-cli-rollback-race-coverage
kind: story
stage: done
tags: [infra, security, testing]
parent: epic-skilltap-plugin-distribution-bootstrap
depends_on: [story-skilltap-plugin-distribution-bootstrap-cli-rollback-safety]
release_binding: 3.0.2
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Complete deterministic CLI rollback race coverage

The CLI rollback helper uses atomic exchange primitives, but its current tests
cover only a replacement observed before rollback and the normal restoration
path. The story's contract also requires replacement-during-rollback,
first-install cleanup races, and residual temporary-file behavior, including a
truthful attention result whenever a replacement wins.

## Acceptance criteria

- Linux and macOS rollback paths preserve a replacement that arrives before or
  during rollback; no stale path operation overwrites or unlinks that inode.
- A replacement that prevents a clean restoration/removal is surfaced as
  `attention_required`/recovery attention, while a normal matching restoration
  remains completed and idempotent.
- First-install cleanup removes only the expected published inode, preserves a
  replacement appearing during cleanup, and leaves an explicitly reported
  residual when safe cleanup cannot establish ownership.
- Isolated tests deterministically cover replacement-before-rollback,
  replacement-during-rollback, no-prior cleanup, residual temporary files,
  normal prior restoration, and unsupported-platform fail-closed behavior.
- Existing checksum, version, major-policy, and artifact-boundary behavior is
  unchanged; tests run without touching the operator's install or harness
  paths.

## Review origin

Fresh-context review of `story-skilltap-plugin-distribution-bootstrap-cli-rollback-safety`
found that the implementation had no synchronization seam for the required
replacement-during-rollback and residual cleanup scenarios, and could report a
clean restoration when a replacement arrived after the first exchange.

## Implementation notes

- Execution capability: highest; this is a security-sensitive file publication
  and recovery boundary.
- Review weight: standard (source: autopilot).

## Implementation notes

- Execution capability: highest; rollback identity and residual cleanup are
  security-sensitive publication boundaries.
- Review weight: standard (source: autopilot).
- Files changed: `crates/cli/src/entrypoint.rs`.
- Tests added: deterministic replacement-during-exchange, replacement-during
  first-install cleanup, residual temporary-file, and normal restoration paths.
- Discrepancies from design: unsupported platforms continue to fail closed;
  Unix helpers use injected post-exchange/cleanup hooks in tests while the
  production path remains hook-free.
- Adjacent issues parked: none.

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Substrate standard review of replacement-during-exchange,
first-install cleanup races, residual cleanup, normal restoration, and
unsupported-platform fail-closed paths. The implementation preserves foreign
replacement identities and reports recovery attention. `cargo fmt --all
-- --check`, full offline workspace tests, and `cargo clippy
--workspace --all-targets --offline -- -D warnings` are green.
