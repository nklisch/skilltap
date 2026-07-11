---
id: epic-rust-control-plane-storage-document-repositories
kind: story
stage: review
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

## Implementation notes

- Added explicit `ConfigRepository`, `InventoryRepository`, and
  `StateRepository` ports with corresponding filesystem adapters. Each exposes
  typed `load` and `replace` operations and `DocumentState::{Missing, Present}`.
- Added one private generic document engine and private TOML/JSON codecs.
  Loading inspects without creating, refuses non-regular owned paths, separates
  syntax decode from schema validation, and preserves document/action/path
  context without rendering contents or parser/runtime details.
- Replacement deterministically encodes, decodes the complete encoded value
  again before mutation, creates the configuration root only after successful
  validation, and delegates exactly one publication to `FileSystem::atomic_write`.
  No configuration lock or lifecycle behavior is acquired implicitly.
- TOML preflight uses a table only for schema classification and then decodes
  the original text; JSON likewise decodes the original bytes after preflight,
  preserving strict duplicate-field rejection.
- Added nine fake/system tests for missing first use, typed round trips,
  byte-identical repeated replacement, malformed/unknown/version failures,
  safe errors, non-regular files, create/write failures, preserved old bytes,
  and concurrent old-or-new system reads.
- Files changed: `crates/core/src/runtime/filesystem.rs`,
  `crates/core/src/storage/mod.rs`, and
  `crates/core/src/storage/repository.rs` plus its sidecar tests.
- Verification passed with 118 workspace tests: `cargo fmt --all -- --check`,
  `cargo check --locked --workspace --all-targets`,
  `cargo clippy --locked --workspace --all-targets -- -D warnings`,
  `cargo test --locked --workspace`, and
  `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`.
- Discrepancies from design: none.
- Adjacent issues parked: none.
