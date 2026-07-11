---
id: epic-rust-control-plane-cli-maintainability
kind: feature
stage: implementing
tags: [refactor, testing]
parent: epic-rust-control-plane
depends_on: [epic-rust-control-plane-cli-shell]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# CLI Maintainability

## Brief

Reduce structural and test-infrastructure pressure revealed by the completed
CLI shell without changing grammar, output bytes, channels, exits, storage or
filesystem effects, public identities, test identities, or release behavior.

## Evidence

- `crates/cli/tests/compiled_binary.rs` locally owns an isolated-machine and
  compiled-runner framework assigned by architecture to
  `skilltap-test-support`; application tests duplicate temporary-root cleanup,
  and `bare_help.rs` duplicates a compiled assertion without honoring the
  release-binary override.
- `StatusApplication::execute` is a 122-line orchestration combining document
  loading/classification, scope resolution, target resolution, and projection.
- `output.rs` mixes 210 production lines with 167 inline test lines while the
  other CLI modules use private sidecars.
- CI/release repeat the compiled verification command, and one compiled test
  hardcodes `3.0.0` instead of using the workspace version contract.

The cadence scan leaves the declarative 499-line Clap grammar, exhaustive
79-line transitional dispatch, typed outcome builders, and independent
parser-vs-binary command-tree coverage unchanged.

## Design

This is a behavior-preserving decomposition. Establish a reusable compiled CLI
fixture in `skilltap-test-support`, then route CLI integration tests and
application temporary roots through that ownership point. The fixture remains
domain-agnostic: it owns process/environment isolation and captured bytes, not
CLI assertions or JSON semantics. The redundant dedicated bare-help test is
removed only after the compiled contract retains the same assertion.

Separately split `StatusApplication::execute` into private typed phases for
owned-document loading, scope resolution, target resolution, and projection,
preserving the exact early-return and output ordering. Move output tests to a
normal private sidecar. Finally centralize the release-binary verification
invocation in one script and derive expected version text from the workspace
constant in Rust tests; CI and release retain their distinct gates while
calling the same verification contract.

## Pre-mortem

- **Fixture extraction changes environment isolation.** Preserve every removed
  environment variable, current-directory choice, binary override rule, and
  no-create assertion byte-for-byte.
- **Status phases reorder visible output/errors.** Capture representative
  plain/JSON bytes before moving code and preserve document/scope/target early
  return order exactly.
- **Removing a duplicate test loses a boundary.** Keep the bare binary
  assertion in the compiled contract before deleting `bare_help.rs`.
- **A wrapper script hides release failures.** Accept an explicit binary path,
  fail fast, and execute both smoke and compiled suites with locked Cargo.

## Implementation units

1. `epic-rust-control-plane-cli-maintainability-test-support` — centralize
   isolated machine, binary routing, process execution, output helpers, and
   temporary roots in test support; remove the redundant bare test — depends
   on `[]`.
2. `epic-rust-control-plane-cli-maintainability-status-phases` — extract typed
   private status load/scope/target/projection phases without output drift —
   depends on `[]`.
3. `epic-rust-control-plane-cli-maintainability-output-tests` — move unchanged
   output tests to a private sidecar — depends on `[]`.
4. `epic-rust-control-plane-cli-maintainability-verification` — centralize the
   optimized binary verification invocation and eliminate the hardcoded Rust
   version literal — depends on
   `[epic-rust-control-plane-cli-maintainability-test-support]`.

## Acceptance criteria

- Public CLI grammar/API identities, plain/JSON bytes, channels, exit codes,
  error ordering, storage reads, and filesystem effects are unchanged.
- Test identities are unchanged except the explicitly redundant bare-help
  integration test removal; its assertions remain in the compiled suite.
- Test support owns compiled binary/environment isolation without depending on
  core domain or CLI output semantics.
- Status phases preserve the exact observable early-return/projection order.
- CI and all release runners exercise the same explicit optimized binary
  verification contract.
- Full locked format/check/Clippy/test/rustdoc plus optimized binary ladder
  passes.
