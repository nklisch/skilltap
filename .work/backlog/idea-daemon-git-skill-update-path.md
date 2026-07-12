---
id: idea-daemon-git-skill-update-path
created: 2026-07-12
updated: 2026-07-12
tags: [correctness]
---

An isolated Git-backed standalone skill installed through `skill install` is
selected by `daemon run`, but the daemon's update path invokes the installer
with command `daemon run`. When the resolved source tree is compared with the
managed destination, the installer emits `skill_update_required` and leaves
the resource pending instead of performing the safe update cycle described in
`docs/SPEC.md`. The safe-update compiled regression
`safe_update_cycle_reports_changed_git_revision_and_records_daemon_result`
captures the failure; preserve it while making the daemon path reuse the
explicit safe skill-update behavior.
