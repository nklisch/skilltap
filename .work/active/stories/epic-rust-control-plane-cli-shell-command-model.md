---
id: epic-rust-control-plane-cli-shell-command-model
kind: story
stage: done
tags: [cli]
parent: epic-rust-control-plane-cli-shell
depends_on: []
release_binding: 3.0.0
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Implement the V3 Command Model

Implement the complete Clap command tree and reusable scope, target, selector,
acknowledgment, and output groups. Normalize parsing without process exits and
convert values immediately into validated core request types. Cover exact
optional-project, conflict, flag-relevance, help/version, and representative
nested-command forms with tests. Run the locked ladder.

## Implementation notes

- Added the complete v3 Clap tree for harness, adoption, status, planning,
  synchronization, marketplace, plugin, standalone skill, instruction, and
  daemon commands.
- Added reusable scope, target, selection, acknowledgment, output, and scoped
  target groups while attaching each group only where its flags are meaningful.
- Preserved all three scope forms at the parse boundary: global by omission,
  current project via valueless `--project`, and another project via
  `--project <path>`. `--project` never consumes a following option and
  conflicts with `--all-scopes` in either order.
- Converted target, harness, native identifier, source locator, Git revision,
  relative skill path, harness binary, instruction mode, and daemon interval
  inputs directly into validated `skilltap-core` types. Project paths remain
  paths until the runtime scope resolver canonicalizes them against the working
  directory and Git root.
- Enforced the exact `<plugin>@<marketplace>` install form, repeatable selectors,
  supported target values, non-UTF-8 rejection, and validation of empty,
  control-character, relative artifact path, interval, and executable inputs.
- Added fifteen parser contract tests covering the full command tree,
  help/version as returned Clap outcomes rather than process exits, missing
  subcommand normalization input, optional-project forms, conflicts, relevant
  and irrelevant flags, validated conversions, selectors, representative nested
  commands, and malformed values.
- Kept rendering, exit mapping, application dispatch, and native/domain business
  logic outside this unit. The binary contains only a temporary `try_parse`
  placeholder for the composition story to replace.
- Verification passed: `cargo fmt --all -- --check`, workspace locked check and
  Clippy with warnings denied, workspace locked tests (172 tests), rustdoc with
  warnings denied, release build, compiled-binary help/version smoke, and an
  explicit no-subcommand exit-1/usage assertion.
- Files changed: `crates/cli/src/command.rs`,
  `crates/cli/src/command/tests.rs`, `crates/cli/src/lib.rs`, and
  `crates/cli/src/main.rs`.
- Discrepancies from design: none. Adjacent issues parked: none.

## Review correction

- Moved the unchanged parser contract tests from the 847-line command module to
  the private `command/tests.rs` sidecar. Production grammar and conversion code
  now occupy 499 lines, while test module paths and test identities remain
  unchanged.
- Added compile-time slice assertions in the existing sync test proving that
  both repeatable selection collections contain validated core `NativeId`
  values. The CLI continues to treat selector spelling as opaque because the
  foundation documents define no further selector grammar.

## Review

Approved after the test-sidecar correction. The complete documented grammar,
scope/target/flag relevance, validated values, help/version/no-command behavior,
and compiled smoke contract pass; parser test identities remain unchanged.
