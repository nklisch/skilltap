---
id: epic-rust-control-plane-cli-shell-command-model
kind: story
stage: implementing
tags: [cli]
parent: epic-rust-control-plane-cli-shell
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Implement the V3 Command Model

Implement the complete Clap command tree and reusable scope, target, selector,
acknowledgment, and output groups. Normalize parsing without process exits and
convert values immediately into validated core request types. Cover exact
optional-project, conflict, flag-relevance, help/version, and representative
nested-command forms with tests. Run the locked ladder.
