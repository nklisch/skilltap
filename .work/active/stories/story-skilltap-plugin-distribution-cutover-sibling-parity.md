---
id: story-skilltap-plugin-distribution-cutover-sibling-parity
kind: story
stage: review
tags: [infra, testing, cleanup]
parent: epic-skilltap-plugin-distribution-cutover
depends_on: [story-skilltap-plugin-distribution-cutover-canonical-verification]
release_binding: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Check active sibling marketplace parity without mutation

Extend the explicit sibling marketplace pointer check to serve cutover
evidence. A supplied `../skills` checkout must contain a `skilltap` entry that
points directly at this repository's `plugin/` subtree and canonical identity;
wrong/missing entries report remediation while leaving the active repository
untouched. Absence of a sibling checkout remains a safe skip.

Acceptance criteria:

- Valid direct canonical pointer passes deterministically.
- Missing, copied, or wrong-repository pointers fail with an actionable message
  when an explicit checkout is supplied.
- No check can delete, archive, or rewrite `../skills`.

## Implementation notes
- Execution capability: highest; cross-repository publication safety boundary.
- Review weight: standard (autopilot caller policy).
- Files changed: `scripts/verify-install-surfaces.sh`, `docs/LEGACY-CUTOVER.md`.
- Tests added: explicit `SKILLTAP_SKILLS_MARKETPLACE` read-only pointer validation; absent sibling checkout safely skips.
- Discrepancies from design: no sibling checkout is mutated or archived; local default remains a safe skip until an external parity checkout is supplied.
- Adjacent issues parked: none.
