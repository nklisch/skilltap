---
id: epic-real-harness-recovery-native-lifecycle-managed-project
kind: story
stage: implementing
tags: [correctness, testing]
parent: epic-real-harness-recovery-native-lifecycle
depends_on: [epic-real-harness-recovery-native-lifecycle-managed-project-load-contract]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Materialize unsupported Codex project lifecycle safely

## Scope

Resolve Codex project marketplace/plugin operations to the documented managed
load-path lifecycle when native project commands are unavailable. This story
owns blocker 9.

## Acceptance

- Codex project operations use validating project marketplace edits and owned
  plugin/skill/MCP load-path publications without invoking an unverified native
  command or writing a cache.
- Explicit sources are bounded and validated before planning; complete required
  components are faithful, optional omissions are disclosed/acknowledged, and
  missing required behavior remains blocked.
- Materialized state records skilltap ownership and source/fingerprint evidence,
  not native provenance.
- Update and removal preserve unknown native fields and fail closed on drift,
  foreign ownership, or changed destinations.
- Successful install/update/remove operations repeat as zero-change; authorized
  global Codex and Claude operations remain native.

## Implementation notes

- Execution capability: strongest available; the change crosses native
  authorization, managed filesystem publication, catalog parsing, provenance,
  and drift protection.
- Review weight: highest, inherited from the caller's recovery/autopilot scope.
- Files changed: `crates/harnesses/src/managed_codex_project.rs`,
  `crates/harnesses/src/lib.rs`, `crates/harnesses/src/lifecycle.rs`,
  `crates/core/src/lifecycle_operation.rs`,
  `crates/cli/src/application.rs`,
  `crates/cli/src/application/execution.rs`,
  `crates/cli/src/application/lifecycle.rs`, and
  `crates/cli/tests/compiled_binary.rs`.
- Tests added: bounded catalog parsing, contained named local-source
  resolution, duplicate/path-escape rejection, unknown-field preservation,
  and an isolated compiled-CLI scenario covering project marketplace add,
  complete plugin install, executable sibling preservation, repeat no-op,
  materialized ownership/provenance, cache non-mutation, and drift refusal.
- Discrepancies from design: remote Git sources fail closed with an actionable
  local-checkout requirement because the existing source boundary resolves a
  Git revision but does not expose a verified checkout tree. Managed Codex
  project publication therefore consumes an explicit bounded local marketplace
  checkout; it never interprets a remote catalog or mutates a cache.
- Verification: the focused harness adapter suite passes (2 tests),
  `git diff --check` passes, and the CLI crate compiled immediately before the
  concurrent per-target `ResourceState` schema transition. The compiled CLI
  E2E is present but temporarily cannot compile until the overlapping CLI
  constructors are migrated to that new schema; the coordinating worker owns
  that migration and will rerun this scenario afterward.
- Adjacent issues parked: none.

## Repair follow-up (2026-07-12)

- The corrective load-contract story replaced copied plugin bundles with
  effective complete-skill and MCP projections, separated marketplace and
  plugin ownership, added bounded Git checkout/revision evidence, and made
  multi-surface publication recoverable and freshly verified.
- The isolated local/Git compiled lifecycle scenario and full 521-test
  workspace suite pass. The three review blockers below are resolved and this
  parent is returned to review.

## Review findings (2026-07-12)

- **Blocker — materialized plugin is not published or verified through an
  effective Codex load path**: the implementation copies the whole source tree
  to `<project>/.agents/plugins/<name>` and rewrites the project marketplace
  entry, then records materialized ownership. A marketplace source makes a
  plugin available; it is not itself effective installation. No complete skill
  is projected to `<project>/.agents/skills`, no MCP configuration is projected
  to `<project>/.codex/config.toml`, compatibility/acknowledgment is not applied,
  and no effective load observation gates state publication. This contradicts
  `docs/ARCH.md`'s explicit managed-projection contract. Tracked by
  `epic-real-harness-recovery-native-lifecycle-managed-project-load-contract`.
- **Blocker — shared catalog ownership self-invalidates lifecycle and later
  failures are not transactional**: marketplace registration fingerprints the
  project catalog, then plugin install rewrites that same file without
  refreshing the marketplace target binding. An isolated reproduction of
  marketplace add → plugin install → marketplace update exits 2 with
  `managed_project_drifted`, even though skilltap caused the change. The
  execution port also changes the plugin tree before the catalog write and
  does not restore the tree when that later write fails. Tracked by the same
  corrective story.
- **Blocker — normal Git marketplace sources cannot enter the fallback**:
  project lifecycle rejects every non-local source with
  `managed_project_source_requires_checkout`, while the product contract
  requires skilltap-owned acquisition from explicit accessible sources.
  Tracked by the same corrective story.

## Review (2026-07-12)

**Verdict**: Request changes

**Blockers**:
`epic-real-harness-recovery-native-lifecycle-managed-project-load-contract`
**Important**: none
**Nits**: none

**Notes**: Fresh-context deep lane at review weight `standard` (explicit
caller selection). The focused happy-path and full workspace suites pass, but
adversarial lifecycle and foundation-contract review exposed the blockers
above. Security/path containment is sound for the supported local input; the
failure is capability completeness, effective-state truth, ownership, source
coverage, and rollback behavior.

## Review findings (2026-07-12, projection lifecycle pass)

- **Blocker — update/remove are not closed over the installed component set**:
  the corrective implementation retains only an aggregate hash and derives
  destinations from the current source. Renamed/removed upstream skills and
  MCP servers remain effective, and a removed catalog entry can prevent
  uninstall. Accepted omissions are silent, and optional plugin directories
  incorrectly require an unavailable acknowledgment during removal. Tracked by
  `epic-real-harness-recovery-native-lifecycle-managed-project-projection-manifest`.

## Review (2026-07-12, projection lifecycle pass)

**Verdict**: Request changes

**Blockers**:
`epic-real-harness-recovery-native-lifecycle-managed-project-projection-manifest`
**Important**: none
**Nits**: none

**Notes**: Fresh-context deep review at the project-default `standard` weight.
Effective load surfaces and transaction handling are sound for an unchanged
source shape, but update/remove require exact prior component identity.
