---
id: epic-real-harness-recovery-native-lifecycle-managed-project-load-contract
kind: story
stage: review
tags: [correctness, architecture, testing]
parent: epic-real-harness-recovery-native-lifecycle
depends_on:
  - epic-real-harness-recovery-native-lifecycle-managed-project-projection-manifest
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

## Implementation notes

- Codex project plugin install/update/remove now projects complete skill
  directories to `<project>/.agents/skills/<name>` and compatible MCP servers
  to `<project>/.codex/config.toml`; copied plugin-bundle presence is no longer
  treated as effective installation.
- Marketplace registration remains the sole owner of the project marketplace
  document. Plugin lifecycle fingerprints only its effective skill/MCP
  projections, so marketplace add → plugin install → marketplace update is a
  healthy no-op instead of self-authored drift.
- Local and Git marketplace sources share the bounded resolver. Git lifecycle
  records the exact checked-out commit and stores its checkout below
  skilltap's managed source root; remote catalog payloads still fail closed.
- Optional plugin components and plugin-root-relative MCP commands require
  `--yes`; accepted unsupported MCP servers are omitted rather than published
  as broken configuration. Existing same-name foreign MCP configuration fails
  closed before mutation.
- Multi-tree/file execution revalidates every surface, rolls earlier
  publications back when a later publication fails, and freshly verifies all
  effective skill and config destinations before state journaling.
- Verification: `cargo test --workspace` passes (521 tests), including an
  isolated compiled local/Git scenario covering executable complete skills,
  MCP projection, partial acknowledgment, foreign ownership, Git SHA
  provenance, cache non-mutation, catalog-update health, repeat no-op, and
  drift refusal.

## Review findings (2026-07-12)

- **Blocker — source evolution loses the owned projection set**: managed state
  records only an aggregate fingerprint, while update and removal reconstruct
  skill and MCP destinations exclusively from the newly resolved source. If a
  release renames or removes a skill/MCP server, the old owned projection is
  never planned for deletion. If the catalog removes the plugin entry,
  `plugin remove` cannot resolve the source at all.
- **Blocker — accepted partial loss is silent and unsupported plugins can be
  uninstallable**: acknowledged optional directories and plugin-root-relative
  MCP omissions are not emitted as exact consequences or retained in state.
  The unsupported-directory check also runs during removal, while
  `plugin remove` has no `--yes`, so a plugin containing hooks, agents, or
  another optional directory cannot be removed through this fallback.

Tracked by
`epic-real-harness-recovery-native-lifecycle-managed-project-projection-manifest`.

## Review (2026-07-12)

**Verdict**: Request changes

**Blockers**:
`epic-real-harness-recovery-native-lifecycle-managed-project-projection-manifest`
**Important**: none
**Nits**: none

**Notes**: Fresh-context deep review at the project-default `standard` weight.
Projection, Git acquisition, containment, and multi-surface filesystem rollback
are materially improved. The remaining blocker is lifecycle identity across
source revisions. The compiled scenario is temporarily red at `6c657f0`
because the concurrently repaired native post-observation fixture reports the
managed operation's harness unreachable; that fallout is not duplicated here.

## Review findings (2026-07-12, final integrated pass)

- **Blocker — successful managed publication is not recoverable after its
  terminal state boundary fails**: the retry predicate and regression do not
  match the Pending representation produced by the journal. Tracked by
  `epic-real-harness-recovery-native-lifecycle-managed-project-journal-recovery`.

## Review (2026-07-12, final integrated pass)

**Verdict**: Request changes

**Blockers**:
`epic-real-harness-recovery-native-lifecycle-managed-project-journal-recovery`
**Important**: none
**Nits**: none

**Notes**: The effective skill/MCP load contract, Git acquisition, partial
disclosure, source-independent removal, and clean-path rollback behavior pass
on cumulative main. The remaining journal-boundary mismatch prevents approval.
