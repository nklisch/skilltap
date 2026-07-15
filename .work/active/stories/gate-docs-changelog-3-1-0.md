---
id: gate-docs-changelog-3-1-0
kind: story
stage: done
tags: [documentation]
parent: null
depends_on: []
release_binding: 3.1.0
gate_origin: docs
created: 2026-04-02
updated: 2026-07-15
---

# Add the 3.1.0 changelog entry

## Drift category
changelog-gap

## Location
- Doc: `CHANGELOG.md:1`
- Contradicting source: `.work/active/release-3.1.0.md:1`

## Current doc text
> # Changelog
>
> ## v3.0.3

## Contradiction
Release `3.1.0` is at quality-gate with the expanded harness support epic, target-agnostic managed fallback, and daemon marketplace refresh bound to it, but the latest changelog entry remains `v3.0.3`.

## Required edit
Add `## v3.1.0` above `v3.0.3`. Summarize the expanded registry and its verified, mixed, declaration-managed, and observe-only tiers; target-agnostic managed projection; canonical project skill links; and marketplace refresh at the start of daemon updates. State active truth without historical transition prose.

## Verification

Added the `v3.1.0` entry with the expanded support tiers, managed projection, canonical project skill tree, and daemon marketplace refresh. `git diff --check` passes.
