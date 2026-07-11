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

| ID | Title | Kind | archived_atop | Git ref |
|---|---|---|---|---|
| `feature-refactor-project-wide-cleanup` | Project-wide structural cleanup | feature | `pre-release` | `bb11b244dbc165bf5f0e895c332f5f09c51e69c8` |

No specific git tag corresponds to this synthetic release. Future releases
will use real semver tags (`v2.2.6`, `v2.3.0`, etc.) as `release_binding`.
