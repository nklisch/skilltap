---
id: epic-rust-control-plane-storage-lock-release
kind: story
stage: review
tags: [correctness]
parent: epic-rust-control-plane-storage
depends_on: [epic-rust-control-plane-storage-document-repositories]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Release Provisional Configuration Locks

Own each successfully acquired directory/file lock in an RAII provisional
guard that explicitly unlocks on every later acquisition error, then transfers
files into the public guard only after all identity checks pass. Add a
deterministic failed-swap → immediate reacquire loop and require 30 consecutive
parallel core-suite passes plus the full locked ladder. Do not retry or hide
contention.

## Implementation notes

- Files changed: `runtime/filesystem/locking.rs` and its existing test sidecar.
- Provisional ownership: directory and file locks are consumed into a private
  `ProvisionalLock` immediately after successful nonblocking acquisition. Its
  `Drop` explicitly unlocks before closing the file on every later error path.
- Transfer: directory-before-file ordering, descriptor identity acquisition,
  path verification, callback seam, and all error mappings are unchanged. Only
  after both path identity checks pass are both provisional owners disarmed and
  their files moved into `SystemConfigurationLockGuard`.
- Public guard behavior: explicit release and RAII drop ordering remain file
  then directory. Contention remains fail-fast; no retry or error masking was
  added.
- Regression: the existing path-swap adversarial test now performs 128 cycles
  of failed post-lock identity verification followed by immediate successful
  acquire/release, then retains its two-guard exclusion scenario.
- Stress verification: 30/30 consecutive default-parallel full
  `skilltap-core` package runs passed, including the storage integration tests.
- Test inventory: all 142 live workspace test identities are unchanged.
- Verification passed: locked format, all-target workspace check,
  warnings-denied Clippy, full workspace tests, warnings-denied rustdoc, and
  diff hygiene.
- Discrepancies from design: none.
- Adjacent issues parked: none.
