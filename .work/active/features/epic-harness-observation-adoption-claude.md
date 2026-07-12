---
id: epic-harness-observation-adoption-claude
kind: feature
stage: done
tags: [infra]
parent: epic-harness-observation-adoption
depends_on: [epic-harness-observation-adoption-detection]
release_binding: 3.0.0
research_refs:
  - .research/analysis/campaigns/marketplace-standards/specialists/claude.md
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
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

## Design

The Claude adapter composes detection, strict bounded JSON, and external-tree
runtime ports for user/global and one personal project scope. It treats
qualified `name@marketplace` as identity, distinguishes marketplace plugins,
skills-directory plugins, and standalone complete skills, and keeps enabled
declarations separate from effective cache installations and version basis.
Settings and cache are evidence only; shared project declarations remain
observable but non-adoptable. Trust, consent, policy, bridge, and malformed
state become typed findings without exposing native payloads.

## Design decisions

- **Identity**: marketplace-qualified names remain distinct from unqualified
  local plugin names; similar names or URLs never imply equivalence.
- **Scope**: global and one personal project scope are supported; shared
  project declarations are reported but cannot become adoption candidates.
- **Cache**: effective cache contents corroborate enabled declarations and
  version basis only; cache presence never grants consent or mutation authority.

## Implementation units

1. `epic-harness-observation-adoption-claude-paths` — derive bounded Claude
   global/project settings, plugin, cache, and skill roots — depends on
   `[epic-harness-observation-adoption-detection,
   epic-harness-observation-adoption-runtime]`.
2. `epic-harness-observation-adoption-claude-settings` — parse bounded settings
   and declarations, preserving qualified identities and malformed siblings —
   depends on `[epic-harness-observation-adoption-claude-paths]`.
3. `epic-harness-observation-adoption-claude-resources` — observe plugins,
   complete skills, cache effective state, trust/consent/policy findings —
   depends on `[epic-harness-observation-adoption-claude-settings]`.
4. `epic-harness-observation-adoption-claude-integration` — verify global,
   personal project, shared declarations, cache/trust distinctions,
   deterministic no-mutation and safe errors — depends on
   `[epic-harness-observation-adoption-claude-paths,
   epic-harness-observation-adoption-claude-settings,
   epic-harness-observation-adoption-claude-resources]`.

## Acceptance criteria

- Claude observation is bounded, deterministic, read-only, and scope-exact.
- Qualified marketplace/plugin identities, enabled/effective/cache state,
  complete skills, trust/consent/policy, bridges, and malformed siblings are
  represented safely without adoption or mutation authority.
- Linux and native macOS behavior suites pass without settings/cache writes.

## Implementation

- Completed all four Claude stories: bounded path derivation, settings and
  qualified identity parsing, complete plugin/skill/cache tree observation,
  and integration verification.
- `skilltap-harnesses` now keeps declarations, effective cache evidence,
  shared-project state, and trust/consent signals separate without mutation or
  adoption authority.

## Verification

- Twelve harness detection/Codex/Claude tests, 16 fixtures, workspace Clippy,
  and the locked integration suites pass with safe diagnostics and no native
  settings/cache writes.

## Review

- Aggregate review approved from the green child records and locked workspace
  verification; macOS execution remains CI-gated.
