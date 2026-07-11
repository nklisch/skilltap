---
id: epic-rust-control-plane-storage-maintainability-runtime-tests
kind: story
stage: implementing
tags: [refactor, testing]
parent: epic-rust-control-plane-storage-maintainability
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Split Runtime Filesystem Tests by Contract

Mechanically split runtime filesystem tests into metadata/no-follow,
publication/copy recovery, ownership/link safety, and configuration-locking
modules. Preserve every test name, assertion, platform guard, and fault
scenario. Compare the test list and run the full locked ladder.
