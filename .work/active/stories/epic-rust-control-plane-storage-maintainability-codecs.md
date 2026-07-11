---
id: epic-rust-control-plane-storage-maintainability-codecs
kind: story
stage: review
tags: [refactor, testing]
parent: epic-rust-control-plane-storage-maintainability
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Separate Document Codecs from Repository IO

Extract private TOML/JSON codecs, schema probes, and duplicate-key validation
from repository filesystem orchestration. Add focused codec tests while
retaining repository integration coverage and exact bytes, classification,
ordering, public identities, and test list. Run the full locked ladder.

## Implementation notes

- Files changed: `crates/core/src/storage/repository.rs`,
  `crates/core/src/storage/repository/codec.rs`,
  `crates/core/src/storage/repository/tests.rs`, and
  `crates/core/src/storage/repository/tests/codec.rs`.
- Tests added: none; four existing codec-focused tests moved through a lexical
  include so every fully-qualified test identity and assertion remains
  unchanged.
- The private TOML/JSON codecs, schema probes, and duplicate-key validation now
  live in `repository/codec.rs`; public repository declarations and filesystem
  orchestration remain in `repository.rs`.
- Pre/post `cargo test -p skilltap-core -- --list` output is byte-identical: 146
  core unit tests, three core integration tests, and unchanged doctest entries.
- Verification: focused repository tests plus the full locked format,
  all-target check, warnings-denied Clippy, workspace test/doctest, and
  warnings-denied rustdoc ladder pass (150 workspace tests).
- Discrepancies from design: none.
- Adjacent issues parked: none.
