---
id: story-skilltap-plugin-distribution-cutover-canonical-verification
kind: story
stage: implementing
tags: [infra, security, testing]
parent: epic-skilltap-plugin-distribution-cutover
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Verify canonical plugin and binary cutover evidence

Add an offline, deterministic cutover gate that runs canonical package,
installer, bootstrap, and implicit skill checks before legacy retirement. It
must fail with an actionable missing-evidence message and never mutate native
harness caches or external repositories.

Acceptance criteria:

- Canonical native package identity, complete skill tree, installer contract,
  and bootstrap fixtures all pass before the gate succeeds.
- Missing/failed evidence exits nonzero with the exact remediation boundary.
- The gate is repeatable and isolated from caller HOME/configuration.

## Implementation notes
- Execution capability: highest; publication and retirement safety gate.
- Review weight: standard (autopilot caller policy).
