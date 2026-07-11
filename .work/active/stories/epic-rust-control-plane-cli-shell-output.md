---
id: epic-rust-control-plane-cli-shell-output
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

# Implement Stable Outcomes and Rendering

Implement the schema-1 outcome envelope, safe error/next-action values,
plain/JSON renderers, and the completed/invalid/attention/partial exit mapping.
Both representations must derive from one outcome, JSON must be exactly one
document, and renderers must not expose debug/native source values. Add focused
contract tests and run the locked ladder.
