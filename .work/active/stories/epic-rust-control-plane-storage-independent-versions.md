---
id: epic-rust-control-plane-storage-independent-versions
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

# Version Storage Documents Independently

Replace the shared storage schema constant with public config, inventory, and
state version constants. Each constructor/serializer and repository codec probe
must bind to its document's expected version. Add mixed-version tests proving a
future version change in one codec cannot change validation/classification or
encoded bytes for the other two. Preserve current schema-1 golden bytes and the
full locked ladder.
