---
id: epic-rust-control-plane-storage-integration
kind: story
stage: implementing
tags: [infra, testing]
parent: epic-rust-control-plane-storage
depends_on:
  - epic-rust-control-plane-storage-document-repositories
  - epic-rust-control-plane-storage-managed-artifacts
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Verify the Machine Storage Contract

Compose all owned repositories against one isolated machine configuration root.

## Acceptance criteria

- First-use reads create nothing; sequential first writes create only the
  documented config, inventory, state, and managed surfaces.
- All repositories coexist, reload exact values, and remain byte/semantically
  idempotent on immediate repeat.
- Corrupting any one document yields its typed error without changing or
  masking the other stores or managed trees.
- Managed publication followed by atomic state reference produces no reference
  to an incomplete tree; failed publication leaves prior state/tree intact.
- Fixtures assert no authentication-like material appears in owned files and
  reader observations are complete.
- Full locked workspace and warnings-clean rustdoc pass.
