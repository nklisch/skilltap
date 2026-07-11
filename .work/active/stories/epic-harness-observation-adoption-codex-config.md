---
id: epic-harness-observation-adoption-codex-config
kind: story
stage: done
tags: [infra,correctness]
parent: epic-harness-observation-adoption-codex
depends_on: [epic-harness-observation-adoption-codex-paths]
release_binding: null
research_refs: [.research/analysis/campaigns/marketplace-standards/specialists/codex.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Parse Codex Configuration Evidence

Observe documented Codex config, marketplace declarations, trust/project
settings, and malformed siblings with strict bounded decoding and safe finding
categories. Preserve unknown native fields, distinguish declared from effective
state, and retain successful siblings when one document is missing, malformed,
or replaced. No writes, cache browsing, or guessed install behavior.

## Implementation

- Added bounded `observe_codex_config` TOML parsing that retains only safe
  counts/presence for marketplaces, plugins, and trust policy while tolerating
  unknown native fields and rejecting malformed/unsupported documents.
- Added redacted Debug and malformed/config secret coverage alongside the
  existing detection test harness.

## Verification

- Harness Clippy and all eight detection/Codex path/config tests pass in the
  locked offline workspace.

## Review

- Fast-lane review approved the bounded, unknown-field-tolerant parser and
  redacted diagnostics with green warnings-denied tests.
