---
id: gate-cruft-unify-file-managed-skill-planning
kind: story
stage: done
tags: [cleanup]
parent: null
depends_on: []
release_binding: 3.1.0
gate_origin: cruft
created: 2026-04-02
updated: 2026-07-15
---

# Unify duplicated file-managed skill planning

## Confidence
Medium

## Category
over-abstraction / duplicated abstraction family

## Location
`crates/harnesses/src/adapters/gemini_managed.rs:184`, `opencode_managed.rs:184`, `kiro_managed.rs:183`, and `configuration_constrained/common.rs:27`

## Evidence
Gemini, OpenCode, and Kiro duplicate the shared `plan_skills`, `skill_tree`, `fingerprint_tree`, and `append_tree_fingerprint` family already used by configuration-constrained and trust/interactive targets. Their source-plugin input differs only by fields that skill planning does not read.

## Removal
Narrow or unify the source-plugin view accepted by shared planning, preserve target-specific diagnostics where useful, migrate the three adapters to `configuration_constrained::common`, and delete the local copies. Verify lifecycle behavior and fingerprints remain unchanged.

## Verification

Migrated Gemini, OpenCode, and Kiro to a shared, policy-driven skill projection planner while preserving complete-source validation, diagnostics, Kiro Power handling, manifests, fingerprints, and drift semantics.

- `cargo test -p skilltap-harnesses`: 163 passed.
- `cargo clippy -p skilltap-harnesses --all-targets -- -D warnings`: clean.
- Independent standard review: no material findings.
- `git diff --check`: clean.
