---
id: epic-cross-harness-materialization-publish-transaction
kind: story
stage: done
tags: []
parent: epic-cross-harness-materialization-publish
depends_on: [epic-cross-harness-materialization-publish-batch]
release_binding: 3.0.0
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
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

## Implementation Notes

- Added `PublicationSink`, `ManagedPublicationSink`, `apply_publication`,
  `PublishedArtifact`, and `PublicationReceipt` to the core publication
  boundary.
- The managed sink delegates complete-tree publication to the existing
  `ManagedArtifactRepository`; existing artifacts are reported as reused and
  conflicts remain typed repository failures.
- Batch application preserves exact completed entries and resource/target
  context when a later sink call fails. Lock and state ownership remain with
  the caller, so the sink cannot bypass the existing execution lock.
- Verification: targeted publication tests and core clippy passed.

## Review Record

- Inline review: **pass**. The transaction boundary is deterministic,
  idempotence-preserving, and does not introduce a second filesystem writer.
