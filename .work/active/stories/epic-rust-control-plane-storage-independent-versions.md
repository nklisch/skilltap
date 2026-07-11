---
id: epic-rust-control-plane-storage-independent-versions
kind: story
stage: done
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

## Implementation notes

- Replaced the shared schema constant with public `CONFIG_SCHEMA_VERSION`,
  `INVENTORY_SCHEMA_VERSION`, and `STATE_SCHEMA_VERSION` constants. Each
  document constructor, `schema()` accessor, and serialization wire now uses
  only its own constant.
- Parameterized the private generic TOML and JSON codecs with an expected schema
  version. Each concrete file repository supplies its document-specific
  constant, so generic preflight classification no longer couples formats.
- Updated schema constructors and integration fixtures to name the relevant
  document version explicitly; no compatibility alias remains.
- Added a mixed-version regression that advances only the config codec's
  hypothetical expectation. Config schema-1 bytes classify as unsupported,
  while inventory and state still decode exactly and all encoded bytes remain
  unchanged. Existing schema-1 config, inventory, and state goldens still pass.
- Files changed: storage module, config/inventory/state schemas, repository and
  tests, schema tests, machine-storage integration test, and this item.
- Verification passed: locked format, all-target check, warnings-denied Clippy,
  142 workspace tests, and warnings-denied rustdoc.
- Discrepancies from design: none. Adjacent issues parked: none.

## Review

Approved. Each document now owns its public version constant and binds both
schema serde and repository probing to it. The mixed-version regression advances
only config's expected codec while inventory/state validation and bytes remain
exact; all schema-1 goldens and the locked 142-test ladder pass.
