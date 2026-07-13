---
id: release-3.0.3
kind: release
stage: released
tags: []
parent: null
depends_on: []
release_binding: 3.0.3
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Release 3.0.3

## Bound items

- `story-fix-managed-lifecycle-test-observation` — remove accidental ambient
  Codex dependence from managed lifecycle application tests while preserving
  production post-mutation observation.

## Gate runs

- **gate-security** (2026-07-12) — no findings; production composition remains
  system-observed and the change grants no mutation authority.
- **gate-tests** (2026-07-12) — 1 clean-runner isolation gap, fixed and reviewed.
- **gate-cruft** (2026-07-12) — no findings; reused the existing observation mode.
- **gate-docs** (2026-07-12) — no findings; public behavior and contracts are unchanged.
- **gate-patterns** (2026-07-12) — no new recurring structure.

## Verification

- The three CI-failing tests pass with Git available and Codex absent from PATH.
- Native postcondition tests, formatting, and strict CLI Clippy pass.

## Shipment

- **Date shipped:** 2026-07-12
- **Mapping:** tag-based
- **Items shipped:** 1
- **Gate findings:** 1 test-isolation gap fixed and independently reviewed;
  no security, cruft, documentation, or pattern findings.
- **Publishing:** versioned source tag, GitHub release artifacts, website
  deployment, and Homebrew formula handoff.

## Shipped items

The full body lives in Git history under the configured `delete-refs`
retention policy.

| id | title | kind | archived_atop | git ref |
|----|-------|------|---------------|---------|
| `story-fix-managed-lifecycle-test-observation` | Honor disabled native observation in managed lifecycle tests | story | — | `45f3bbe` |
