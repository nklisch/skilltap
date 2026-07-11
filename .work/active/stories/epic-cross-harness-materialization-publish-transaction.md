---
id: epic-cross-harness-materialization-publish-transaction
kind: story
stage: implementing
tags: []
parent: epic-cross-harness-materialization-publish
depends_on: [epic-cross-harness-materialization-publish-batch]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Apply Managed Publication Transactions

Connect publication batches to the existing managed-artifact repository and
configuration lock. Preserve idempotence, ownership boundaries, and typed
partial publication residuals.

Acceptance criteria:

- Repeating an identical publication is a no-op.
- Conflicts never overwrite unmanaged destinations.
- A later-target failure is surfaced as partial publication with exact target
  context.
