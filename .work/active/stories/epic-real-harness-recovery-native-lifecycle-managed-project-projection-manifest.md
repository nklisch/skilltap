---
id: epic-real-harness-recovery-native-lifecycle-managed-project-projection-manifest
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

# Reconcile managed project projections from an installed component manifest

## Finding

Managed Codex project state stores an aggregate projection fingerprint but not
the exact skill, MCP, and omitted-component set that produced it. Update and
removal instead derive identity from the latest source, so source-shape changes
can strand effective owned components or make uninstall impossible.

## Required fix

- Persist a validated per-target installed projection manifest containing each
  owned effective skill destination, owned MCP server identity/value evidence,
  source revision, and every acknowledged omitted component/consequence.
- Plan update as an old-versus-new manifest reconciliation: remove renamed or
  deleted owned projections, publish new projections, preserve foreign and
  unknown config fields, and replace state only after fresh exact
  postconditions.
- Plan removal from installed state rather than requiring the plugin to remain
  resolvable in the latest catalog/source. Refuse drift or foreign replacement,
  but allow removal of plugins that originally contained unsupported optional
  directories without a new partial-loss acknowledgment.
- Emit exact acknowledged omissions in plain/JSON output and retain them as
  target evidence so status and later lifecycle commands explain the partial
  installation.
- After a journal failure, report the exact manifest and surfaces requiring
  fresh observation.

## Acceptance

- Updating a plugin that removes, renames, or adds skills and MCP servers
  leaves exactly the new faithful projection and no stale owned load surface.
- Removing an installed plugin succeeds after its marketplace entry or source
  disappears, using recorded ownership and fingerprint evidence.
- A plugin with hooks/agents or plugin-root-relative MCP declarations installs
  only with `--yes`, reports and records each omission, and later removes
  without requiring an unavailable acknowledgment.
- Foreign same-name skills/MCP servers and drift block before mutation;
  unrelated config keys and MCP servers survive update/remove semantically.
- Local and Git isolated E2Es cover source-shape evolution, catalog deletion,
  partial disclosure/state evidence, repeat no-ops, rollback, and cache
  non-mutation.
