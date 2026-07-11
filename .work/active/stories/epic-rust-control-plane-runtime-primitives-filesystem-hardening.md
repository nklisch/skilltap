---
id: epic-rust-control-plane-runtime-primitives-filesystem-hardening
kind: story
stage: done
tags: [infra, correctness]
parent: epic-rust-control-plane-runtime-primitives
depends_on: [epic-rust-control-plane-runtime-primitives-filesystem-lock]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Harden Filesystem Publication and Opens

## Brief

Close the fresh-review filesystem races while preserving the existing runtime
port: backups publish complete and no-clobber, ownership-sensitive opens do not
follow links, lock acquisition verifies pathname identity, and symlink targets
have one canonical parent-prefix form.

## Acceptance criteria

- Recoverable copies stage and sync complete bytes before an atomic no-clobber
  publication; readers never see partial contents, existing destinations never
  change, and every pre-publication failure cleans its temporary.
- A reported failure after attempted publication removes only the destination
  inode created by that operation where safely possible and returns precise
  partial/cleanup context if rollback cannot be proven.
- Backup sources and lock files are opened without following symlinks and the
  opened descriptor identity is verified against the path before use/lock;
  deterministic adversarial tests cover swaps at the former check/use seams.
- Lock contention remains fail-fast and two successful guards cannot arise for
  two inodes through a pathname swap during acquisition.
- `RelativeSymlinkTarget` permits leading `../` segments followed by normal
  components, but rejects `dir/../AGENTS.md`, absolute, current-dir, redundant,
  or parent-only forms.
- Full locked format/check/Clippy/test/rustdoc ladder passes.

## Implementation notes

- Files changed: hardened `crates/core/src/runtime/filesystem.rs`; extended typed runtime errors and
  exports in `runtime/error.rs` and `runtime/mod.rs`; added the maintained `libc` Unix constants
  dependency in workspace/core manifests and `Cargo.lock`.
- Publication: recoverable copies now read from a no-follow, identity-verified descriptor into a
  same-directory temporary, sync it completely, and atomically publish with no-clobber `hard_link`.
  Failures remove only matching `(device, inode)` paths and distinguish cleaned failure,
  `TemporaryLeft`, and `RollbackUnproven` outcomes.
- Locking: acquisition no-follow opens and identity-verifies both the configuration directory and
  lock file, holds an exclusive parent-directory lock with the file lock, and therefore remains
  fail-fast even when the lock pathname is swapped to another inode.
- Tests added: atomic backup visibility/no-clobber, three injected cleanup/rollback outcomes,
  deterministic source-swap rejection, lock acquisition/path-swap rejection, two-inode contention,
  lock symlink refusal, and leading-parent-only symlink target normalization.
- Discrepancies from design: none. No unsafe code was required; safe Unix `OpenOptionsExt` uses
  `O_NOFOLLOW`, `O_CLOEXEC`, and `O_DIRECTORY`, while standard metadata supplies descriptor/path
  identity on both Linux and macOS.
- Verification: locked format, all-target check, warnings-denied Clippy, workspace tests (90 core
  tests), and warnings-denied rustdoc pass.
- Adjacent issues parked: none.

## Review

Approved. Recoverable copies now stage a synced inode and publish it with an
atomic no-clobber hard link; all failure phases either prove cleanup or report
the exact remaining partial state. No-follow descriptor opens and device/inode
verification close source and lock check/use seams, while the held parent lock
keeps cooperating skilltap writers on one namespace. Link targets accept only
a canonical leading parent prefix. Twelve adversarial filesystem tests and
warnings-denied workspace Clippy pass on review.
