---
id: epic-rust-control-plane-cli-maintainability-verification
kind: story
stage: implementing
tags: [refactor, testing, infra]
parent: epic-rust-control-plane-cli-maintainability
depends_on: [epic-rust-control-plane-cli-maintainability-test-support]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Centralize Optimized Binary Verification

Add one explicit-path script that runs the smoke and compiled-binary contracts;
route CI and release runners through it without changing gate order or scope.
Replace the hardcoded compiled-test version with the workspace version
constant. Run the locked and optimized binary ladders.
