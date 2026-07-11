---
id: epic-rust-control-plane-storage-removal-residuals
kind: story
stage: implementing
tags: [correctness]
parent: epic-rust-control-plane-storage
depends_on: [epic-rust-control-plane-storage-managed-artifacts]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Report Managed Removal Residuals

Extend runtime and storage errors for partial tree removal with expected and
observed identity, path presence, content progress (`intact`, `partial`,
`empty`, or `unknown`), and parent-directory sync state. Recursive deletion must
track whether any owned entry was removed; top unlink and parent sync are
reported independently. Add injected tests for failure before/after partial
content removal, identity/path replacement, empty-but-present, and
unlink-success/sync-failure. State/reference callers must be able to choose
re-observation without guessing. Preserve safe error rendering and run the full
locked ladder.
