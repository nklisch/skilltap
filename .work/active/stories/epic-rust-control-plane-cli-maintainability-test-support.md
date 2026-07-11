---
id: epic-rust-control-plane-cli-maintainability-test-support
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

# Centralize CLI Test Environments

Move domain-agnostic isolated machine, compiled binary override/execution, and
captured-output helpers into `skilltap-test-support`; route compiled CLI and
application temporary roots through it. Preserve environment/current-directory
semantics and all assertions. Remove the redundant `bare_help.rs` test only
after its compiled-suite assertion remains. Run the locked and binary ladders.
