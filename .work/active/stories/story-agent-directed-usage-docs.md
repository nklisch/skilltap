---
id: story-agent-directed-usage-docs
kind: story
stage: done
tags: [content]
parent: null
depends_on: []
release_binding: 3.0.2
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Show humans how to delegate skilltap workflows to agents

## Outcome

The landing page, README, getting-started guide, and environment guide now give
high-level prompts such as “Use skilltap to sync…” and explain that agents
should discover exact syntax, plan first, and return partial or conflicting
operations for a human decision.

## Verification

- `scripts/verify-install-surfaces.sh`
- `npm --prefix website run build`

