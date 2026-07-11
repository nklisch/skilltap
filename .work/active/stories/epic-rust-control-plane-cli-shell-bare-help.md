---
id: epic-rust-control-plane-cli-shell-bare-help
kind: story
stage: implementing
tags: [bug, cli]
parent: epic-rust-control-plane-cli-shell
depends_on: [epic-rust-control-plane-cli-shell-composition]
release_binding: null
gate_origin: tests
created: 2026-07-11
updated: 2026-07-11
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
