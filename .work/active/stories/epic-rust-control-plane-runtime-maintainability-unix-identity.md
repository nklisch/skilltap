---
id: epic-rust-control-plane-runtime-maintainability-unix-identity
kind: story
stage: implementing
tags: [refactor]
parent: epic-rust-control-plane-runtime-maintainability
depends_on: [epic-rust-control-plane-runtime-maintainability-sidecar-tests]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Extract Unix Identity Internals

Move private file identity, no-follow open, and descriptor/path verification
helpers to `runtime/filesystem/unix_identity.rs`, keeping cfg pairs adjacent,
call order and error mapping identical, and all public exports unchanged. Run
the adversarial filesystem suite and full locked ladder.
