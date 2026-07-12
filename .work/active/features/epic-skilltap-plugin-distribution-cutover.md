---
id: epic-skilltap-plugin-distribution-cutover
kind: feature
stage: drafting
tags: [infra, content, cleanup]
parent: epic-skilltap-plugin-distribution
depends_on: [epic-skilltap-plugin-distribution-release]
release_binding: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Retire the Sibling Skills Publisher

## Brief

Complete the one-way publication cutover from `../skills` to the canonical
skilltap repository. Remove or retire the old skilltap-adjacent guidance,
including `claude-code-marketplace` where it is superseded, add an explicit
deprecation/archive record, and verify that users have a working canonical
plugin and binary path before the old publisher disappears.

This feature tracks the cross-repository handoff and evidence; it does not
create a compatibility layer or preserve the sibling repository as a second
source of truth. Any remaining non-skilltap plugins in that repository are
outside this feature's product scope, even if repository archival requires an
operator-level decision about their future.

## Epic context

- Parent epic: `epic-skilltap-plugin-distribution`
- Position in epic: terminal cutover consumer; it runs only after a verified
  canonical release.

## Foundation references

- `docs/VISION.md` — canonical publisher boundary
- `docs/SPEC.md` — Self-Hosted Plugin Distribution
- `docs/ARCH.md` — Plugin Publication Boundary
- `../skills/AGENTS.md` — current sibling publication rules
- `../skills/.claude-plugin/marketplace.json` — current sibling catalog
- `../skills/.agents/skills/claude-code-marketplace/` — superseded guidance

<!-- Feature design will define the cutover checklist and external-repository
handoff without changing skilltap's native resource semantics. -->
