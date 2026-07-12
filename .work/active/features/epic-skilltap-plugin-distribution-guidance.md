---
id: epic-skilltap-plugin-distribution-guidance
kind: feature
stage: drafting
tags: [content]
parent: epic-skilltap-plugin-distribution
depends_on: [epic-skilltap-plugin-distribution-package, epic-skilltap-plugin-distribution-cli-contract, epic-skilltap-plugin-distribution-bootstrap]
release_binding: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Skilltap Agent Guidance

## Brief

Author the single portable `skilltap` skill carried by the plugin. It should
help an agent recognize when to invoke the binary, choose among status,
adopt, plan, sync, lifecycle, instruction, and daemon commands, understand
global/project scope, locate configuration and managed state, diagnose common
health results, and explain the next user decision. It should link agents to
the executable help surface for exact syntax rather than freezing a duplicate
command reference in prose.

The skill remains harness-neutral and contains the complete directory
artifact, with supporting references only when they materially improve
diagnosis. This feature also defines the replacement/deprecation wording for
the obsolete skilltap-adjacent guidance, while the actual sibling repository
archive is handled by the final cutover feature.

The skill is implicitly available to agents. It points agents to the
self-bootstrap flow, direct `--help`, status/plan/sync diagnostics, and the
latest-compatible update policy without turning the skill into a second CLI
implementation.

## Epic context

- Parent epic: `epic-skilltap-plugin-distribution`
- Position in epic: consumer of the package, CLI, and bootstrap contracts;
  release integration packages the resulting skill.

## Foundation references

- `docs/VISION.md` — Agent Forward, Non-Goals
- `docs/SPEC.md` — Self-Hosted Plugin Distribution, Output, Configuration
  Directory, Status, Planning, Synchronization
- `docs/UX.md` — Help and Diagnostic Discovery, Target and Scope
- `README.md` and `website/guide/` — current user-facing operating model
- `.research/analysis/campaigns/marketplace-standards/specialists/agent-skills.md`

## Design decisions

- **Invocation policy**: The skill is implicitly available to agents so they
  can recognize self-setup, status, and recovery requests without a manual
  invocation ceremony.
- **Guidance boundary**: The skill explains bootstrap, harness detection,
  configuration layout, update policy, and diagnostic next actions at a high
  level; direct `--help` remains authoritative for exact syntax.

<!-- Feature design will define the skill's progressive-disclosure sections and
reference boundaries. No UI mockups apply to this terminal/skill surface. -->
