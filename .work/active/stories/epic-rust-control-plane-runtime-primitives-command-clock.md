---
id: epic-rust-control-plane-runtime-primitives-command-clock
kind: story
stage: implementing
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
