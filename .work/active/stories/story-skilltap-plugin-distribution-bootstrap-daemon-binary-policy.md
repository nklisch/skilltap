---
id: story-skilltap-plugin-distribution-bootstrap-daemon-binary-policy
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

# Apply the bootstrap binary policy in the optional daemon

The bootstrap feature persists `BinaryUpdatePolicy`, but the daemon update
cycle currently reads only the ordinary resource update policy and processes
plugins and Git-backed skills. Consequently the documented default
same-major self-update, opt-out mode, and explicit major opt-in are inert.
Connect the daemon's existing update-cycle application service to the verified
bootstrap release/installation boundary without creating a second updater.

Acceptance criteria:

- `bootstrap.mode = off` performs no release resolution or binary mutation and
  reports a deterministic disabled result.
- `bootstrap.mode = check` resolves the current release and records an
  available compatible update without publishing a binary.
- `bootstrap.mode = apply-safe` applies only a verified install or newer
  same-major update; a newer major remains pending unless the persisted,
  explicit `bootstrap.allow_major` opt-in is set.
- Binary update failures, blocked majors, unknown installed versions, drift,
  and partial harness setup remain visible as attention/pending daemon results;
  the daemon never acknowledges partial operations or overwrites an unrelated
  destination.
- The daemon reuses the same bounded resolver, checksum verification, atomic
  installer, lock, and post-install identity checks as foreground
  `skilltap bootstrap`; no shell/network/cache write path is duplicated.
- Isolated daemon tests cover off/check/apply-safe, same-major update,
  blocked/opted-in major, missing binary, failed verification, idempotent
  repeat, and result persistence. Existing plugin/skill update behavior and
  user-service definitions remain unchanged.

## Review origin

Fresh-context feature review found that the stored binary policy is never
consumed by `StatusApplication::execute_daemon_cycle`; only resource updates
are scheduled. This leaves the feature's explicit unattended update contract
and `docs/SPEC.md` daemon promise unimplemented.

## Implementation notes

- Execution capability: highest; this crosses the daemon lifecycle and
  security-sensitive release publication boundary.
- Review weight: standard (source: autopilot).
- Adjacent issue: CLI post-install rollback should be audited for a
  no-clobber race before release review.
