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

## Implementation notes

- Added `skilltap_core::storage` with strict schema-1 `ConfigDocument`,
  `InventoryDocument`, and `StateDocument` types plus validated update
  intervals, nanosecond Unix timestamps, per-resource apply records, harness
  state, resource state, and owner/role-bound managed artifact records.
- The complete inventory spike succeeded with the existing `DesiredResource`
  serde contract. Storage adds only a deterministic list/map document wire;
  desired targets, sources, update intent, components, accepted consequences,
  and dependencies remain domain-owned single sources of truth.
- Constructors and deserialization share validation paths for versions,
  canonical positive intervals, desired dependency graphs, declared project
  roots, state identity/path uniqueness, provenance/ownership/artifact-role
  consistency, duplicate operation results, and timestamp range/precision.
- Added strict TOML config/inventory and JSON state golden fixtures. Negative
  mutations cover unknown document and nested fields, unsupported versions,
  desired-policy leakage into state, duplicates, dangling/cyclic graphs,
  undeclared projects, invalid timestamps, and invalid artifact context.
- Added maintained `toml` 1.1.2 as a workspace dependency. No repository,
  filesystem mutation, observation, planning, or lifecycle behavior was added.
- Files changed: `Cargo.toml`, `Cargo.lock`, `crates/core/Cargo.toml`,
  `crates/core/src/lib.rs`, and `crates/core/src/storage/`.
- Tests added: 8 storage schema/golden/negative-contract tests. Verification
  passed with 107 workspace tests: `cargo fmt --all -- --check`,
  `cargo check --locked --workspace --all-targets`,
  `cargo clippy --locked --workspace --all-targets -- -D warnings`,
  `cargo test --locked --workspace`, and
  `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`.
- Discrepancies from design: none.
- Adjacent issues parked: none.

## Review finding

Fresh-context review requested one correction: `HarnessPolicy.binary` used
opaque `NativeId`, allowing undocumented relative paths such as
`relative/path/codex`. The config boundary must accept either one normal PATH
executable name or a validated absolute path, with the same constructor and
TOML behavior. All other schema surfaces were approved.
