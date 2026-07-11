---
id: epic-harness-observation-adoption-claude
kind: feature
stage: drafting
tags: [infra]
parent: epic-harness-observation-adoption
depends_on: [epic-harness-observation-adoption-detection]
release_binding: null
research_refs:
  - .research/analysis/campaigns/marketplace-standards/specialists/claude.md
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Claude Code Observation Adapter

Implement read-only Claude observation for user/global and one personal project
scope. Parse bounded JSON list operations and user/local/project/managed
settings; distinguish marketplace-installed plugins, skills-directory plugins,
and standalone whole skills; keep qualified `name@marketplace` identity;
separate enabled declarations from effective cache installs and version basis;
and report trust, consent, policy, bridge, and malformed-state findings.
Project-shared declarations remain observable but non-adoptable. Never mutate
settings/cache, grant consent, or assume repository movement is an update.
