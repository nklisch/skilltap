---
id: story-skilltap-plugin-distribution-guidance-core
kind: story
stage: review
tags: [content]
parent: epic-skilltap-plugin-distribution-guidance
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Author the portable skilltap activation skill

Write the complete `plugin/skills/skilltap/SKILL.md` activation surface. Keep
the frontmatter portable and the body concise, agent-forward, and harness
neutral. It should teach an agent when to invoke skilltap, select a command
family, choose target/scope, run bootstrap or status first, and defer exact
syntax to executable help.

Acceptance criteria:

- Required Agent Skills frontmatter is valid and matches the containing
  directory name.
- Setup, health, adoption, plan/sync, lifecycle, instructions, update, and
  daemon intents each have a clear first command or help route.
- The body explains no-search/no-recommendation, native-first behavior,
  partial-operation acknowledgment, and separate binary/harness bootstrap
  results.
- Configuration, instruction, and diagnostic detail links to the sibling
  references rather than duplicating a command table.

## Implementation notes
- Execution capability: standard; prose artifact constrained by foundation and extension contracts.
- Review weight: standard (autopilot caller policy).
- Files changed: `plugin/skills/skilltap/SKILL.md`.
- Tests added: existing offline plugin package validation passes with the complete portable skill tree.
- Discrepancies from design: none.
- Adjacent issues parked: none.
