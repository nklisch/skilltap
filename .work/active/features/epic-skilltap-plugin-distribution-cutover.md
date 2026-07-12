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

# Retire the Legacy Skilltap Skills Publisher

## Brief

Complete the one-way publication cutover from the public
`nklisch/skilltap-skills` repository to the canonical skilltap repository.
Remove or retire its old skilltap-adjacent guidance, including
`claude-code-marketplace` where it is superseded, add an explicit
deprecation/archive record, and verify that users have a working canonical
plugin and binary path before the old publisher disappears.

This feature tracks the cross-repository handoff and evidence; it does not
create a compatibility layer or preserve the legacy repository as a second
source of truth. The active local `../skills` repository is unrelated and must
remain intact. Any remaining non-skilltap content in `skilltap-skills` is
outside this feature's product scope, even if repository archival requires an
operator-level decision about its future.

## Epic context

- Parent epic: `epic-skilltap-plugin-distribution`
- Position in epic: terminal cutover consumer; it runs only after a verified
  canonical release.

## Foundation references

- `docs/VISION.md` — canonical publisher boundary
- `docs/SPEC.md` — Self-Hosted Plugin Distribution
- `docs/ARCH.md` — Plugin Publication Boundary
- `https://github.com/nklisch/skilltap-skills` — legacy public repository
- `https://github.com/nklisch/skilltap-skills/blob/main/README.md` — current
  legacy skilltap distribution surface
- `https://github.com/nklisch/skilltap-skills/tree/main/.agents/skills` —
  superseded guidance locations

## Design decisions

- **Retirement target**: Only the public `nklisch/skilltap-skills` repository is
  retired. The active sibling `../skills` repository remains fully supported
  and is not modified or archived.
- **Cutover gate**: Retire the legacy repository only after the canonical
  plugin, website installer, binary bootstrap, and implicit skill are verified
  as usable.

<!-- Feature design will define the cutover checklist and external-repository
handoff without changing skilltap's native resource semantics. -->
