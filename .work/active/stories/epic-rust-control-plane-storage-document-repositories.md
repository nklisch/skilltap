---
id: epic-rust-control-plane-storage-document-repositories
kind: story
stage: implementing
tags: [infra]
parent: epic-rust-control-plane-storage
depends_on: [epic-rust-control-plane-storage-schemas]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Implement Owned Document Repositories

Add explicit config, inventory, and state repository ports and filesystem
adapters with a private shared codec/publication engine.

## Acceptance criteria

- `load` distinguishes `Missing` and `Present`; missing reads do not create the
  root or file.
- The first successful replacement creates the configuration root, validates
  the complete value again, encodes deterministically, and delegates one atomic
  file publication to the runtime port.
- Malformed syntax, unknown fields, invalid values, and unsupported schema
  versions retain document/action/path context and never trigger a rewrite.
- Config/inventory use TOML and state uses JSON; repeat replacement is
  byte-identical/idempotent.
- Public repositories remain typed while codec machinery is private; no lock is
  acquired implicitly.
- Fake-port and isolated filesystem tests cover old-or-new reads and failures;
  full locked verification passes.
