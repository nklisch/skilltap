---
id: story-skilltap-plugin-package-assets
kind: story
stage: done
tags: [architecture, infra]
parent: epic-skilltap-plugin-distribution-package
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Establish canonical plugin publication assets

Create the repository-owned `plugin/` publication root with the complete
shared `skilltap` skill directory, separate native Claude and Codex manifests,
and each harness's native marketplace catalog. Use `skilltap` as the exact
public identity and the Cargo workspace version as the package version. The
catalogs must refer to the package root through their documented relative
source form and must not flatten one harness's schema into the other.

## Acceptance criteria

- `plugin/.claude-plugin/plugin.json` and `plugin/.codex-plugin/plugin.json`
  are valid native manifests with matching public name, description, and
  release version.
- Claude's catalog is at `plugin/.claude-plugin/marketplace.json` and Codex's
  catalog is at `plugin/.agents/plugins/marketplace.json`; each uses its own
  required fields and contains one exact `skilltap` entry.
- `plugin/skills/skilltap/SKILL.md` is a complete strict skill stub with
  top-level `name: skilltap` and non-empty `description`; no component is
  stored below a manifest directory.
- The package tree has no symlinks, path traversal, arbitrary discovery
  metadata, or unrelated harness components.
- The asset layout is exercised by the package validation story before the
  feature can advance.

## Notes

The guidance feature owns the substantive skill prose and may add supporting
files beneath the same complete skill directory. Do not edit the active
`../skills` publisher in this story; release integration validates its direct
source pointer later.

## Implementation notes

- Added the repository-owned `plugin/` root with separate native Claude and
  Codex manifests and catalogs.
- The Codex manifest explicitly points at `./skills/` and keeps its native
  `interface` metadata; the Claude manifest remains free of Codex-only fields.
- Both development catalogs point to the package root (`./`), with Codex using
  its documented local source object. No hooks, MCP servers, executables, or
  cache files were introduced.
- The skill stub is intentionally concise; the guidance feature owns the
  substantive operational content.

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Standard substrate review. Verified the native Claude and Codex
manifests/catalogs, complete skill directory boundary, and package identity
against the implementation and focused/full workspace tests. Corrected the
plugin tree diagram in `docs/ARCH.md` so the foundation matches the checked-in
catalog locations.
