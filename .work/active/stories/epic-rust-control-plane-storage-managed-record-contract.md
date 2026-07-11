---
id: epic-rust-control-plane-storage-managed-record-contract
kind: story
stage: done
tags: [correctness]
parent: epic-rust-control-plane-storage
depends_on: [epic-rust-control-plane-storage-managed-artifacts]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Unify Managed Artifact Record Validation

Create one storage-owned canonical record contract used by constructors, serde,
state validation, and the managed repository. Non-backup roles derive/validate
the bounded owner-hash/role/fingerprint leaf; backup roles validate generated
owner-bound leaves and forbid fingerprints. Make arbitrary invalid construction
fallible, update the state golden to a loadable canonical record, and add a real
state round-trip → repository load test. No duplicated validator/path function.
Run the full locked ladder.

## Implementation notes

- Added one storage-owned `managed_record` module as the canonical authority for
  `ArtifactRole`, `ManagedArtifactRecord`, owner hashing, role components,
  artifact-path derivation, backup-leaf generation, and record validation.
- `ManagedArtifactRecord::new` is now fallible. `for_artifact` derives the
  bounded owner-hash/role/fingerprint leaf and rejects backup roles;
  `for_backup` generates the owner-bound process/sequence leaf, forbids a zero
  process, and records no fingerprint. Custom serde passes through the same
  constructor and rejects arbitrary paths, missing/extra fingerprints, unknown
  fields, and noncanonical backup leaves.
- `ResourceState` and managed repository load/remove call the record's shared
  owner validator. Repository publish/backup use the canonical factories; the
  former repository-local path, role-component, owner-hash, and validator
  functions were removed.
- Updated all record callers and the state golden to use the canonical derived
  path. Tests cover maximum-length owner bounds, artifact/backup serde round
  trips, arbitrary invalid construction, invalid backup fingerprints, owner
  mismatch, and provenance/role mismatch.
- Extended the machine-storage integration test to serialize and deserialize a
  real referenced `StateDocument`, recover its validated record, and load the
  exact previously published multi-file tree through the repository.
- Verification passed: five consecutive eight-thread core runs, locked format,
  all-target check, warnings-denied Clippy, 144 workspace tests, and
  warnings-denied rustdoc.
- Discrepancies from design: none. Adjacent issues parked: none.

## Review

Approved. One module now owns owner hashing, role components, canonical artifact
and backup leaves, fallible construction, serde, and owner validation. State and
repository consume that same contract; the golden is loadable and a real
StateDocument JSON round-trip resolves the published tree. Five parallel runs
and the locked 144-test ladder pass.
