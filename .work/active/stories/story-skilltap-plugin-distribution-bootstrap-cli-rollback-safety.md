---
id: story-skilltap-plugin-distribution-bootstrap-cli-rollback-safety
kind: story
stage: implementing
tags: [infra, security, testing]
parent: epic-skilltap-plugin-distribution-bootstrap
depends_on: [story-skilltap-plugin-distribution-bootstrap-command]
release_binding: null
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
