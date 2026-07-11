---
id: refactor-cli-composition-boundary
kind: feature
stage: drafting
tags: [refactor]
parent: null
depends_on: []
release_binding: null
research_refs: []
research_origin: null
gate_origin: refactor-design
created: 2026-07-11
updated: 2026-07-11
---

# Deduplicate CLI Document and Scope Composition

## Discovery finding

`crates/cli/src/application.rs` repeats document loading, scope resolution,
target resolution, and error projection across status, adoption,
reconciliation, and inventory-list commands. The duplicated boundary logic is
already causing each new lifecycle command to grow a separate variant.

## Classification

Pure refactor: extract shared read-only composition helpers while preserving
command-specific observation and mutation behavior.

## Value and guardrails

The extraction reduces drift between global/project/all-scope behavior and
keeps output/error mapping consistent. It must not change scope defaults,
target enablement semantics, document read ordering, or native observation
side effects. Add parity tests before deleting duplicated paths.

## Design status

Awaiting the normal refactor-design pass after the current lifecycle adapter
wave. This item is intentionally not folded into a behavior-changing feature.
