---
id: gate-cruft-discard-backup-handle
kind: story
stage: review
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
