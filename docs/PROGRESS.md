# Autopilot Progress

**Status:** in-progress
**Started:** 2026-05-05
**Last updated:** 2026-05-06
**Phases since last refactor:** 8
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
| 31a | v2 policy compose + trust-glob                 | done     | 2026-05-06 |
| 31b | HTTP registry adapter removal                  | done     | 2026-05-06 |
| 31c | Install/update/remove cutover + sync apply     | pending  | —         |
| 32  | Agent flag (subsumed by 31a; cutover w/ 31c)   | pending  | —         |
| 33a | Status dashboard (additive)                    | done     | 2026-05-06 |
| 33b | Smart scope default in policy compose          | pending  | —         |
| 34  | Component-ref syntax + toggle promotion        | pending  | —         |
| 35  | Try + MCP-only install + Claude Desktop        | pending  | —         |
| 36  | Doctor v2.0 upgrades                           | pending  | —         |
| 37  | Command surface promotion + aliases            | pending  | —         |
| 38  | v2.0 polish + docs + release                   | pending  | —         |

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

### Phase 31 split into 31a / 31b / 31c

- Original Phase 31 (~25 implementation units) was the full security simplification + HTTP removal + install cutover. Too big for one session and high blast radius.
- 31a (done): additive v2 policy module — composeV2, trust-glob.
- 31b (pending): HTTP registry adapter removal. Best done together with 31c since the v1 readers also need updates.
- 31c (pending): full install/update/remove cutover to v2 state + sync apply. Major destructive change.

### Phase 31b/c deferral and Phase 32 reordering

- Decision: defer 31b/c and Phase 32 cutover until after additive Phases 33–36 land.
- Rationale: 31b/c/32 all touch the same files (install.ts, policy.ts, taps.ts, plugin/state.ts, cli command files) and risk cascading test breakage. Doing them together when the new shapes are fully in place is safer than piecemeal.
- 33–36 are mostly additive (new commands, new doctor checks, new detect targets) and don't depend on the cutover. They can ship now and create user-visible value without the destructive churn.
- Phase 32 (agent flag) is largely subsumed by 31a's `composeV2` (which already implements --agent flag, SKILLTAP_AGENT env, config.agent.default, agent.block). Phase 32's remaining work is the cutover — replacing v1 agent-mode checks across the codebase, which fits naturally with 31c.

### Phase 33: scoped to status dashboard (additive)

- 33.1 (smart scope default in policy compose) deferred to 31c — modifying v1 policy.ts is part of the cutover.
- 33.2–33.7 (status command, bare command, --json) are purely additive: new CLI command reading v2 state.json + manifest + lockfile + drift via planSync().
- Status command falls back gracefully when state.json doesn't exist yet (most current users), suggesting `skilltap migrate`.

---

## Suggested Additions

(none yet)

---

## Testing Passes

(none yet)

---

## Session Handoff Notes (2026-05-06)

**Next session resumes at: Phase 31 — Security Simplification.**

Phases 26–30 are `done`, all tests pass. The v2.0 data layer is in place:
schemas, state.json, migrate command, project manifest + lockfile I/O,
sync drift/plan, native .skilltap/ plugin format. Nothing v1.0 has been
removed yet — Phase 31 is the first destructive cutover.

### Phase 31 scope (the bulk of pending work)

This is a meaty phase that touches policy/security across many files.
A clean fresh session is recommended.

1. **Rewrite `core/src/policy/compose.ts`** — single rule (no human/agent
   split). Pull values from v2.0 `[security]` block. Apply `trust = []`
   short-circuit before any scan logic.
2. **Add glob matcher** — match `trust` patterns against tap name OR full
   source URL. Use `Bun.Glob` or a small inline matcher.
3. **Remove HTTP registry adapter** — delete `core/src/registry/` and any
   tap config keys/types that referenced HTTP. Migration already errors
   on HTTP taps so users have been warned.
4. **Remove security presets** — drop `PRESET_VALUES`, `SECURITY_PRESETS`
   from the v1.0 schema. Drop `[[security.overrides]]` parsing.
5. **Move semantic to opt-in only** — `scan = "semantic"` in config OR
   `--deep` flag enables Layer 2. Default config never enables it.
   Remove the v1.0 "auto-offer Layer 2 after Layer 1" prompt.
6. **Cut over readers to v2.0 paths** — install/update/remove read state
   via `loadState()` and write via `saveState()`. v1.0 `installed.json`
   and `plugins.json` files are no longer read or written. Hard-error
   the v1.0 detection in cli/src/index.ts (replace soft hint).
7. **Update install/installPlugin/update/remove** to use the new policy
   shape and v2 state. Also wire **manifest writes** here — `install`
   adds entries to `skilltap.toml` + `skilltap.lock` when in a project.
8. **Sync apply** — Phase 29 deferred apply; this is where it lands.
   `sync` calls install/remove/update via the new v2 paths and writes
   the lockfile + state.
9. **Update `composePolicy` callers** in cli/ — drop the human/agent
   branching, surface the trust-list short-circuit.
10. **Test rewrites** — every existing security/policy test needs
    rewriting for the new shape. Some tests will be deleted entirely
    (preset tests, override tests). Migration tests stay.

Estimate: ~25 implementation units. Splittable into 31a (policy + trust
+ remove HTTP) and 31b (cut over readers + install rewrite + apply).

### What's stable enough to build on

- `core/src/sync/{drift,plan}.ts` — already callable; Phase 33 (status)
  and Phase 36 (doctor drift checks) can consume `planSync()` directly.
- `core/src/state/{load,save}.ts` — works against v2 state.json.
- `core/src/manifest/{load,save,lockfile,publish}.ts` — works.
- `core/src/migrate/run.ts` — works end-to-end for v1→v2 upgrade.
- `core/src/plugin-v2/*` — works; .skilltap/ priority detection landed.

### Verification commands when resuming

```bash
bun test packages/core/src/manifest/
bun test packages/core/src/state/
bun test packages/core/src/migrate/
bun test packages/core/src/sync/
bun test packages/core/src/plugin-v2/
bun test packages/core/src/plugin/detect.test.ts
bun test packages/core/src/schemas/config-v2.test.ts
```

All should be green (242+ tests). Run before starting Phase 31 to
confirm clean baseline.

### Watchdog loop

A 30-min cron loop is scheduled in this session (`6972f010`). It dies
when this session exits. Re-arm in the new session per autopilot's
opening steps.

## Completion Summary

(written on completion)
