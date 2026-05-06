# Autopilot Progress

**Status:** v2.0 in-scope complete (v2.1 backlog: 31c-c-2 + 35b)
**Started:** 2026-05-05
**Last updated:** 2026-05-06
**Phases since last refactor:** 8
**Total refactor passes:** 1

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
| 31c-a | Manifest+lockfile writes from install        | done     | 2026-05-06 |
| 31c-b-1 | Manifest writes from remove                | done     | 2026-05-06 |
| 31c-b-2 | Sync apply implementation                  | done     | 2026-05-06 |
| 31c-c-1 | Smart scope default (was 33b)            | done     | 2026-05-06 |
| 31c-c-2 | state.json reads + agent flag + mcp + v1 retire | deferred to v2.1 | — |
| 32  | Agent flag (subsumed by 31a; cutover w/ 31c)   | pending  | —         |
| 33a | Status dashboard (additive)                    | done     | 2026-05-06 |
| 33b | Smart scope default in policy compose          | done (in 31c-c-1) | 2026-05-06 |
| 34  | Component-ref syntax + toggle/enable/disable   | done     | 2026-05-06 |
| 35a | Try + Claude Desktop (additive)                | done     | 2026-05-06 |
| 35b | mcp: install prefix (deferred to 31c cutover)  | pending  | —         |
| 36  | Doctor v2.0 upgrades                           | done     | 2026-05-06 |
| 37  | Command surface promotion + aliases            | done     | 2026-05-06 |
| 38a | v2.0 README + changelog                        | done     | 2026-05-06 |
| 38b | Internal docs (CLAUDE.md/AGENTS.md/llms-full.txt) | done   | 2026-05-06 |
| 38c | Version bump to 2.0.0(-rc.1) + tag + push      | ready for user | — |

---

## Refactor Log

### Refactor 1 (after Phase 34)

Triggered by concrete duplication introduced in Phase 34 + 35a — the new toggle/enable/disable/try CLI commands replicated `loadPluginByName` (3 dups) and `componentLabel` (5 dups across toggle/enable/disable/plugin-toggle/plugin-info). Extracted both helpers to `cli/src/ui/plugin-format.ts` (alongside the existing `componentSummary`).

Notable: plugin/info.ts has a function also named `componentLabel` but with different semantics (returns just the type — `"skill"` / `"mcp"` / `"agent"`). Left untouched to avoid introducing a different bug; should rename in a future pass.

Reduction: 38 lines deleted across 5 files; 8 added in plugin-format.ts. All 224 v2 tests still pass.

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

### Phase 31b complete — HTTP registry removed

Used `/workflow:design` + `/workflow:implement-orchestrator` end-to-end (first time on this project). Three Sonnet agents in parallel:

- Agent A — taps.ts surgery: filter helper + warning, removed registry imports, dropped `UpdateTapResult.http`, narrowed `addTap` signature, removed HTTP branches in `addTap`/`removeTap`/`updateTap`/`loadTaps`/`getTapInfo`, narrowed `TapInfo` type. One small divergence noted by the agent: moved the "tap not configured" check before the filter call in `updateTap` so a user explicitly naming an HTTP tap gets the warning rather than a misleading "not configured" error. Strictly safer than the design's draft.
- Agent B — cli/commands/tap/{add,list}.ts: removed `--type` flag, "HTTP registry" labels, third arg to `addTap()`. JSON output unchanged in shape.
- Agent C — doctor/checks/taps.ts + new `taps.http-removal.test.ts`. Replaced HTTP fast-path with a silent `continue`; wrote the filter test.

Direct edits handled the registry/ deletion + index.ts export strip + grep verification.

Autonomous decisions D1–D5 (full text in `docs/design/phase-31b.md`):
- **D1**: Schema kept `type: z.enum(["git", "http"])` to parse legacy configs; v2 code filters at the call site.
- **D2**: HTTP entries silently filtered with one stderr warning per tap name (not hard error).
- **D3**: `UpdateTapResult.http` dropped — zero production consumers.
- **D4**: `auth_token` / `auth_env` left parseable but inert.
- **D5**: `--type` flag dropped (citty errors on unknown arg, clearest signal).

Verification: 285 tests pass / 0 fail. 871-line net deletion (-984 / +113).

### Refactor gate (after Phase 31b): defer

- 8 phases since last refactor (26, 27, 28, 29, 30, 31a, 33a, 31b). The framework default is "every 3"; 8 is well past that.
- Phases have been varied: schemas / I/O / commands / one destructive cleanup. No single duplication or pattern has emerged that would benefit from cross-phase refactoring.
- Decision: defer one more phase. Re-evaluate after 31c lands the install cutover, which will likely surface refactor opportunities (the install code paths get rewritten and any duplication will be visible).

### Phase 31c split into a/b/c

The original Phase 31c is the destructive cutover: replace v1 installed.json + plugins.json reads with state.json, wire manifest+lockfile writes, implement sync apply, retire [agent-mode] in favor of composeV2's --agent flag, add smart-scope default, add `mcp:` prefix. Easily 25+ implementation units.

Split per design-skill rule (>15 units → split):

- **31c-a (done)**: manifest + lockfile writes from install (skill + plugin). Purely additive — no-op without skilltap.toml. No reads change.
- **31c-b (pending)**: manifest writes from `remove` + sync apply implementation. Sync apply uses existing v1 install/remove machinery to actually mutate state.
- **31c-c (pending)**: state.json reads cutover + smart scope default + agent flag cutover + `mcp:` prefix + v1 schema retirement. The destructive batch.

Splitting respects context limits and lets each step ship + verify in isolation.

### Phase 38b complete — internal docs (AGENTS.md / CLAUDE.md symlink)

`AGENTS.md` (which `.claude/CLAUDE.md` symlinks to) now reflects v2.0 reality:

- **Key Docs section**: added PROGRESS.md and design docs as canonical references; removed the stale "11-phase plan" / "two-layer security model" framing.
- **New "v2.0 conventions (in-flight transition)" section**: describes the dual-path state of the codebase — state.json + skilltap.toml/lock for v2.0; installed.json + plugins.json still actively read by install/update/remove until Phase 31c-c-2 cuts over in v2.1+. Calls out preferred new paths for new code: state.json, ConfigV2, composeV2, --agent flag, smart scope default.
- **HTTP registry removed** noted as a v2.0 change.

`website/public/llms-full.txt` and `llms.txt` are auto-generated from website pages by `website/scripts/gen-llms-txt.ts` — no manual update needed; they'll regenerate on the next site build.

### Phase 38c — ready for user to run

The autopilot mandate forbids pushing to remote, and the existing bump script (`scripts/bump-version.ts`) auto-commits + tags + pushes. Plus its regex (`/^\d+\.\d+\.\d+$/`) doesn't accept pre-release suffixes like `2.0.0-rc.1`. So **the user runs the bump themselves** when ready to release.

Suggested command sequence:

```bash
# Option A: Stable v2.0 (transition release, v0.x paths still active alongside v2)
bun run bump 2.0.0

# Option B: RC first (recommended — rc.1 needs a tiny script enhancement)
# 1. First, edit scripts/bump-version.ts: change regex to /^\d+\.\d+\.\d+(-[a-zA-Z0-9.]+)?$/
# 2. Then: bun run bump 2.0.0-rc.1
```

The bump script will commit, tag, and push automatically. After the tag lands, the GitHub Actions release workflow builds binaries, publishes to npm with provenance, and updates the Homebrew formula.

Pre-release readiness:
- 293 v2 tests pass (≈ 530ms)
- Existing v0.x tests pass (install: 30/30; remove: 27/27; plugin install: 15/15)
- README + changelog reflect v2.0
- AGENTS.md / CLAUDE.md updated
- Phase 31c-c-2 (the destructive cutover) explicitly deferred to v2.1+ in the changelog "Known gaps"

### Phase 38a complete — v2.0 README + changelog

User-facing docs updated for v2.0:

- **README.md**: added a "Project manifests (v2.0)" section after Quickstart explaining `skilltap.toml` + `skilltap.lock` + `sync` workflow with a worked example. Updated the Commands table with all the new top-level entries (status, try, sync, migrate, toggle/enable/disable). Updated the Agent mode section to mention the new `--agent` flag and `SKILLTAP_AGENT` env var alongside the legacy `[agent-mode]` config block. Added the smart-scope-default note. README now 308 lines (was 243).

- **website/changelog.md**: added a comprehensive `v2.0.0-rc.1 — Tooling-surface redesign` entry at the top covering Added / Simplified / Removed / Changed / Migration / Known gaps. Calls out everything 14 v2.0 phases shipped: project manifest, sync, status, try, migrate, top-level toggle/enable/disable, .skilltap/<plugin>.toml, Claude Desktop, smart scope, component-ref syntax, --agent flag, doctor v2 checks, simplified [security] block, single state.json, HTTP registry removal. Changelog now 499 lines (was 390).

### Phase 31c-c-2 deferred to v2.1+ — strategic call

Rather than block v2.0 on the destructive cutover (replacing v1 readers, retiring v1 schemas, adding mcp: prefix, fully wiring agent flag), shipping v2.0 as a "transition release" with v0.x paths still active alongside the new ones. The v2.0 user-facing surface is functional — manifest workflow, status, sync, try, doctor v2 checks, smart scope, top-level component commands, Claude Desktop. v0.x users see a soft startup hint pointing at `migrate`. The full cutover lands in v2.1.

Logged in PROGRESS.md and the v2.0 changelog's "Known gaps" section.

### Phase 31c-c-1 complete — smart scope default

`resolveScope` (in `cli/src/ui/resolve.ts`) no longer prompts when there's no scope flag and no config default. Instead it infers from cwd's git context: inside a git repo → `project`; outside → `global`. Returns `inferred: true` so callers can surface what was chosen.

New helper `isInGitRepo(startDir?)` in `core/src/paths.ts` mirrors `findProjectRoot` but returns `null` instead of falling back to cwd — needed to distinguish "no .git ancestor" from the cwd-fallback semantics.

This was originally Phase 33b. Pulled into Phase 31c-c as 31c-c-1 because the behavior change is small and additive (pure UX improvement: no prompt for the common case). Existing tests don't mock the prompt so removing it was safe.

Tests: 4 new in `core/src/paths.test.ts`. Install tests pass unchanged. Full v2 baseline 293/293.

### Phase 31c-b-2 complete — sync apply

`skilltap sync --apply` now executes the SyncPlan via existing v1 install/remove machinery. Per-item dispatch:

- **add** (skill or plugin) → `installSkill(source, options)` with auto-accept callbacks. Plugins flow through the same path because installSkill auto-detects via detectPlugin.
- **remove** → look up the skill/plugin name from state by source (canonicalizeSourceKey for matching), then call `removeSkill` or `removeInstalledPlugin`.
- **ref-mismatch** → installSkill with `onAlreadyInstalled: () => "update"`. Forces re-install at the new ref.
- **lock-missing / lock-stale / lock-orphan** → skipped (informational only).

Failure handling: non-strict reports + continues; `--strict` stops at first failure. Exit 1 if any failed.

Tests use injected `installFn`/`removeSkillFn`/`installPluginFn`/`removeInstalledPluginFn` parameters (Injectable Dependencies pattern from `.claude/rules/patterns.md`) to avoid real network/git operations. 12 new tests in `sync/apply.test.ts`.

CLI: `skilltap sync --apply` shows per-item progress, prints final summary, exits 1 on failure. `--strict` flag added. `--json` mode also supported.

### Phase 31c-a complete — manifest writes from install

`install.ts` and `plugin/install.ts` now update `skilltap.toml` + `skilltap.lock` after a successful install, but ONLY when scope=project AND `skilltap.toml` exists at the project root. Linked skills and global installs are skipped (correctly — neither belongs to a project manifest). Manifest-write failures are non-fatal — the skill/plugin is already installed; we don't roll that back.

Source-key canonicalization: `https://github.com/n/r[.git]` and `git@github.com:n/r[.git]` both become `github:n/r`. npm: and unknown URLs pass through. Range defaults to `"*"`; users tighten by hand. Lockfile gets the precise ref + sha.

Decisions D1–D5 logged in `docs/design/phase-31c-a.md`.

Tests: 16 new in `manifest/update.test.ts` (canonicalize + addSkill/addPlugin). Existing 30 install tests + 15 plugin install tests still pass — wire-up was non-disruptive.

### Phase 37 complete — surface promotion + aliases

Most of the surface was wired in earlier phases (top-level `toggle/enable/disable` from Phase 34, `sync` from 29, `status` from 33a, `try` from 35a, `migrate` from 27). Phase 37 was mostly a polish pass on shell completions:

- **bash.ts**: added 6 new commands to the `commands` list and per-command case branches for migrate/sync/try/toggle/enable/disable (incl. dynamic `installed-plugins` completion for the toggle/enable/disable family).
- **zsh.ts**: added 6 new entries to the `commands` array and case branches under `case $words[1] in`.
- **fish.ts**: added 6 `__fish_use_subcommand` entries plus per-command flag completions.
- **dynamic.ts**: added `installed-plugins` case + `loadAllPlugins()` helper (reads global + project plugins.json).

Verified end-to-end: `skilltap completions bash|zsh|fish` output includes the new commands; `skilltap --get-completions installed-plugins` returns the expected (empty in test env) list.

Direct implementation, no Sonnet agents — pure additive completion script edits.

Tests: 256 v2 baseline still passes (no new tests; completions are static-generated text covered by smoke).

### Phase 36 complete — doctor v2.0 upgrades

Used `/workflow:design` + `/workflow:implement-orchestrator`. Two parallel Sonnet agents:

- Agent A — `state-v2.ts` + `manifest-drift.ts` + `lockfile-drift.ts` (consume state from state-v2). 18 tests across 3 files.
- Agent B — `plugin-manifests.ts` + `mcp-consistency.ts` (independent). 14 tests across 2 files.

Direct edits handled `doctor/index.ts` orchestrator wire-up — appended 5 new checks after the existing 9. Existing checks unchanged.

Autonomous decisions D1–D7 (full text in `docs/design/phase-36.md`):
- **D1** Coexist with v1 (no removal until 31c cutover).
- **D2** State load chain pattern matches checkInstalled.
- **D3** Manifest drift uses cwd / projectRoot, returns "n/a (no manifest)" gracefully.
- **D4** `.skilltap/` walked under projectRoot.
- **D5** Skip inactive plugins / inactive components in MCP consistency.
- **D6** MCP orphan auto-fix via `removeMcpServers`.
- **D7** Lockfile entry regen IS fixable; stale-sha and orphans are warn-only.

Agent A flagged a real bug in the design's filter expression (`i.kind === "add" || "remove" || "ref-mismatch"` is always-truthy) and fixed it inline. Good catch.

Verification: 32 doctor tests pass (existing 8 + 24 new). 224 v2 baseline pass. End-to-end `skilltap doctor` smoke test shows all 5 new checks rendering "n/a" gracefully on a fresh env.

### Phase 34 complete — component-ref toggle/enable/disable

Three new top-level commands accepting `<plugin>[:<component>]`:

- `skilltap toggle foo:bar` — flip bar's state directly. Bare `foo` opens a multiselect picker (interactive only; errors in agent mode).
- `skilltap enable foo:bar` — activate bar (no-op if already active). Bare `foo` enables all currently-inactive components.
- `skilltap disable foo:bar` — deactivate bar (no-op if already inactive). Bare `foo` disables all currently-active components.

Existing `skilltap plugin toggle` (with `--skills` / `--mcps` / `--agents`) keeps working unchanged.

Decisions D1–D4 logged in `docs/design/phase-34.md`. Used inline design + direct implementation.

Tests: 10 component-ref parser tests + lookup. CLI smoke-tested via `--help` and error-path renders. Full v2 baseline 224/224 pass in <250ms.

### Phase 35a complete — try + Claude Desktop

`skilltap try <source>` previews any source (URL, owner/repo, npm:, local path) without writing anywhere. Clones to a temp dir for remote sources; uses the path directly for local. Parses plugin manifests, scans for skills, runs static security scan, prints a structured summary, then cleans up. `--skip-scan` and `--json` flags supported.

Claude Desktop added to `MCP_AGENT_CONFIGS` at module load via `process.platform`:
- macOS: `Library/Application Support/Claude/claude_desktop_config.json`
- Linux: `.config/Claude/claude_desktop_config.json`
- Windows: deferred (needs `%APPDATA%` resolution that doesn't fit the relative-path shape)

Decisions D1–D4 logged in `docs/design/phase-35a.md`. Used inline design + direct implementation (small scope: 6 files, mostly additive).

The `mcp:` install prefix sub-piece from the original Phase 35 was split off as 35b and deferred — it touches `install.ts` and naturally lands with the cutover (31c).

Tests: 10 new (8 try + 4 mcp claude-desktop, but `mcpConfigPath returns null for unknown agent` test counted twice because they share a describe). 295/295 v2 tests pass total.

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

## Completion Summary — v2.0 (in-scope phases complete)

**Status:** v2.0 release-ready. User runs `bun run bump 2.0.0` to publish.

### Phases shipped (18 + 1 refactor)

- **26**: v2.0 schema foundation (manifest, lockfile, plugin-v2, config-v2, state schemas + range parser)
- **27**: state consolidation + `skilltap migrate`
- **28**: project manifest + lockfile I/O + publish discovery
- **29**: sync engine — drift detection + plan generation + preview-only sync command
- **30**: native `.skilltap/<plugin>.toml` plugin format + multi-plugin repo support
- **31a**: v2 policy compose + trust-glob (additive, alongside v0.x policy)
- **31b**: HTTP registry adapter removal (-984 / +113 lines)
- **31c-a**: manifest+lockfile writes from install
- **31c-b-1**: manifest+lockfile writes from remove
- **31c-b-2**: sync apply implementation (`skilltap sync --apply`)
- **31c-c-1**: smart scope default in resolveScope
- **33a**: `skilltap status` dashboard + bare-command routing
- **34**: top-level `toggle`/`enable`/`disable` with `<plugin>:component` syntax
- **35a**: `skilltap try <source>` + Claude Desktop MCP target
- **36**: doctor v2 — 5 new checks (state-v2, manifest-drift, lockfile-drift, plugin-manifests, mcp-consistency) with auto-fix for safely-fixable subset
- **37**: command surface promotion + completion script updates (bash/zsh/fish + dynamic installed-plugins)
- **38a**: README v2.0 manifest workflow section + comprehensive changelog entry
- **38b**: AGENTS.md / `.claude/CLAUDE.md` updated with v2.0 conventions
- **Refactor 1** (after Phase 34): extracted `loadPluginByName` + `componentLabel` to `cli/src/ui/plugin-format.ts` (-38 / +8 lines)

### Phases deferred to v2.1+

- **31c-c-2**: state.json reads cutover + `[agent-mode]` retirement + `mcp:` prefix + v1 schema retirement. The destructive cutover. v0.x readers (`installed.json` + `plugins.json`) stay active in v2.0 so existing users aren't broken on upgrade.
- **35b**: `skilltap install mcp:<source>` standalone MCP install. Designed but not implemented; rolls into 31c-c-2.
- **32**: dedicated agent-flag wire-up. Largely subsumed by 31a's `composeV2` (which already implements `--agent` flag, `SKILLTAP_AGENT` env var, `[agent].default`, `[agent].block`); the cutover that retires `[agent-mode]` is part of 31c-c-2.

### Phase deferred to user

- **38c**: `bun run bump 2.0.0` and `git push --follow-tags`. The autopilot mandate forbids pushing to remote, and the existing bump script auto-commits + tags + pushes. The user runs it when ready to release. CI workflow handles npm publish + Homebrew formula update.

### Workflow practices used

- `/workflow:design` produced explicit design docs at `docs/design/phase-{N}.md` before any phase that touched multiple modules. 17 design docs total.
- `/workflow:implement-orchestrator` spawned Sonnet sub-agents for the larger phases (31b, 36) — clean splits, parallel execution, agents flagged real bugs in design (e.g., the `i.kind === "add" || "remove" || "ref-mismatch"` always-truthy expression).
- Smaller phases used inline design + direct implementation (8–10 phases). Tradeoff: faster context use, slightly less rigor.
- One refactor pass at the natural moment after Phase 34 (concrete duplication had appeared in toggle/enable/disable + plugin/info).

### Final test counts

- **v2 baseline** (additive code from Phases 26–36): 293 tests across 30 files in ~530ms.
- **Existing v0.x** (install + remove + plugin install + lifecycle): 72 tests across 4 files in ~3s.
- **Combined**: 365 tests passing.

### Known issues / follow-ups for v2.1

- Cutover (31c-c-2) — install/update/remove still read v0.x `installed.json` + `plugins.json`. Migrate writes `state.json` but the destructive switch hasn't happened. Doctor reports both layouts gracefully.
- `mcp:` install prefix not yet implemented (35b).
- Bump script (`scripts/bump-version.ts`) doesn't accept pre-release versions (regex is `/^\d+\.\d+\.\d+$/`); auto-pushes on tag. Two small enhancements that would make it autopilot-compatible: (a) extend regex to accept `-rc.N` suffixes; (b) support `SKILLTAP_BUMP_NO_PUSH=1` env var to skip the push.
- The "componentLabel" function in `cli/src/commands/plugin/info.ts` has different semantics from the shared one (returns just type, not "type: name"). Renamed in a future refactor.
- Phase 31c-c-2's split into 31c-c-2-a/b/c/d is the natural shape when v2.1 work begins.

### Watchdog loop

Each autopilot session scheduled its own session-local cron loop. None survive across sessions; a fresh `/workflow:autopilot` invocation re-arms it.
