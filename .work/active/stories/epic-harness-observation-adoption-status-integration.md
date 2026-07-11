---
id: epic-harness-observation-adoption-status-integration
kind: story
stage: implementing
tags: [cli,testing]
parent: epic-harness-observation-adoption-status
depends_on: [epic-harness-observation-adoption-status-policy, epic-harness-observation-adoption-status-observation]
release_binding: null
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Verify Harness Status CLI

Exercise plain and JSON list/enable/disable/status flows across global,
current/explicit project, all-scopes, and named targets. Cover first-use
no-create, partial sibling success, stable exit classes, repeat idempotence,
safe diagnostics, and native byte/type/link/mtime no-mutation.
