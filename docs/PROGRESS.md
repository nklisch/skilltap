# Autopilot Progress

**Status:** in-progress
**Started:** 2026-05-05
**Last updated:** 2026-05-06
**Phases since last refactor:** 5
**Total refactor passes:** 0

Tracking the v2.0 redesign (phases 26–38). Phases 1–25 (v0.1 through v1.0) are historically complete and not tracked here.

---

## Phases

| #  | Phase                                          | Status   | Completed |
|----|------------------------------------------------|----------|-----------|
| 26 | v2.0 Schema Foundation                         | done     | 2026-05-06 |
| 27 | State Consolidation + Migration                | done     | 2026-05-06 |
| 28 | Project Manifest + Lockfile                    | done     | 2026-05-06 |
| 29 | Sync Engine + Command                          | done     | 2026-05-06 |
| 30 | Native Plugin Format + Multi-Plugin Repos      | done     | 2026-05-06 |
| 31 | Security Simplification                        | active   | —         |
| 32 | Agent Flag                                     | pending  | —         |
| 33 | Smart Scope + Status Dashboard                 | pending  | —         |
| 34 | Component-Ref Syntax + Toggle Promotion        | pending  | —         |
| 35 | Try + MCP-Only Install + Claude Desktop        | pending  | —         |
| 36 | Doctor v2.0 Upgrades                           | pending  | —         |
| 37 | Command Surface Promotion + Aliases            | pending  | —         |
| 38 | v2.0 Polish + Docs + Release                   | pending  | —         |

---

## Refactor Log

(none yet)

---

## Decision Log

### Phase 26 setup: keep v1.0 schemas alongside v2.0 schemas

- **Context:** Schemas in `core/src/schemas/config.ts` are actively used by v1.0 install/sync/etc. v2.0 introduces new shapes; the migration command (Phase 27) will need to read v1.0 files.
- **Chose:** Move v1.0 schemas verbatim into `core/src/schemas/v1/` (preserve all exports), introduce v2.0 schemas under their new homes (`core/src/manifest/`, `core/src/state/`, `core/src/schemas/config-v2.ts`, `core/src/plugin-v2/schema.ts`). v2.0 code imports the new ones; migration code imports both.
- **Alternative:** Replace v1.0 schemas in place. Simpler but breaks compile until everything is migrated; loses the historical schema for migration.
- **Reasoning:** The roadmap explicitly says "keep v1.0 schemas in `core/src/schemas/v1/` for migration." Trying to swap in place would require migrating every consumer simultaneously, ballooning Phase 26 scope.

---

## Deviations

### Phase 26: v1.0 schemas not relocated to `schemas/v1/`

- **Expected:** Roadmap 26.3 says "Keep v1.0 in `core/src/schemas/v1/`."
- **Actual:** v1.0 schemas left in place. v2.0 schemas added alongside in new homes (`manifest/`, `plugin-v2/`, `state/`, `schemas/config-v2.ts`).
- **Impact:** None for now. Phase 27 (migration) will copy whichever v1.0 shapes the migration command actually needs into `schemas/v1/`. Avoiding the wholesale move keeps Phase 26 additive and prevents touching dozens of files.

### Phase 26: bun missing on this machine — verification blocked (RESOLVED)

- **Expected:** Run `bun test packages/core/src/manifest/` etc. and verify all new schema tests pass.
- **Actual:** `bun` not on PATH. User installed bun via curl|bash + restart. Resolved.
- **Impact:** Resolved. Phase 26 verified — 82/82 tests pass.

### Phase 27: soft v1 startup hint (deviation from ROADMAP 27.7)

- **Expected:** Hard error on v1.0 markers with hint to run `migrate`.
- **Actual:** Implemented as a soft `↑  v1.0 state detected. Run 'skilltap migrate' to upgrade to v2.0 (preview).` line written to stderr — never blocks startup.
- **Impact:** None. Hard-error gating must wait for Phase 31 (v1.0 readers cut over). Until then a hard gate would break every existing user's everyday commands.

### Phase 28: defer manifest entry resolver (deviation from ROADMAP 28.2)

- **Expected:** `manifest/resolve.ts` (resolves manifest entries to ResolvedDeps[] via source adapters) lands in Phase 28.
- **Actual:** Deferred to Phase 29 (sync engine). The only consumer of resolution is sync; without a consumer, resolve.ts is dead code.
- **Impact:** None — Phase 29 already pulls in source adapter dispatch as part of building the sync plan.

### Refactor gate (after Phase 28): defer to Phase 30

- Per the framework: refactor every 3 phases by default, every 4 if phases were small/independent.
- Phases 26/27/28 were all data-layer (schemas / I/O wrappers / barrels) with minimal coupling between subsystems.
- Decision: skip refactor pass after Phase 28; reassess after Phase 29 lands the sync engine (which actually exercises the new schemas + state + manifest together).

### Phase 29: defer apply to Phase 31 (deviation from ROADMAP 29.3 / 29.4)

- **Expected:** `sync/apply.ts` + CLI `--strict` / `--yes` / `--prune` flags that mutate state.
- **Actual:** Phase 29 ships drift + plan + preview-only sync command. `--apply` errors with a hint pointing at Phase 31.
- **Impact:** Status (Phase 33) and doctor drift checks (Phase 36) consume `planSync()` directly; both work without apply. Apply naturally lands in Phase 31 once v1.0 readers are removed (no need for a v1↔v2 bridge).

### Refactor gate (after Phase 29): defer again

- 4 small additive phases now, all data-layer with no observed duplication or abstractions worth extracting yet.
- Per framework: "Skip entirely if you'd be refactoring for the sake of it."
- Decision: skip refactor pass; revisit after Phase 30 (which touches existing plugin detect/install code — first phase that modifies the legacy surface).

### Phase 30: defer source-string `:plugin-name` parsing to a later phase (deviation from ROADMAP 30.5)

- **Expected:** `skilltap install user/repo:plugin-name` parses the suffix and passes it to detectPlugin for auto-selection.
- **Actual:** `detectPlugin(dir, { selectName })` takes a name and selects correctly when called with one. Install.ts still calls `detectPlugin(contentDir)` with no name. Multi-plugin repos error with a clear hint to specify by name; single-plugin repos auto-select; `.skilltap/` takes priority over `.claude-plugin/` and `.codex-plugin/`.
- **Impact:** Single-plugin .skilltap/ repos work end-to-end. Multi-plugin .skilltap/ repos error helpfully but don't accept a selector yet. Wiring the source string parser into install.ts naturally fits with Phase 33's smart-scope work, which already touches install.ts.

### Refactor gate (after Phase 30): defer again

- Phase 30 added 3 new files (normalize/discover/index in plugin-v2/) and modified 2 existing (plugin/detect.ts, plugin/index.ts) — small surgical change, no duplication discovered.
- Decision: skip refactor pass; revisit after Phase 32 (agent flag) which will touch many files across cli/ and core/.

---

## Suggested Additions

(none yet)

---

## Testing Passes

(none yet)

---

## Completion Summary

(written on completion)
