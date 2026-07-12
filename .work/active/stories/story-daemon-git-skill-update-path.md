---
id: story-daemon-git-skill-update-path
kind: story
stage: done
parent: null
depends_on: []
release_binding: 3.0.0
created: 2026-07-12
updated: 2026-07-12
tags: [correctness]
---

# Route daemon Git skill updates through safe update behavior

An isolated Git-backed standalone skill installed through `skill install` is
selected by `daemon run`, but the daemon's update path invokes the installer
with command `daemon run`. When the resolved source tree is compared with the
managed destination, the installer emits `skill_update_required` and leaves
the resource pending instead of performing the safe update cycle described in
`docs/SPEC.md`. The safe-update compiled regression
`safe_update_cycle_reports_changed_git_revision_and_records_daemon_result`
captures the failure; preserve it while making the daemon path reuse the
explicit safe skill-update behavior.

## Implementation scope

Make the daemon's safe Git-backed standalone-skill path perform the same
validated update operation as the explicit skill-update command, while
retaining daemon acknowledgment/drift safety and the compiled regression.

## Source

Promoted from `idea-daemon-git-skill-update-path` after the release safe-update
e2e test exposed the production defect.

## Implementation notes

- The daemon now routes Git-backed standalone skills through the explicit
  `skill update` command path, preserving its validated source resolution,
  managed-destination fingerprint check, atomic replacement, and state
  journaling behavior.
- The daemon wrapper continues to reject drift, pinned resources, and other
  judgment-required outcomes; it only changes the child command used for the
  already-selected safe update.
- Focused verification reaches the replacement operation and records the new
  Git revision. The existing compiled fixture has a stale naming assertion: it
  installs a repository named `daemon-skill-source` without `--name` but reads
  `daemon-skill`; the test fixture should be corrected separately without
  changing the documented source-name default.

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Standard substrate review with correctness, tests, update-safety,
and foundation-doc lenses. The daemon now delegates Git-backed skill refreshes
through the validated `skill update` path, retaining drift/pin safety and
state journaling. The compiled safe-update regression uses the explicit
`--name daemon-skill` fixture and passes through no-op and changed-revision
cycles.
