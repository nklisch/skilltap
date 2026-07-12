---
id: gate-patterns-3.0.0
kind: story
stage: done
tags: [patterns]
parent: null
depends_on: []
release_binding: 3.0.0
gate_origin: patterns
created: 2026-07-12
updated: 2026-07-12
---

# Patterns extracted for 3.0.0

## New patterns codified

- `validated-wire-contract` — Serialize domain values through private wire DTOs and rebuild them through validating constructors.
- `validated-string-newtypes` — Represent bounded domain text with one validated, serde-aware newtype rather than raw `String`.
- `bounded-native-process-port` — Resolve binaries and run direct argument vectors through the bounded runner with explicit limits.
- `isolated-native-fixture-roots` — Exercise native and filesystem behavior only inside test-support-owned temporary roots and fake binaries.

## Inconsistencies flagged

None. No existing pattern catalog was present, and inspected production native
process call sites use the bounded request boundary.

## Pattern files written

- `.agents/skills/patterns/validated-wire-contract.md`
- `.agents/skills/patterns/validated-string-newtypes.md`
- `.agents/skills/patterns/bounded-native-process-port.md`
- `.agents/skills/patterns/isolated-native-fixture-roots.md`
- `.agents/skills/patterns/SKILL.md` (new index)
- `.agents/rules/patterns.md` (generated hook-loaded digest)

The repository's `.claude/skills` path already symlinks to `.agents/skills`,
so the canonical pattern tree is mirrored without a second source of truth.
