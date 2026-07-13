---
id: epic-real-harness-recovery-native-lifecycle-managed-project-projection-manifest
kind: story
stage: done
tags: [correctness, architecture, testing]
parent: epic-real-harness-recovery-native-lifecycle
depends_on:
  - epic-real-harness-recovery-native-lifecycle-managed-project-journal-recovery
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

## Implementation notes

- Added a backward-compatible typed per-target manifest for owned skill and
  MCP projections, each with its component fingerprint, plus exact
  acknowledged omission/consequence records. State refresh and operation
  journaling preserve the manifest; older state defaults to no manifest.
- Update reconciles the union of prior and desired component identities,
  verifies every prior projection before mutation, removes deleted/renamed
  skills and MCP entries, and preserves unrelated TOML fields and servers.
- Removal plans entirely from installed state, so a missing catalog entry or
  source does not prevent cleanup. Pre-manifest installations fail with the
  actionable `managed_project_projection_manifest_missing` diagnostic instead
  of guessing ownership.
- Accepted omissions appear as exact `omitted:<component>` output resources
  and remain persisted as target evidence. Removal ignores prior omission
  acknowledgment because omitted surfaces were never installed.
- The compiled isolated scenario covers local skill/MCP rename and restoration,
  repeat no-op, per-component drift refusal, Git SHA provenance, partial MCP
  disclosure, foreign MCP collision, catalog-deletion uninstall, cache
  non-mutation, and exact state/output evidence.

## Review findings (2026-07-12, final integrated pass)

- **Blocker — the managed Pending recovery predicate rejects the real journal
  shape**: first-install Pending state retains the attempted manifest/revision,
  while updates retain the previous effective binding. The recovery predicate
  accepts neither, and the regression manually deletes those fields before
  retrying. A successful apply followed by terminal state-publication failure
  therefore becomes self-authored drift. Tracked by
  `epic-real-harness-recovery-native-lifecycle-managed-project-journal-recovery`.

## Review (2026-07-12, final integrated pass)

**Verdict**: Request changes

**Blockers**:
`epic-real-harness-recovery-native-lifecycle-managed-project-journal-recovery`
**Important**: none
**Nits**: none

**Notes**: Fresh-context deep review at the caller's highest
(maximum-equivalent) weight. The cumulative 8679f8b workspace suite and strict
Clippy pass, and source-independent removal, old-versus-new cleanup, exact
omissions, Git/local isolation, schema conflict rejection, cache non-mutation,
and bounded rollback reporting are sound. The blocker is a direct mismatch
between the journal writer, recovery predicate, and its fixture.

## Review (2026-07-12, blocker closure)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: The sole final blocker is closed by the exact Pending-attempt model
in `730faf2`. Cumulative manifest reconciliation, source-independent removal,
omission evidence, drift protection, and rollback coverage remain green in the
full workspace suite and strict Clippy.
