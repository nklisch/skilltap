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

## Review findings

Fresh-context review requested three corrections:

- bind owned-document validation and bytes to one no-follow regular-file
  descriptor so an inspect/read pathname swap cannot follow a link;
- make duplicate top-level JSON fields, especially `schema`, invalid regardless
  of duplicate key order before unsupported-version classification; and
- prevent raw runtime/I/O details from escaping through `Error::source()` as
  well as display/debug output.

The runtime filesystem port may gain the narrow descriptor-bound read primitive
required by the first correction; ordinary link-following reads remain
unchanged for callers that explicitly need them.

## Review corrections

- Added `FileSystem::read_regular_no_follow`, returning `None` for missing paths
  or bytes from one no-follow regular-file descriptor. The system adapter opens
  with `O_NOFOLLOW`, captures descriptor identity, verifies pathname identity,
  and reads from that same descriptor. Ordinary `FileSystem::read` is unchanged.
- Repository loading now calls the descriptor-bound method directly; there is
  no inspect/read pathname race. Fake tests make both `inspect` and ordinary
  `read` unreachable, while runtime tests deterministically cover missing,
  regular, symlink-follow refusal, and a post-open pathname swap.
- Replaced JSON `Value` schema lookup with a duplicate-aware raw top-level map
  visitor after syntax parsing. Duplicate fields—including both schema orders
  and duplicates alongside an unsupported version—are always `Invalid`; unique
  unsupported versions remain `UnsupportedSchema`, and syntax errors remain
  `Malformed`. Typed decoding still consumes the original bytes.
- Storage runtime failures now discard the raw runtime error after assigning
  safe document/action/path context. `Display`, `Debug`, and `Error::source()`
  expose no runtime or I/O detail; regression tests assert the empty source chain.
- Verification passed with 120 workspace tests across the full locked
  format/check/Clippy/test/rustdoc ladder. No lock, managed-artifact, or resource
lifecycle behavior was added.

Re-review confirmed the three corrections but reproduced one special-file
blocker: read-only `open` of a FIFO waits before descriptor type validation.
The owned no-follow open must include nonblocking mode and a bounded FIFO
regression must prove deterministic fail-fast behavior.
