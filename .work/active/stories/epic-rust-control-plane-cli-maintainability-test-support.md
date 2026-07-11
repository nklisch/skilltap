---
id: epic-rust-control-plane-cli-maintainability-test-support
kind: story
stage: done
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

## Implementation notes

- Files changed: `crates/test-support/src/lib.rs`,
  `crates/cli/tests/compiled_binary.rs`,
  `crates/cli/src/application/tests.rs`, and removal of
  `crates/cli/tests/bare_help.rs`.
- Tests added: isolated-machine directory construction and captured-output UTF-8
  helpers in `skilltap-test-support`; every existing application and compiled
  test identity remains except the declared redundant bare-help integration
  test.
- Preserved contracts: compiled-binary overrides remain absolute or resolve
  relative to the invoking working directory; child processes retain isolated
  `HOME`, `XDG_CONFIG_HOME`, default or explicit current directory, removed
  `SKILLTAP_HOME`, raw output bytes, and exit status. The compiled suite retains
  the bare invocation exit, empty stdout, normalized error, root usage, and
  no-color assertions.
- Verification: locked format, check, Clippy with warnings denied, workspace
  tests (192 tests), rustdoc, optimized binary build/smoke, and the six-test
  compiled contract against the optimized binary all pass.
- Discrepancies from design: none.
- Adjacent issues parked: none.

## Review

Approved. Test support now owns domain-agnostic process/environment isolation,
binary override resolution, and captured bytes; all compiled/application
contracts remain, with only the explicitly redundant bare test removed.
