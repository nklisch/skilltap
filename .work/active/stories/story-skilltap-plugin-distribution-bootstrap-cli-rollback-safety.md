---
id: story-skilltap-plugin-distribution-bootstrap-cli-rollback-safety
kind: story
stage: done
tags: [infra, security, testing]
parent: epic-skilltap-plugin-distribution-bootstrap
depends_on: [story-skilltap-plugin-distribution-bootstrap-command]
release_binding: 3.0.2
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Make CLI bootstrap rollback identity-safe

The composed CLI performs a post-publication version probe. If that probe
fails it snapshots the prior binary and calls `restore_previous_binary`, which
checks the destination inode and then uses ordinary `rename` (or `write` on
non-Unix). A replacement can arrive after the check and be clobbered by the
rollback. The lower-level artifact installer has its own no-clobber paths, but
this CLI-level recovery path remains a separate publication boundary.

Acceptance criteria:

- Post-install identity failure restores the prior executable only when the
  published destination still has the exact observed identity; a replacement
  that arrives before rollback is preserved and the outcome reports recovery
  attention.
- First-install cleanup removes only the expected published inode and never
  unlinks a replacement that appears during cleanup.
- Supported Linux and macOS paths use atomic no-replace/exchange primitives or
  fail closed when the platform cannot provide them; no overwrite-capable
  fallback remains in the CLI rollback helper.
- Isolated tests deterministically cover replacement-before-rollback,
  replacement-during-rollback, no-prior cleanup, residual temporary files, and
  the normal prior restoration path. Existing binary, checksum, and major
  policy behavior remains unchanged.

## Review origin

Fresh-context feature review found that `crates/cli/src/entrypoint.rs`
`restore_previous_binary` performs a stat-then-rename/write sequence after a
post-install identity failure. This duplicates the artifact boundary and can
overwrite an unrelated destination during a race.

## Implementation notes

- Execution capability: highest; this is a security-sensitive rollback and
  publication boundary.
- Review weight: standard (source: autopilot).
- Files changed: `crates/cli/src/entrypoint.rs`, `crates/cli/Cargo.toml`,
  `Cargo.lock`.
- Tests added: atomic exchange rollback tests for normal restoration and
  replacement preservation, plus identity-safe first-install cleanup.
- Discrepancies from design: unsupported platforms fail closed; Linux/macOS
  use native no-replace/exchange primitives and report recovery attention when
  a replacement wins the race.
- Adjacent issues parked: none.

## Review findings (2026-07-12)

- **Blocker — required rollback race coverage is absent and one race is
  misclassified**: tests cover only replacement-before-rollback and the normal
  exchange. They do not deterministically exercise replacement-during-rollback,
  residual temporary files, or the no-prior cleanup race required by this
  story. If a replacement arrives after the first exchange but before the
  displaced-identity check, the helper can return `Restored` and delete the
  private published inode while leaving the replacement in place; the caller
  then reports a clean rollback instead of recovery attention. Add a
  synchronization seam or deterministic hook, preserve the replacement, and
  classify that outcome as attention while retaining safe residuals.
- **Follow-up**: `story-skilltap-plugin-distribution-bootstrap-cli-rollback-race-coverage`.

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Substrate standard review of the identity-safe rollback boundary and
its deterministic replacement-preservation coverage. The follow-up race seam
now closes the previously identified gap. `cargo fmt --all -- --check`, full
offline workspace tests, and `cargo clippy --workspace --all-targets --offline
-- -D warnings` are green.
