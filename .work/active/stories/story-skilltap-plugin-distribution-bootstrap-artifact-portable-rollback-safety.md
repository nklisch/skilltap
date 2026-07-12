---
id: story-skilltap-plugin-distribution-bootstrap-artifact-portable-rollback-safety
kind: story
stage: done
tags: [infra, security, testing]
parent: epic-skilltap-plugin-distribution-bootstrap
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Make artifact publication and rollback safe on every supported platform

Close the remaining artifact-boundary gaps found while reviewing
`story-skilltap-plugin-distribution-bootstrap-artifact-boundary-hardening`.
The Linux `renameat2` path is identity-safe, but the non-Linux fallbacks use
ordinary `fs::rename`, which can overwrite a destination that appears after
the initial observation. The no-prior rollback path also checks identity and
then unlinks by pathname, allowing a replacement race to remove an unrelated
file.

Acceptance criteria:

- macOS and Linux publication paths never overwrite a destination that appears
  or changes after observation; unsupported atomic primitives fail closed with
  `DestinationChanged`/`InstallFailed` rather than using overwrite-capable
  fallback behavior.
- Rollback with no prior payload removes only the expected published identity,
  using an atomic identity-safe operation; a replacement after observation is
  preserved.
- Existing exchange rollback behavior remains replacement-safe on both
  supported platforms, with cleanup on every failed/interrupted path.
- Isolated test-support coverage exercises the redirect-hop rejection,
  oversized/symlink payloads, checksum/permission/interruption cleanup,
  replacement races, and post-publish rollback preservation required by the
  bootstrap artifact contract. Tests remain offline and use temporary roots.

## Review origin

Standard fresh-context review of `614a29c`/`a1610a4` found an overwrite-capable
non-Linux fallback, a stat-then-unlink rollback race, and missing fixture
coverage for the full acceptance matrix.

## Implementation notes

- Execution capability: highest; this is a cross-platform security-sensitive
  filesystem boundary.
- Review weight: standard (autopilot caller policy).
- Files changed: `crates/core/src/runtime/artifact.rs`, `crates/core/tests/bootstrap_integration.rs`.
- Tests added: offline redirect-hop attestation, no-prior rollback replacement preservation, and non-executable payload preservation.
- Discrepancies from design: Linux uses `renameat2`, macOS uses `renameatx_np` exchange plus hard-link no-replace publication; unsupported platforms fail closed instead of falling back to overwrite-capable rename.
- Adjacent issues parked: none.

## Review (2026-07-12)

**Verdict**: Request changes

**Blockers**: `remove_published_if_identity` exchanges the marker into the
destination, verifies the displaced inode at the private marker, then
unconditionally removes the destination pathname (`crates/core/src/runtime/artifact.rs:728-737`).
An unrelated replacement can win that pathname after the exchange and before
the remove, so no-prior rollback can still delete the replacement. Revalidate
the destination marker identity immediately before removal (or remove only via
an atomic identity-safe exchange) and add a deterministic post-exchange race
fixture (this item)

**Important**: none

**Nits**: none

**Notes**: Standard substrate review of `9c87afb` at highest implementation
capability with standard review weight. Redirect-hop validation now rejects an
unattested intermediate host, macOS uses hard-link no-replace publication and
`renameatx_np` exchange, unsupported platforms fail closed, and the existing
tests pass. The added no-prior test replaces the destination before calling the
helper; it does not cover a replacement between the exchange and the unlink,
which is the remaining correctness race. Keep the item at `stage: implementing`
until that identity window is closed.

## Review (2026-07-12, current rollback)

**Verdict**: Request changes

**Blockers**: `d78b343` makes `remove_published_if_identity_with` a no-op to
avoid the unlink race. Consequently a no-prior `restore_destination` leaves
the newly published (possibly invalid) binary at the destination after a
parent-sync failure; this does not satisfy the required no-prior rollback or
cleanup semantics (this item)

**Important**: the prior hard-link-plus-unlink macOS `rename_noreplace`
fallback remains a path race, and the current no-prior fixtures no longer
assert that the destination is removed when no replacement exists (this item)

**Nits**: none

**Notes**: Standard substrate review of `d78b343` at highest implementation
capability with standard review weight. The portable publication paths fail
closed and existing exchange rollback remains replacement-preserving, but
silently retaining a failed first publication is an observable partial install
and leaves the rollback acceptance unmet. Keep the story at
`stage: implementing` until an atomic no-replace move-to-private-marker
rollback (or an explicitly documented equivalent) removes only the expected
inode while preserving replacements.

## Review (2026-07-12, atomic cleanup)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Standard substrate review of `053cd1a` plus the cleanup regression
test in `ac6dfbb` at highest implementation capability. No-prior rollback now
atomically moves the observed destination to a private no-replace marker,
removes only the expected identity, and restores unrelated replacements
without unlinking a raced pathname. macOS uses `renameatx_np(RENAME_EXCL)`
instead of the prior hard-link/unlink fallback; unsupported platforms remain
fail-closed. Redirect-hop, publication, replacement, no-prior cleanup, and
bootstrap integration tests pass. Advancing the story to `stage: done`.
