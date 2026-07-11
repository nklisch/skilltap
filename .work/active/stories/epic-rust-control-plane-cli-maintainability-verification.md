---
id: epic-rust-control-plane-cli-maintainability-verification
kind: story
stage: review
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

## Implementation notes

- Files changed: `scripts/verify-compiled-binary.sh`,
  `.github/workflows/ci.yml`, `.github/workflows/release.yml`, and
  `crates/cli/tests/compiled_binary.rs`.
- Verification contract: the new wrapper requires exactly one binary path,
  runs the existing smoke check first, then runs the locked compiled CLI suite
  against that exact executable. CI calls the wrapper once after its optimized
  build. Every release matrix runner calls it at the existing pre-artifact
  compiled-test gate, while the post-strip/sign artifact smoke remains in its
  original position.
- Version contract: the compiled suite now derives its expected version from
  `skilltap_core::VERSION`; no release literal remains in the Rust assertion.
- Verification: locked format, check, Clippy with warnings denied, workspace
  tests (192 tests), rustdoc, optimized build, wrapper smoke, and all six
  compiled contracts against the optimized binary pass. The local Cargo target
  override places the optimized artifact at `/storage/cargo-target/release/skilltap`,
  which also verifies the wrapper's absolute explicit-path behavior.
- Discrepancies from design: none.
- Adjacent issues parked: none.
