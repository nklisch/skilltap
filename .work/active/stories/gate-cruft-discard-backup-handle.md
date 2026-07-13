---
id: gate-cruft-discard-backup-handle
kind: story
stage: done
tags: [cleanup]
parent: null
depends_on: []
release_binding: 3.0.2
research_refs: []
research_origin: null
gate_origin: cruft
created: 2026-07-12
updated: 2026-07-12
---

# Discard unused backup handle directly

Preserve the fallible durable backup operation while discarding its inert
return handle immediately. The handle has no drop behavior and is not part of
replacement rollback state.

## Implementation Notes

- Kept the backup publication and its typed error mapping unchanged.
- Removed the unused local binding and explicit post-failure discard.
- Verification: `cargo test -p skilltap application::execution --offline`
  compiled the CLI and passed the selected test targets.

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none
**Rejected**: none

**Notes**: Substrate review at effective weight `standard` (caller), Fast lane for a two-line cleanup with unchanged fallible backup publication and error mapping. `ManagedArtifactHandle` has no drop behavior, so immediate result discard is behavior-preserving; replacement rollback still uses the explicit backup tree. The focused CLI target compiles and passes in a detached clean worktree.
