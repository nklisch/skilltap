---
id: story-skilltap-plugin-distribution-cli-help-contract
kind: story
stage: implementing
tags: [content, testing]
parent: epic-skilltap-plugin-distribution-cli-contract
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Complete executable help contract

Implement the clap help contract described by the parent feature. Add concise
purpose text to every public argument, attach one shared exit-status footer to
all public leaf commands, and preserve the existing command grammar and
validators.

Acceptance criteria:

- `Cli::command()` exposes truthful purpose and usage text for the root, groups,
  and every leaf.
- Every user-facing argument has help text and an appropriate value name.
- Every leaf help page states exit classes 0/1/2/3 and only advertises flags
  meaningful to that operation.
- `crates/cli/src/command/tests.rs` walks the generated tree and catches missing
  descriptions or misplaced scope, target, acknowledgment, selector, and JSON
  flags.
- Existing parser behavior and the shared `--yes` semantics are unchanged.
