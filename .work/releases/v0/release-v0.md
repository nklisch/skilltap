---
id: release-v0
kind: release
stage: released
release_binding: "0"
created: 2026-05-19
---

# Release v0 — pre-substrate history

Synthetic release container created at bootstrap to bind work shipped before
the agile-workflow substrate existed. Items here were classified `done-shipped`
during `/agile-workflow:convert` based on git history and prior placement in
`docs/designs/completed/`.

Bound items:

- `feature-refactor-project-wide-cleanup` — project-wide structural cleanup
  (13-step refactor plan), marked complete in `1ec2f6b`.

No specific git tag corresponds to this synthetic release. Future releases
will use real semver tags (`v2.2.6`, `v2.3.0`, etc.) as `release_binding`.
