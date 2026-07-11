---
id: epic-rust-control-plane-cli-shell
kind: feature
stage: drafting
tags: []
parent: epic-rust-control-plane
depends_on: [epic-rust-control-plane-storage, epic-rust-control-plane-runtime-primitives]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Non-Interactive CLI Shell

## Brief

Deliver the runnable `skilltap` composition root and deterministic command tree
for the v3 public surface. Centralize scope, target, and selector argument
validation; render concise plain results or exactly one stable JSON document;
and map completed, invalid, attention-required, and partial-apply outcomes to
the documented exit codes.

Handlers compose core repositories and runtime ports without containing domain
or native-format business logic. Commands whose later capability epics have not
landed must fail explicitly or expose a deliberate foundation-only behavior;
this feature does not simulate harness observation or reconciliation.

## Epic context

- Parent epic: `epic-rust-control-plane`
- Position in epic: integration consumer — depends on storage and runtime
  primitives and establishes the executable contract used by later epics

## Foundation references

- `docs/SPEC.md` — Operating Model, Output, Exit Codes
- `docs/UX.md` — Command Tree, Target and Scope, Common Flags, JSON Output,
  Errors, Exit Codes
- `docs/ARCH.md` — `skilltap-cli`, Dependency Direction, Error Model

<!-- The feature design pass will fill in implementation units. -->
