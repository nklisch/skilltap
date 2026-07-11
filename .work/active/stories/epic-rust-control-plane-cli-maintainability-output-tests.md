---
id: epic-rust-control-plane-cli-maintainability-output-tests
kind: story
stage: implementing
tags: [refactor, testing]
parent: epic-rust-control-plane-cli-maintainability
depends_on: []
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Move Output Tests to a Sidecar

Move the unchanged inline output test module to `output/tests.rs`, preserving
all fully qualified test identities, bodies, assertions, and test-list order.
Run the full locked ladder.
