---
id: epic-safe-update-automation-service-run
kind: story
stage: implementing
tags: []
parent: epic-safe-update-automation-service
depends_on: [epic-safe-update-automation-service-lifecycle]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Run One Safe Daemon Cycle

Invoke the same foreground safe-update planning/application service once per
daemon run, with no acknowledgments and bounded lock/resolver behavior.

Acceptance criteria:

- Daemon cycles never apply partial, drifted, pinned, or conflicted updates.
- Lock/source failures terminate with typed status and no hang.
- Repeating a cycle is idempotent and preserves unmanaged/drifted resources.
