---
id: epic-rust-control-plane-storage-integration
kind: story
stage: done
tags: [infra, testing]
parent: epic-rust-control-plane-storage
depends_on:
  - epic-rust-control-plane-storage-document-repositories
  - epic-rust-control-plane-storage-managed-artifacts
release_binding: 3.0.0
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
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

## Implementation notes

- Added a Unix integration suite that composes the typed config, inventory,
  state, and managed-artifact repositories against one isolated machine root.
  No production contract changes were required.
- First-use coverage proves missing reads and repository construction create
  nothing. Sequential writes create exactly `config.toml`, `inventory.toml`,
  `state.json`, and `managed/`; exact typed reload and a complete recursive byte
  snapshot prove immediate repeat writes and publication are idempotent.
- Corruption coverage independently replaces each owned document with malformed,
  invalid, or unsupported-schema bytes. The affected repository returns its
  typed error while the other documents and managed tree remain readable and
  unchanged; the corrupt bytes are never rewritten.
- Publication ordering coverage publishes and loads the complete multi-file
  tree before atomically adding its state reference. A concurrent observer
  accepts complete old/new state observations, retries the runtime boundary's
  deliberate path-identity rejection during rename, and requires every visible
  reference to load the exact complete tree.
- A conflicting immutable publication proves failure leaves both the prior
  state bytes and managed tree intact. Every owned-file fixture snapshot also
  asserts that authentication-like sentinel material is absent.
- The three integration tests passed three consecutive parallel runs. The full
  locked format/check/Clippy/test/rustdoc ladder passes with 141 workspace tests.
- Discrepancies from design: none. Adjacent issues parked: none.

## Review

Approved. The three real-adapter tests cover no-create first use and exact
surfaces, byte-stable reload/idempotence, independent corruption, complete-tree
publication before atomic state reference under a concurrent observer, and
conflicting publication preserving prior state/tree. Production code is
unchanged; three repeated focused runs and the full locked 141-test ladder pass.
