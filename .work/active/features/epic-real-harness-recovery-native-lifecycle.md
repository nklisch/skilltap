---
id: epic-real-harness-recovery-native-lifecycle
kind: feature
stage: drafting
tags: [correctness, testing]
parent: epic-real-harness-recovery-and-adapter-expansion
depends_on: [epic-real-harness-recovery-runtime-boundary]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Align native lifecycle adapters with current harness contracts

## Brief

Correct Codex and Claude marketplace/plugin observation and mutation vectors,
the attested capability-profile registry, project-scope capability behavior,
update fallbacks, and post-mutation observation. Commands must match current
real CLI help and isolated execution before current versions gain mutation
authority; absent native lifecycle must select the documented managed load-path
fallback or remain explicitly unsupported rather than invoke an invented
command.

This feature owns blocker inventory entries 2, 5-7, and 9-10. It consumes the
runtime feature's version and root model. It does not redesign stored dual-target
provenance or general output aggregation, which are handled by the
state/diagnostics feature.

## Epic context

- Parent epic: `epic-real-harness-recovery-and-adapter-expansion`
- Position in epic: consumes the repaired runtime boundary and enables real
  native marketplace/plugin verification.

## Foundation references

- `docs/HARNESS-CONTRACTS.md` — Codex and Claude native commands, roots, and
  materialization fallback.
- `docs/ARCH.md` — harness adapter contract and plugin resolution.
