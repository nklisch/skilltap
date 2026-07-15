---
id: gate-cruft-remove-common-target-name-plumbing
kind: story
stage: implementing
tags: [cleanup]
parent: null
depends_on: []
release_binding: 3.1.0
gate_origin: cruft
created: 2026-04-02
updated: 2026-07-15
---

# Remove dead target-name plumbing from shared projection planning

## Confidence
Medium

## Category
dead parameter

## Location
`crates/harnesses/src/adapters/configuration_constrained/common.rs:27`, `:109`, `:148`, and `:180`

## Evidence
`target_name` is threaded through shared skill planning and explicitly discarded with `let _ = target_name;` in three helpers. Shared diagnostics are target-agnostic.

## Removal
Remove the unused parameter from `plan_skills`, `skill_tree`, `observe_tree`, and `verify_prior_skill`, then update Kimi, Vibe, Kilo, Amp, and Junie callers. Preserve all validation and error semantics.
