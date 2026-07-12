---
id: story-daemon-git-skill-update-path
kind: story
stage: implementing
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
