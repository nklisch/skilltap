---
id: epic-rust-control-plane-storage-managed-record-contract
kind: story
stage: implementing
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
