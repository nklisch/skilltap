---
id: epic-rust-control-plane-storage-lock-release
kind: story
stage: implementing
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
