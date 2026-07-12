---
id: story-skilltap-plugin-distribution-cutover-legacy-record
kind: story
stage: done
tags: [content, cleanup]
parent: epic-skilltap-plugin-distribution-cutover
depends_on: [story-skilltap-plugin-distribution-cutover-canonical-verification]
release_binding: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Record legacy skilltap retirement and archive handoff

Write `docs/LEGACY-CUTOVER.md` with the superseded public repository and old
skill surfaces, canonical plugin/website/bootstrap replacements, evidence gate,
and explicit operator deletion/archive checklist. Preserve current-state
truth, do not retain old TypeScript guidance, and state that active `../skills`
is not the retirement target.

Acceptance criteria:

- Legacy `nklisch/skilltap-skills` and `claude-code-marketplace` surfaces are
  named with canonical replacements.
- Deletion/archive is destructive, operator-confirmed, idempotent, and gated on
  published canonical evidence; this repo performs no external mutation.
- Active `../skills` remains explicitly supported and excluded from archive.

## Implementation notes
- Execution capability: standard; current-state deprecation and handoff prose.
- Review weight: standard (autopilot caller policy).
- Files changed: `docs/LEGACY-CUTOVER.md`.
- Tests added: canonical cutover evidence gate covers replacement package,
  installer, bootstrap, and complete guidance tree.
- Discrepancies from design: external archive/delete remains operator-gated;
  this repository performs no remote destructive action.
- Adjacent issues parked: none.

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Fast substrate review at standard weight. The current-state record
names the public retirement target and superseded surfaces, points to the
canonical plugin/installer/bootstrap, excludes active `../skills`, and keeps
destructive deletion/archive explicitly operator-gated.
