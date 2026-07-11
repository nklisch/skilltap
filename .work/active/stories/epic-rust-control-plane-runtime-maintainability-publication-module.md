---
id: epic-rust-control-plane-runtime-maintainability-publication-module
kind: story
stage: implementing
tags: [refactor]
parent: epic-rust-control-plane-runtime-maintainability
depends_on: [epic-rust-control-plane-runtime-maintainability-unix-identity]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Extract Publication State Machine

Move the private recoverable-copy staging, no-clobber publication, cleanup,
rollback, and residual construction machinery to
`runtime/filesystem/publication.rs`. Preserve the public `FileSystem` method,
all injected test seams, exact error precedence/state, and exports. Run the
complete recovery matrix and full locked ladder.
