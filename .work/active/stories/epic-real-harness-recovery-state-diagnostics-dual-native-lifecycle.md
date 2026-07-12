---
id: epic-real-harness-recovery-state-diagnostics-dual-native-lifecycle
kind: story
stage: implementing
tags: [correctness, testing]
parent: epic-real-harness-recovery-state-diagnostics
depends_on:
  - epic-real-harness-recovery-state-diagnostics-target-evidence
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Reconcile dual-native lifecycle without losing siblings

## Scope

Union target sets for sequential identical installs and drive install, update,
and removal from exact target bindings. Preserve unselected desired and
observed siblings, and prove that a plugin native to both harnesses never
falls back to a managed copy.

## Acceptance

- A Codex-only install followed by Claude-only install widens the same desired
  resource to both targets and mutates only the missing target.
- Repeating narrowed and target-all operations is a no-op.
- Target-all install/update/remove records and acts on separate native target
  evidence with no managed plugin artifact.
- Narrowed update/removal preserves the sibling inventory, installation,
  revision, provenance, ownership, and applicable journal evidence.
- A same-key definition conflict fails before inventory publication or native
  mutation.
