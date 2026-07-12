---
id: epic-skilltap-plugin-distribution-package
kind: feature
stage: drafting
tags: [architecture, infra]
parent: epic-skilltap-plugin-distribution
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Canonical Plugin Package and Channel Metadata

## Brief

Create the repository-owned plugin publication tree described by the
foundation: one complete `skilltap` skill directory plus separate native
Claude and Codex manifests and marketplace catalog definitions. The feature
establishes the public plugin identity, component paths, portable frontmatter
rules, and version/source parity checks that every later publication step can
trust.

This is the package contract, not the final guidance prose, binary bootstrap,
or release workflow. It must preserve the harness distinction: Claude and
Codex documents are validated independently, and no Pi or universal plugin
manifest is introduced.

## Epic context

- Parent epic: `epic-skilltap-plugin-distribution`
- Position in epic: foundation package contract; bootstrap, guidance, and
  release work depend on it.

## Foundation references

- `docs/SPEC.md` — Self-Hosted Plugin Distribution, Validation
- `docs/ARCH.md` — Plugin Publication Boundary
- `docs/HARNESS-CONTRACTS.md` — Codex and Claude plugin/marketplace contracts
- `.research/analysis/briefs/current-agent-extension-standards.md`
- `.research/analysis/campaigns/marketplace-standards/specialists/codex.md`
- `.research/analysis/campaigns/marketplace-standards/specialists/claude.md`
- `.research/analysis/campaigns/marketplace-standards/specialists/agent-skills.md`

## Design decisions

- **Canonical source**: The package is authored and versioned in the skilltap
  repository. The active `../skills` repository publishes a second marketplace
  entry that points directly at this plugin subdirectory; `nklisch/skilltap-
  skills` is only a legacy migration source.
- **Channel parity**: Claude and Codex receive distinct native manifests and
  catalogs around one complete shared skill directory; no Pi channel or
  universal manifest is added.

<!-- Feature design will define the exact manifests, catalog entries, and
validation units. -->
