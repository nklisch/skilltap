---
id: epic-rust-control-plane-cli-shell-bare-help
kind: story
stage: done
tags: [bug, cli]
parent: epic-rust-control-plane-cli-shell
depends_on: [epic-rust-control-plane-cli-shell-composition]
release_binding: 3.0.0
gate_origin: tests
created: 2026-07-11
updated: 2026-07-12
---

# Restore Bare CLI Help

## Reproduction

The compiled binary invoked as bare `skilltap` exits `1` and emits the stable
`missing_command` outcome, but omits the concise root usage/help required by
`docs/UX.md` and the CLI-shell acceptance contract.

## Fix contract

Preserve exit `1`, safe normalized error semantics, and one-document JSON.
For plain bare invocation only, include concise root help/usage with the error;
do not expose raw invalid arguments or change other parse failures. Add
in-process and compiled-binary regression coverage and run the locked ladder.

## Implementation notes

- Files changed: `crates/cli/src/entrypoint.rs`,
  `crates/cli/src/entrypoint/tests.rs`, and the dedicated
  `crates/cli/tests/bare_help.rs` compiled-binary regression.
- Tests added: the in-process entrypoint now requires the normalized
  `missing_command` error and concise root usage; a JSON-requested root parse
  failure remains exactly one sanitized document; the compiled binary verifies
  exit `1`, stderr routing, normalized code, and root usage.
- Implementation: direct-read only. The missing-subcommand parse branch appends
  Clap's generated root usage only for plain output, after rendering the shared
  typed error outcome. Other parse kinds and JSON rendering remain unchanged.
- Verification: `cargo fmt --all -- --check`,
  `cargo check --locked --workspace --all-targets`,
  `cargo clippy --locked --workspace --all-targets -- -D warnings`,
  `cargo test --locked --workspace` (191 tests),
  `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`,
  `cargo build --locked --release -p skilltap`, and the bare release-binary
  regression all pass.
- Discrepancies from design: none.
- Adjacent issues parked: none.

## Review

Approved. Plain bare invocation now preserves the safe normalized error while
adding generated root usage on stderr with exit `1`; JSON and all other parse
failures remain single-document/sanitized and unchanged.
