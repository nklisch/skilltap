---
id: epic-rust-control-plane-runtime-primitives-command-clock
kind: story
stage: done
tags: [infra]
parent: epic-rust-control-plane-runtime-primitives
depends_on: [epic-rust-control-plane-runtime-primitives-errors-paths]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Direct Command and Clock Ports

## Brief

Provide synchronous time and native-process boundaries without shell parsing,
terminal output, or ambient-secret capture.

## Acceptance criteria

- `CommandRunner` accepts an executable plus an exact argument vector and
  optional canonical working directory; the system adapter never invokes a
  shell.
- Results retain exit status, stdout, stderr, and elapsed duration for both zero
  and non-zero exits; spawn/wait failures are typed separately from command
  exits.
- Generic errors and debug-safe evidence do not include argv or inherited
  environment values. The runner itself writes nothing to stdout/stderr.
- `Clock` has deterministic fake and system implementations suitable for
  update/observation timestamps without an async runtime.
- Isolated executable fixtures prove argument preservation, working directory,
  output capture, non-zero behavior, and safe error text.
- Locked formatting, all-target check, Clippy, tests, and rustdoc pass.

## Implementation notes

- Files changed: new `crates/core/src/runtime/{command,clock}.rs`, runtime exports in `runtime/mod.rs`, and isolated Rust fixture `crates/core/tests/fixtures/command_fixture.rs`.
- Public surface: `CommandRequest`, `CommandOutput`, `CommandRunner`, `SystemCommandRunner`, `Clock`, `SystemClock`, and settable deterministic `FakeClock`.
- Tests added: 5 unit tests covering exact shell-metacharacter/space argument preservation, canonical working directory, captured stdout/stderr, non-zero exits as ordinary results, spawn-error classification, redacted request/output/error debug forms, deterministic fake time, and synchronous system time.
- Process behavior: the adapter passes executable and `OsString` arguments directly to `std::process::Command`, nulls stdin, pipes stdout/stderr, distinguishes `Spawn` from `Wait`, never stores arguments or environment in errors, and does not treat non-zero status as a runner failure.
- Fixture isolation: tests compile and execute a dedicated Rust fixture directly; the production runner never invokes a shell.
- Discrepancies from design: none.
- Adjacent issues parked: none.
- Dispatch rationale: direct-read only; the prior runtime errors and story acceptance criteria defined the complete integration surface.
- Verification: `cargo fmt --all -- --check`, `cargo check --locked --workspace --all-targets`, `cargo clippy --locked --workspace --all-targets -- -D warnings`, `cargo test --locked --workspace`, and `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps` all pass (71 workspace tests).

## Review

Approved. The system runner uses `std::process::Command` with exact `OsString`
arguments, null stdin, piped outputs, and no shell; non-zero exits remain
inspectable results while spawn/wait failures stay typed. Request/output debug
forms and errors omit argument and output contents. The deterministic clock and
system clock are synchronous and terminal-free. Thirteen focused runtime tests
and warnings-denied workspace Clippy pass on review.
