---
id: story-skilltap-plugin-distribution-cli-help-contract
kind: story
stage: done
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

## Implementation notes
- Execution capability: highest available local capability; this changes the public agent-facing CLI contract.
- Review weight: standard (autopilot project default).
- Files changed: `crates/cli/src/command.rs`, `crates/cli/src/command/tests.rs`.
- Tests added: generated command-tree help/exit-footer coverage and scoped flag placement assertions.
- Discrepancies from design: none; Clap remains the sole command grammar source.
- Adjacent issues parked: none.

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Standard fresh-context review of the public CLI contract. Generated
help metadata covers all 26 executable leaves, non-help arguments have
purpose/value text, exit guidance is shared, and scope/target/selection/
acknowledgment/JSON flags remain constrained to their intended operations.
Command parsing and the generic `--yes` semantics remain unchanged. The
conceptual website reference was corrected to describe generic partial/lossy
acknowledgment accurately.
