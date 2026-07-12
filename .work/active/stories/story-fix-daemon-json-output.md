---
id: story-fix-daemon-json-output
kind: story
stage: review
tags: [bug]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Accept JSON Output for One Daemon Cycle

## Symptom

`skilltap daemon run --json` is rejected as invalid arguments even though the
other non-interactive commands support structured output.

## Root cause

The daemon run subcommand had no output argument and dispatch always rendered it
as plain output.

## Fix approach

Add the existing `OutputArgs` to the daemon run leaf and route its JSON choice
through dispatch and the normal renderer without changing service-manager
invocation behavior.

## Regression test

The compiled CLI test suite will assert `daemon run --json` returns one JSON
document and a valid daemon result.

## Implementation notes

- `DaemonCommand::Run` now accepts the existing `OutputArgs` and dispatches its
  JSON choice through the normal renderer.
- `crates/cli/src/command/tests.rs` and
  `crates/cli/tests/compiled_binary.rs` cover parsing and one-document output.
- `docs/SPEC.md` and `docs/UX.md` document the optional structured-output flag.
- Service-manager invocation remains unchanged because the no-flag form still
  parses identically.
- Full workspace tests and clippy with `-D warnings` pass.
