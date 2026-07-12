---
id: epic-real-harness-recovery-filesystem-instructions-relative-bridges
kind: story
stage: implementing
tags: [correctness, security, testing]
parent: epic-real-harness-recovery-filesystem-instructions
depends_on: []
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Compute and validate canonical instruction bridges

## Scope

Replace fixed-depth link strings with one core bridge specification derived
from the actual canonical and native paths. Project effective link targets into
typed observations so status, plan, setup, repair, and sync share one health
classifier.

## Acceptance

- Arbitrary supported `HOME`/`CODEX_HOME` relationships compute a relative link
  whose effective target is the actual canonical `$HOME/AGENTS.md`.
- Health requires that exact resolved canonical path and an existing regular
  destination; dangling, absolute, escaping, wrong-target, and conflicting
  entries fail closed.
- Default global, root project, and nested Claude symlink/import layouts retain
  documented behavior and repeat as no-ops.
- The known custom-home fixed `../AGENTS.md` bridge is reported unhealthy and
  repairable, never managed.
- Unit and compiled-binary coverage uses only isolated roots.

