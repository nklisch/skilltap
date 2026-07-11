---
id: epic-rust-control-plane-runtime-primitives-filesystem-hardening
kind: story
stage: implementing
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
