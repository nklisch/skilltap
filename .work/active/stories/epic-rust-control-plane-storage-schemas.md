---
id: epic-rust-control-plane-storage-schemas
kind: story
stage: implementing
tags: [infra]
parent: epic-rust-control-plane-storage
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Define Versioned Storage Schemas

Implement strict schema-1 config, inventory, state, timestamp/apply, and managed
artifact records described by the parent design.

## Acceptance criteria

- Constructors and TOML/JSON deserialization enforce identical invariants and
  reject unknown fields/unsupported schema versions.
- Config defaults are explicit and match the foundation; missing is not encoded
  as defaults. Interval syntax is canonical and positive.
- A representative inventory containing every resource kind serializes to
  readable deterministic TOML, round-trips, rejects duplicate IDs/dangling or
  cyclic dependencies, and requires declared project roots.
- State rejects desired-policy fields, duplicate operation IDs, inconsistent
  managed ownership/provenance, and duplicate managed paths; timestamps convert
  deterministically to/from `SystemTime`.
- Artifact records use validated relative paths and exact owner/role context.
- Golden fixtures and unknown-field mutations cover all schemas; full locked
  verification passes.
