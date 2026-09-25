---
id: release-3.3.0
kind: release
stage: released
tags: []
parent: null
depends_on: []
release_binding: 3.3.0
created: 2026-09-24
updated: 2026-09-24
---

# Release 3.3.0

## Changes

- Remove exact harness and companion version allowlists and their redundant
  fallback capability machinery. Runtime command/schema checks still apply.
- Repair target ownership during global removal and sequential installation.
- Report failed or partial skill publication accurately.
- Resolve relative local skill paths using filesystem semantics, including
  symlink parents, and retain complete directory contents.
- Restrict updates to desired bindings and preserve independent sources across
  scopes. Shared content replacements require all desired targets; shared native
  paths are written once and verified for each target binding.
- Produce empty plans for satisfied skills, compare global skill state, and
  report attention for available updates or failed update checks.
- Permit first-time declaration-managed installs into absent configuration roots.

## Verification

- 759 workspace tests pass, including 91 compiled CLI tests.
- The optimized binary passes the 91 compiled CLI tests and the independent
  audit's seven regression probes plus positive workflow controls.
- Formatting, Clippy with warnings denied, release identity, plugin packaging,
  offline installer and installation-surface checks pass.
- Documentation builds successfully and generated LLM documentation agrees.
- Current installed Codex 0.155.1 and Claude Code 2.1.282 pass isolated-home skill
  install, repeat, plan, status, removal, and no-change repeated removal checks.
  These checks verify file lifecycle, not interactive model loading or activation.

## Review

Security, tests, cleanup, documentation, and project patterns were reviewed.
The peer review's global-update isolation, failed-status-check, symlink-parent,
shared-path replacement, and removal-after-update findings were fixed and
covered by regressions. Path confinement, credential isolation, ownership,
drift checks, and operation-scoped failure reporting are retained.

## Shipment

Tag: `v3.3.0`. Release assets are built by the tag workflow for Linux x64/ARM64
and macOS x64/ARM64, with checksums and provenance attestations. Homebrew is
updated through the release workflow's tap pull request.

The full audit and remediation record is in
[story-audit-skill-lifecycle-regressions.md](story-audit-skill-lifecycle-regressions.md).
