---
id: epic-harness-observation-adoption-detection-registry
kind: story
stage: implementing
tags: [infra]
parent: epic-harness-observation-adoption-detection
depends_on: [epic-harness-observation-adoption-runtime, epic-harness-observation-adoption-contracts, epic-harness-observation-adoption-detection-fixtures]
release_binding: null
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Detect Harness Installations

Implement the Codex and Claude adapter registry and common installation
detection boundary. Resolve configured binaries through the canonical
executable resolver, run bounded version commands with explicit arguments and
environment, retain opaque native version text only in the typed evidence
envelope, and classify missing, inaccessible, non-runnable, timeout, overflow,
non-zero, malformed, and replacement failures without exposing raw payloads.
Detect siblings independently and do not observe resources, mutate native
state, or expose CLI commands.
