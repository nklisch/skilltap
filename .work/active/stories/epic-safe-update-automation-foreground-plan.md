---
id: epic-safe-update-automation-foreground-plan
kind: story
stage: done
tags: []
parent: epic-safe-update-automation-foreground
depends_on: []
release_binding: 3.0.0
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Assemble Foreground Update Plans

Pair exact desired resources with resolved update candidates and emit a
deterministic plan of safe operations plus typed blocked/needs-decision
findings.

Acceptance criteria:

- Clean tracked candidates produce stable update operations.
- Blocked, pinned, drifted, and unresolved candidates produce no mutation.
- Resource scope and target identity remain exact in every finding.

## Implementation Notes

- Added the pure `foreground_update` planning boundary. It pairs each desired
  scope-bearing resource with exactly one candidate, rejects missing,
  duplicate, and unexpected candidates, and sorts entries deterministically.
- Safe/blocked/needs-decision classification delegates to the shared update
  policy; no native or filesystem mutation occurs.
- Verification: targeted foreground planner tests and core clippy passed.

## Review Record

- Inline review: **pass**. Candidate pairing fails closed and preserves the
  existing policy decision without inventing mutation behavior.
