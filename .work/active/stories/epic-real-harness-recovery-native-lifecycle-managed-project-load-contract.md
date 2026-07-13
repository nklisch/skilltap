---
id: epic-real-harness-recovery-native-lifecycle-managed-project-load-contract
kind: story
stage: implementing
tags: [correctness, architecture, testing]
parent: epic-real-harness-recovery-native-lifecycle
depends_on: []
release_binding: null
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Complete the managed Codex project load contract

## Finding

The project fallback currently treats a copied plugin bundle plus a local
marketplace entry as effective installation. Codex's documented project load
surfaces instead require faithful component projection and observation. The
same implementation gives the marketplace resource and plugin resource
conflicting ownership of one catalog file, accepts only pre-existing local
checkouts, and can leave the tree/catalog pair partially changed.

## Required fix

- Acquire explicit local and Git marketplace/plugin sources through the
  existing bounded resolver model, retain requested/ref and resolved revision
  evidence, and reject remote payloads before filesystem planning when they
  cannot be verified.
- Build the normalized component graph and compatibility result before
  mutation. Publish complete skills to documented project skill paths and MCP
  configuration through the unknown-field-preserving project config adapter.
  Unsupported required components block; optional omissions expose material
  consequences and require the normal `--yes`/piecewise acknowledgment.
- Keep immutable materialized artifacts under skilltap's managed root. Treat a
  project marketplace document as registration/availability only; never claim
  plugin installation until every planned component is freshly observed at an
  effective Codex load surface.
- Give every changed destination one coherent ownership/fingerprint model.
  Plugin install/update/remove must not make the marketplace binding report
  self-authored drift, and marketplace update/remove must preserve unrelated
  installed plugin projections and unknown fields.
- Make multi-surface publication recoverable: a later catalog/config/state
  failure restores or precisely reports earlier tree changes without leaving
  an untracked effective resource.

## Acceptance

- A Git-backed project marketplace can be registered and one exact plugin can
  be installed without cache mutation; the resolved source revision is stored.
- Codex effective observation, not copied bundle presence, gates successful
  materialized state and apply journaling for each skill and MCP component.
- Required unsupported behavior blocks. Optional loss requires explicit
  acknowledgment and records exact consequences.
- Marketplace add → plugin install → marketplace update/remove remains healthy;
  skilltap never diagnoses its own catalog rewrite as external drift.
- Injected failure at each tree/catalog/config/state boundary either restores
  the previous complete representation or reports exact owned residuals.
- Install, update, remove, and marketplace lifecycle repeat as zero-change;
  drift, foreign ownership, path replacement, and malformed source documents
  fail before mutation.
- Isolated compiled E2Es validate project load paths and prove Codex caches and
  the operator's real environment remain untouched.
