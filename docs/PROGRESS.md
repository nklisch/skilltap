# Autopilot Progress

**Status:** Resumed 2026-05-08 for v2.2 (capture) + v2.0 Redesign (Phases 39–46). v2.0/v2.1 cutover work below remains complete.
**Started:** 2026-05-05
**Last updated:** 2026-05-08
**Phases since last refactor:** 2 (44 TUI, 45 migrate)
**Total refactor passes:** 3

## Phase 46 completion summary (2026-05-08)

Polish + docs + release prep shipped in one session. Phase 46.10 (version bump) and 46.11 (release verification) are user-gated and pending.

**What shipped:**

- **docs/UX.md rewritten** — old v2.0/v2.1 surface dropped; new file is a clean canonical CLI reference for the redesign surface only (~550 lines). Covers command tree, flag inventory, prompt matrices, common workflows, and error reference.
- **docs/SPEC.md canonical markers** — added top-level note pointing at `## v2.0 Redesign` as canonical; added `> **Superseded.**` notes on `## CLI Commands` and `## v2.0 — Tooling-Surface Redesign`; changed the redesign section header note from "where this conflicts, this wins" to a clear **Canonical** label.
- **docs/ARCH.md canonical markers** — same pattern: top-level note, `> **Superseded.**` on `## v2.0 Architecture Additions`, **Canonical** label on `## v2.0 Redesign Architecture`.
- **README.md quickstart updated** — `install <source>` → `install skill|plugin|mcp <source>`; `--agent` section replaced with non-interactive-use section using `--yes`/`--json`; Commands table updated to v2.0 redesign surface; Authoring section updated (`link` → `adopt`, `verify` → `doctor`); Gotchas updated with redesign breaking changes.
- **AGENTS.md / .claude/CLAUDE.md** (symlinked) — v2.1 conventions section replaced with v2.0 Redesign conventions covering: typed install surface, no `--agent` flag, flat `[security]` block, no legacy fallback, `Output` interface convention, no silent aliases.
- **website/guide/getting-started.md rewritten** — reflects typed `install skill`, `--scope` / `--yes` / `--json` patterns, `adopt` for local dev, `migrate` instructions.
- **website/reference/cli.md rewritten** — clean canonical reference for the redesign surface; removed commands table at end.
- **website/public/llms-full.txt regenerated** — 131.7 KB, 14 pages (was from 2026-05-07 with old content).
- **packages/cli/src/e2e-v3.test.ts** — 13 new e2e tests covering: `install skill` (project + global), `install plugin`, `toggle plugin <name>:<component>`, `adopt <path>`, `remove plugin`, `migrate` from v0.x config, `status`/`doctor`/`update --check` exit codes. All pass.
- **website/changelog.md** — v2.2.0 entry with full breaking-changes list, migration steps, and what was added.

**Verification:** Full suite 2203 pass / 51 skip / 0 fail (up from 2190; +13 new e2e tests).

**Pending (user-gated):**
- 46.10: `bun run bump 2.2.0` + `git tag v2.2.0` + `git push --follow-tags`
- 46.11: Binary builds, npm publish, Homebrew formula update (triggered by the tag push in 46.10 via GitHub Actions)

---

## Phase 45 completion summary (2026-05-08)

Migrate command verification + polish shipped in one commit (`0024038`). Most translation logic was already in place from Phases 27 (state migration) and 40 (config translation); Phase 45 adds the missing post-migration verification.

**What shipped:**
- `runMigrate()` now runs `runDoctor()` after non-no-op migrations. Result included in `MigrationReport.doctorReport` (optional field).
- `runMigrate()` parses `skilltap.toml` (if present at projectRoot) and adds a warning to `MigrationReport.warnings` if it doesn't parse cleanly. Migration still succeeds — manifest issue is informational.
- `commands/migrate.ts` surfaces doctor findings: green checkmark on all-pass; red `✗` on failures with hint to run `skilltap doctor`; yellow `!` on warnings. JSON output mode includes `doctorReport` in payload.
- 3 new tests:
  - No-op path doesn't include `doctorReport` (early-return cleanliness).
  - End-to-end fixture: full v0.x setup with installed.json + plugins.json + v1 config (all 4 legacy block types: `[security.human]`, `[security.agent]`, `[agent-mode]`, `[[security.overrides]]`). Asserts state.json version/skills/plugins, config flatness, `.v1.bak` files, doctorReport defined.
  - Malformed skilltap.toml produces warning but migration succeeds.

**Verification:** Full suite 2190 pass / 51 skip / 0 fail (up from 2187; +3 tests).

**Workflow:** Single Sonnet agent — Phase 45 was small enough to handle in one focused run.

---

## Phase 44 completion summary (2026-05-08)

TUI dashboard shipped across 4 implementation agents and 5 commits (`c74610d` Spike, `9b23393` state, `ea6a1c4` components, `ba8f9cb` integration). Suite up to 2187 pass / 51 skip / 0 fail.

**What shipped:**
- **Spike Unit (Unit 0)** — `c74610d`. Validated Ink-on-Bun stability before any production code committed. ink@7.0.2, react@19.2.6, ink-testing-library@4.0.0 installed; PTY-driven smoke test confirms clean exit on `q` and Ctrl+C; terminal restored cleanly. Phase 41's "riskiest assumption" cleared.
- **State machine** (`packages/cli/src/tui/state/`) — pure types + per-screen reducers (Dashboard, Find, Toggle, Adopt) + root reducer + `keys.ts` registry. 55 unit tests cover every action type and boundary case. No Ink imports anywhere — fully testable in isolation.
- **Ink components** (`packages/cli/src/tui/screens/`) — 4 screen renderers + 4 shared components (`Tabs`, `List`, `DetailPane`, `Footer`). Pure components: state + dispatch as props, no internal state. 43 snapshot tests via `ink-testing-library`.
- **App root** (`packages/cli/src/tui/App.tsx`) — `useReducer(appReducer)` + global `useInput` for navigation/exit + per-screen key dispatch + `useEffect` data loading with cancellation flag.
- **Integration** (`packages/cli/src/tui/index.ts` + `context.ts` + bare-skilltap routing) — `mountTui()` entry + `AppContext` factory wires TUI dispatchers to existing core functions (Phase 39 capture, Phase 42 typed install/remove, Phase 43 adopt). 4 PTY smoke tests pass reliably.
- Bare `skilltap` (TTY) opens dashboard; piped/non-TTY errors with hint to use `skilltap status`.

**Workflow:** Spike (1 agent) gated → 3 sequential implementation agents (state, components, integration). Pattern from Phase 41's design carried forward — pure reducers in core, Ink adapters in CLI.

**Deferred follow-ups (functional but minimal):**
- `loadFindResults` returns `[]` — Find screen renders the search box and accepts input, but no actual search results yet. Wire to tap-registry search in a follow-up.
- `dispatchSync` returns "not yet implemented" — sync trigger isn't exposed in any screen yet.
- Dashboard "Updates" tab returns `[]` — wire to `checkForUpdates` in a follow-up.

These are intentional Phase-44 cuts; the dashboard scaffolding is sound and the data wiring is straightforward to complete.

---

## Refactor 3 (after Phase 43, 2026-05-08)

Triggered by 5 phases since Refactor 2. Plan in `docs/designs/completed/refactor-after-phase-43.md` (10 steps); 9 executed, 3 skipped with rationale (one orchestrator agent committed all 9). Net ~250 lines consolidated across the codebase.

**What landed (9 commits, hashes f92bf4a → b71579e):**
- `setupOutput(args)` helper extracted to `cli/src/ui/setup.ts`. ~46 `createOutput()` boilerplate sites collapsed to one helper call (`f92bf4a`).
- `Bun.$` replaces the lone `node:fs/promises` `rm` in `plugin/capture.ts` (`c94fb14`).
- `resolveScope(args, config)` helper used consistently — inline ternaries in adopt/status/move/info replaced (`4941fd0`). Also fixed `ReturnType<typeof createOutput>` annotations to `Output` directly.
- `remove/shared.ts` mirrors `install/shared.ts` — three remove handlers now share `setupRemoveContext()` (`d062a4b`).
- `pickOne()` helper in `ui/picker.ts` consolidates clack picker boilerplate; toggle.ts and adopt.ts migrated (`a31ad55`).
- Three rename commits with deprecated aliases for one cycle: `installMcpOnly` → `installMcp` (`04d7984`), `removeMcpInstall` → `removeMcp` (`19c84ed`), `adoptAgentPlugin` → `adoptPlugin` (`7f3b01d`).
- Deprecated aliases removed (`b71579e`). Final canonical names only.

**Skipped with rationale (planning audit was based on stale information):**
- **Step 7** (delete `onDeepScan`) — IS actually invoked in `scanner.ts:198-199` and threaded through `install.ts:763`. Plan's "never invoked" claim was wrong.
- **Step 8** (migrate 3 plugin tests to `createTestEnv()`) — they already use `createTestEnv()` for state isolation; remaining `mkdtemp()` calls are for fixture content directories (a different concern). No migration needed.
- **Step 3** (`install/mcp.ts` → `setupInstallContext()`) — `setupInstallContext()` runs interactive prompts (agent selection, intro, builtin tap check, selectAgents) that don't belong in MCP install. The current MCP file already uses the appropriate shared helpers.

**Verification:** Full suite 2085 pass / 51 skip / 0 fail.

**Workflow:** One Sonnet agent executed all 9 steps sequentially, committing per step. Orchestrator (Opus) wrote the plan and verified final state.

---

## Session-end handoff (2026-05-08, end-of-day)

Five phases shipped end-to-end this session: 39 (capture), 40 (cleanup), 41 (output abstraction), 42 (typed install/remove/update/toggle), 43 (Claude Code adoption). Codebase state at `e15f2fa`:
- Full suite: 2085 pass / 51 skip / 0 fail.
- Lines of production code: roughly **net –4000** since session start (phase 40+41+42 were all heavy demolitions).
- CLI surface count: 51 endpoints (v2.1) → ~19 top-level entries (v2.0 redesign).

**Remaining work** (3 phases + 1 deferred refactor pass):
- **Refactor pass** — gate triggered (5 phases since Refactor 2). Recommended before Phase 44 since the TUI work will lean on existing CLI command patterns; refactor pass surfaces duplication early. Use `/refactor-design` → `/implement-orchestrator` → `/extract-patterns`.
- **Phase 44 (TUI dashboard)** — largest remaining. Adds Ink, multi-screen flow (dashboard, find, toggle, adopt). 10 ROADMAP units (44.1–44.10). Best in fresh context.
- **Phase 45 (migrate command)** — medium. Most translation logic already shipped via Phase 40's incremental migrate updates. Phase 45 verifies all v2.x → redesign edge cases + adds doctor-post-migrate verification.
- **Phase 46 (polish + release)** — docs alignment + version bump (user-gated).

**Resume conditions for next /autopilot session:**
1. Refactor gate first: `/refactor-design` to find duplication across the 5-phase reshape, then implement-orchestrator + extract-patterns.
2. Phase 44 next (TUI dashboard) — big addition; fresh context recommended.
3. Phase 45 + 46 to finish the v2.0 redesign.

**User-gated releases pending:**
- v2.2.0 (capture, Phase 39 done): `bun run bump 2.2.0` + `git tag v2.2.0` + `git push --follow-tags`.
- v2.2.0 (full redesign, after 44+45+46): subsequent.

**Watchdog loops still active:** `fceab167` (30m nudge) and `858edc08` (3h re-engagement). They resume from PROGRESS.md on each fire.

**Stashed pre-existing user WIP:** still at `stash@{0}` ("WIP: security flatten + security.enabled kill switch (pre-autopilot v2 redesign)"). The security flatten part is now redundant (Phase 40 did it). The `security.enabled` kill switch is a fresh addition the user may want to bring forward — `git stash pop` and pick what's still relevant.

---

## Resume context (2026-05-08)

A foundation-doc redesign session produced two major commits on `main`:
- `dd4e112` — v2.0 redesign foundation docs (VISION/SPEC/ARCH/UX/ROADMAP/SECURITY).
- `53dd0e1` — split plugin capture into v2.2 (Phase 39) ahead of redesign (Phases 40–46).

Phase 39 implementation completed in this session (commits `7cc6e71` through `a4eb6a0`). Phases 40–46 are pending. Pre-existing user WIP touching security flatten + `security.enabled` kill switch was stashed for clean autopilot work (`stash@{0}` — "WIP: security flatten + security.enabled kill switch (pre-autopilot v2 redesign)").

User explicitly enabled watchdog loops mid-session (CronCreate jobs `fceab167` for 30m nudge, `858edc08` for 3h re-engagement at :07).

**Decision log for this session:**
- Stashed pre-existing user WIP rather than incorporating: WIP was partial (kept `human`/`agent` as deprecated rather than deleting) and would conflict with Phase 40 cleanup. User can `git stash pop` later to resume.
- Used existing `docs/designs/completed/plugin-capture.md` as the design source for Phase 39 instead of generating a fresh design — the document already covered all 12 ROADMAP units in detail.
- Phase 39 Units 1–3 + 5–7 implemented directly by Opus (small, self-contained units). Unit 4 (CLI rendering) orchestrated to a Sonnet agent. **User feedback mid-phase: prefer orchestration via `/implement-orchestrator` for all future implementation work.** Phases 40–46 will run the full autopilot workflow: research gate → `/design` → `/implement-orchestrator` → test checkpoint → progress + commit → refactor gate.

## Phase 43 completion summary (2026-05-08)

Claude Code plugin adoption shipped across two implementation agents and 2 commits. The `agent-plugins/` framework is the new pluggable port for any agent that ships a plugin system; the Claude Code adapter is the first concrete implementation.

**What shipped:**
- `packages/core/src/agent-plugins/` — port + adapters layout:
  - `types.ts` — `AgentPluginScanner` interface (`name`, `detect()`, `scan()`) + `DiscoveredAgentPlugin` type (scanner-tagged plugin record).
  - `claude-code.ts` — concrete scanner. Reads `~/.claude/plugins/installed_plugins.json` and `~/.claude/plugins/known_marketplaces.json` with tolerant Zod schemas (`.passthrough()` everywhere). Walks each plugin's `installPath` via existing `detectPlugin()`. Maps Claude Code's user/local scopes to skilltap's global/project. Optional `overrideEnv` parameter for test isolation.
  - `codex.ts` — stub (Codex has no marketplace; `detect()` returns false).
  - `registry.ts` — `defaultScanners()` + `scanAllAgentPlugins()` (fail-soft: per-scanner errors bubble up as `scannerErrors[]`, loop continues).
- `packages/core/src/adopt.ts` extended:
  - `adoptSkillFromPath(path, options)` — replaces what `link` did pre-Phase-42. Track-in-place (default) symlinks the path; `--move` relocates the dir.
  - `adoptAgentPlugin(plugin, options)` — adds a `state.plugins[]` entry for a Claude Code plugin. Doesn't copy or move files; `record.path = installPath` points at Claude Code's cache. Marker convention: `record.repo` starts with `claude-code:` for adopted plugins.
  - `discoverAllAdoptable(options)` — combines unmanaged skills (existing `discoverSkills`) + agent plugins (new `scanAllAgentPlugins`).
- `packages/core/src/doctor/checks/claude-code-overlap.ts` — doctor check #17. Warns when a Claude Code plugin name overlaps with a skilltap-installed standalone skill or non-adopted plugin.
- `packages/cli/src/commands/adopt.ts` rewritten:
  - `adopt` (TTY, no args) — clack picker over unmanaged skills + Claude Code plugins.
  - `adopt <path>` — external path (path detection: `./`, `/`, `~/`, absolute, or contains `/`).
  - `adopt <name>` — preserved existing behavior, now also matches Claude Code plugins by name.
  - `adopt --source claude-code` — picker filtered to one scanner.
  - `adopt --move` flag for path mode; default is track-in-place.
- `PluginRecord` schema gained a `path: string | null` field for adopted plugins to record `installPath`.

**Verification:** Full suite 2085 pass / 51 skip / 0 fail (up from 2043; +42 new tests). Agent A added 20 agent-plugins tests + 6 adopt extension tests + 8 doctor check tests; Agent B added 7 CLI subprocess tests.

**Workflow:** Two Sonnet agents — Agent A (Units 1-4+6: full core surface), Agent B (Units 5+7: CLI + tests). Orchestrator (Opus) verified intermediate gates and final suite.

**Out of scope for Phase 43 (deferred follow-ups):**
- Auto-symlinking adopted plugin skills into other agent dirs (cursor, codex). User opts in later via `--also` or future commands.
- `--also-uninstall` for `skilltap remove plugin <claude-adopted>` (would shell out to Claude Code's `/plugin uninstall`).
- Full TUI picker UX (Phase 44).
- Adopting Cursor extensions, Gemini plugins, etc. (no concrete schemas yet; framework is ready).

## Phase 42 completion summary (2026-05-08)

Typed CLI surface shipped across three implementation agents and 5 commits. The 51-endpoint surface from v2.1 is collapsed to 19 top-level entries with consistent typed semantics for install/remove/update/toggle.

**What shipped:**
- `install <type> <source>` — citty subcommand group (skill | plugin | mcp). New directory `cli/commands/install/` with `index.ts`, `skill.ts`, `plugin.ts`, `mcp.ts`, `shared.ts`. Original `commands/install.ts` deleted.
- `remove <type> <name>` — citty subcommand group. New directory `cli/commands/remove/` mirroring install.
- `update [type] [name]` — optional positional. Bare = update everything; `update skill` = all skills; `update skill <name>` = one. Plugin/MCP types validate but are stubbed with "not yet implemented" (`updatePlugin` / `updateMcpServer` core helpers deferred to a follow-up unit).
- `toggle [type] [name[:component]]` — optional positional. Bare opens a clack picker (Phase 44 will replace with Ink TUI). MCP toggle stubbed (StoredMcpStandalone schema lacks an `active` field; deferred).
- `mcp:` URL prefix removed from user input. Internal state-storage convention (`state.mcpServers[].source` may begin with `mcp:` for legacy entries) is preserved.
- `tap install` deleted entirely.
- 23 files deleted: `commands/skills/` directory (11 files), `commands/plugin/` directory (4 files), `enable.ts`, `disable.ts`, plus orphaned tests.
- 3 new top-level commands: `info <name>` (auto-detects type from state), `adopt <name>` (lifted from `skills/adopt.ts`), `move <name>` (lifted from `skills/move.ts`).
- `status` extended with `--unmanaged`, `--disabled`, `--active` filter flags (absorbed from the deleted `skills` listing).
- `InstallOptions` callbacks reduced 17 → 10 by merging similar shapes (`onWarnings(warnings, kind)` and `onConfirmInstall(kind, manifest?)` now cover skill + plugin paths). Dropped: `onStaticScanStart`, `onSemanticScanStart`, `onSemanticProgress`, `onOfferSemantic` (replaced by `options.out?.progress(...)`).
- Phase 39 capture callbacks (`onPluginCaptureConfirm`, `onPluginCaptureConflict`) preserved unchanged.

**Verification:** Full suite 2043 pass / 51 skip / 0 fail (down from 2109 — net –66 tests, all from deleted alias/skills/plugin/enable/disable test files; coverage of underlying logic preserved).

**Workflow:** Three Sonnet agents — Agent A (Units 1+2+7: foundation), Agent B (Units 3+4+5+6: update/toggle/mcp/tap), Agent C (Units 8+9+10+11+12: deletions + top-level + tests). Orchestrator (Opus) verified intermediate gates and final suite.

**Deviations:** Two stubs noted for follow-up:
- `updatePlugin` / `updateMcpServer` core helpers — emit "not yet implemented" message; full re-install logic deferred.
- MCP standalone toggle — `StoredMcpStandalone.active` field doesn't exist; deferred to a follow-up.

## Phase 41 completion summary (2026-05-08)

Output mode abstraction shipped across two implementation agents and 5 commits. Ports & Adapters split: `Output` interface in `packages/core/src/output/` (port), three adapters in `packages/cli/src/output/` (tty wrapping format.ts + clack, plain stripping ANSI, json emitting NDJSON).

**What shipped:**
- `Output` interface with 8 methods (info/warn/error/success/block/json/progress/raw) and `Progress` handle (update/succeed/fail/pause/resume).
- `pickMode()` resolver: explicit `--json` > TTY detection > plain.
- Three adapters: tty (clack + ANSI), plain (no colors, no spinners), json (NDJSON one event per line).
- `createOutput(opts)` factory dispatches on pickMode.
- `CaptureOutput` test helper records events in-order — no subprocess needed for unit tests.
- Per-command Zod schemas: 5 with full discriminated unions (install/update/sync/doctor/status), 11 placeholder for follow-up tightening.
- ~57 CLI command files migrated off `successLine`/`errorLine`/`infoLine`/`jsonLine`/`securityBlock` and direct `process.stdout/stderr.write`. Spinner imports collapsed to `out.progress()`.
- `format.ts` slimmed: write functions removed (only pure formatting utilities remain like `ansi`, `table`, `formatLineRef`).
- New `output/write-helpers.ts` for the few pre-initialization fatal-error paths in `ui/policy.ts` / `ui/resolve.ts`.

**Verification:** Full suite 2109 pass / 51 skip / 0 fail (up from 2038; 72 new output unit tests). Source-side acceptance checks: zero direct stdout/stderr writes outside `output/` and `completions/dynamic.ts`; zero format.ts write-function imports in commands.

**Workflow:** Two Sonnet agents — Agent A (Units 1-8: foundation + schemas), Agent B (Unit 9: 3-tier CLI migration + Unit 10: test rewrites). Orchestrator (Opus) verified intermediate gates and final suite.

## Phase 40 completion summary (2026-05-08)

Agent-mode runtime collapsed and v0.x state fallback paths removed across two implementation agents (`f0286a3` Units 1-5 + `b687d31` Unit 6). Net –2200+ production LOC.

**What shipped:**
- Flat `[security]` config schema (no per-mode `human`/`agent` split, no `[agent-mode]` block).
- `composePolicy` returns single non-branching `EffectivePolicy` (no `agentMode` field).
- 22 CLI call sites converted from `agent-out.ts` helpers to `format.ts` helpers (errorLine/successLine/infoLine + new `securityBlock` and `jsonLine` in format.ts).
- `runAgentMode`/`runInteractiveMode` collapsed to single `runInstall`/`runUpdate`.
- `loadInstalled`/`loadPlugins` read state.json only — v0.x fallback paths removed.
- Migrate command's `[agent-mode].scope → defaults.scope` translation added.

**Files deleted:**
- `packages/cli/src/ui/agent-out.ts` (+ test)
- `packages/core/src/agent-env.ts`
- `packages/cli/src/commands/config/agent-mode.ts` (+ test)
- `packages/cli/src/commands/update.agent-mode.test.ts`
- `packages/cli/src/ui/policy.test.ts`

**Verification:** Full suite 2038 pass / 51 skip / 0 fail (down from 2102 before phase — net –64 tests, all from deleted agent-mode-specific tests; underlying logic coverage preserved by rewritten `policy.test.ts` and `schemas/config.test.ts`).

**Workflow note:** Used the full extend → design → implement-orchestrator → test pattern per user direction mid-Phase-39. Two Sonnet agents handled the implementation; orchestrator (Opus) verified intermediate gates and ran final suite.

## Phase 39 completion summary (2026-05-08)

Plugin capture shipped end-to-end across 5 commits. The release (39.11/39.12) is gated on user running `bun run bump 2.2.0`.

**Code shipped:**
- `core/src/plugin/capture.ts` — `detectCaptureMatches` (pure), `applyCapture` (atomic), `mergeBuckets`, `buildCrossSourceHint`. Source-aware partitioning via `canonicalizeSourceKey`.
- `core/src/plugin/install.ts` — capture detection step between scan and placement; `onCaptureConfirm` + `onCaptureConflict` callbacks; `captured` + `forcedCrossSource` on `PluginInstallResult`.
- `core/src/install.ts` — `onPluginCaptureConfirm` + `onPluginCaptureConflict` threaded through to all 3 `installPlugin` call sites; `captured` mirrored on `InstallResult`.
- `core/src/sync/apply.ts` — same-source auto-confirm, cross-source hard-fail (defends teammate sync against silent substitution).
- `core/src/doctor/checks/capture-collisions.ts` — defensive canary check #16.
- `cli/src/ui/install-callbacks.ts` — `printCaptureConflict` + `printCaptureSummary` rendering helpers.
- `cli/src/commands/install.ts` — interactive prompt flow (select for conflict, confirm for capture); agent-mode safe defaults; post-install `Captured N skill(s), M MCP server(s)` summary.

**Tests added:**
- `core/src/plugin/capture.test.ts` — 26 unit tests covering detection partitioning + apply side-effects.
- `core/src/plugin/install.capture.test.ts` — 9 integration tests covering installPlugin end-to-end with state pre-seeded.
- `core/src/sync/apply.test.ts` — 1 test verifying capture callbacks threaded through with correct defaults.
- `core/src/doctor/checks/capture-collisions.test.ts` — 5 tests for canary check.
- `cli/src/commands/install.capture.test.ts` — 5 CLI subprocess tests (agent-mode same-source + cross-source paths, plain-mode capture summary).

**Verification:** Full suite 2102 pass / 0 fail (up from 2091 pre-Phase-39).

**Released artifacts gated on user:**
- 39.11: `bun run bump 2.2.0` + `git tag v2.2.0` + `git push --follow-tags`.
- 39.12: Release workflow verification (binaries, npm publish, Homebrew formula update) — blocked on 39.11.

## ⚠ Earlier autopilot blocker (2026-05-06)

The next pending roadmap step (Phase 38.7) is the version bump. The bump script now supports `SKILLTAP_BUMP_NO_PUSH=1` (post-cutover doc-audit improvement) — autopilot can technically stage the commit + tag locally without pushing, but **picking the release version is still user-owned**: the user decides whether to ship 2.0.0, 2.0.0-rc.1, or fold further work in first. The autopilot mandate also forbids the eventual `git push --follow-tags` regardless.

After 38.7, Phase 38.8 (release workflow verification) and Phase 31c-c-2d-2-final (v2.2 cleanup, needs release window first) become unblocked.

Until the user decides on a version and runs the bump, no further autopilot phases can make progress. The codebase is fully ready: 0 lint errors, 1608/1608 tests pass, foundation + website docs aligned with v2.1, ROADMAP.md visually consistent, v2.1 changelog drafted at `website/changelog.md`.

**Resume conditions for a fresh autopilot session:**
1. After user runs the bump → Phase 38.8 release-workflow verification can be checked.
2. After release window passes → Phase 31c-c-2d-2-final (delete v0.x read-fallback, retire `[agent-mode]` schema) becomes safe.
3. New roadmap added → Tackle a deferred item from the "Deferred (no scheduled version)" list (Windows support, VS Code extension, etc.) which would warrant its own design doc.

**How to run the bump:**
- Standard release: `bun run bump 2.0.0` (commits, tags, pushes — no env var needed).
- Stage-and-review: `SKILLTAP_BUMP_NO_PUSH=1 bun run bump 2.0.0` (commits + tags locally, prints `git push --follow-tags` for the user to run).

**Latest commit at block:** `9e3337e` "Security docs: surface --agent flag + env-var as agent-mode entry points"

## Post-cutover documentation audit (2026-05-06, second autopilot pass)

A second autopilot pass after the initial v2.1 cutover ran a `~25 commits` deep audit of foundation docs, SPEC, and website docs against the actual shipped binary. Pattern: `< grep code for declared flags / schema → diff against doc claims → fix discrepancies >`.

The big find: **the v2.0 redesign block in SPEC + SECURITY + VISION + website/changelog all carried original-design intent as if it were shipped reality**. Phase 31c-c-2c took simpler paths (extended v1 `composePolicy` + kept v0.x per-mode security + kept `[agent-mode]` block instead of replacing it with `[agent]` block), but four sections in SPEC and one each in SECURITY/VISION/changelog still described the never-built features as if live.

**Categories of fixes (count of commits):**
- v2.0-design-vs-shipped reconciliation: 5 commits (SPEC 4 sections, SECURITY 1, VISION 1, changelog 1)
- Per-command flag-table omissions: 7 commits (`install --agent`, `update --semantic/--json/--agent`, `tap install --agent`, `skills info --json`, `skills remove --agent + mcp:`, `skills (list) --disabled/--active`, `verify --all`, `tap remove --yes`, `tap list --json`, `skills link --global`, `plugin toggle --json`, `plugin remove --json`)
- Phantom commands removed: 1 commit (SPEC's `### skilltap tap update` documented a non-existent subcommand)
- Wrong default-output examples: 2 commits (doctor 9-check table → 15-check; status v0.x agent-reporter → v2.0 dashboard)
- Wrong invocation behavior: 3 commits (sync flag set, migrate startup-detection wording, install smart-scope)
- Missing SPEC sections for shipped commands: 1 commit (toggle/enable/disable/self-update added)
- Missing config subcommand sections: 1 commit (config security/telemetry/edit added)
- HTTP-tap historical-flagging in foundation docs: 1 commit (ARCH module descriptions, SPEC HTTP-Registry section)
- `installed.json`/`plugins.json` → `state.json` operational claims: 1 commit (~12 spots in SPEC behavior blocks)

**Verification at end of audit:** 41/41 focused tests pass (CLI policy unit + install + skills/remove + doctor subprocess). No code changes, only docs + the bump-script env-var enhancement from earlier in the run.

**What's now consistent across all docs:**
- v2.1 has per-mode `[security.human]`/`[security.agent]` blocks (not collapsed)
- `on_warn` enum is `prompt|fail|allow` (not `install`)
- Trust mechanism is `[[security.overrides]]` (not glob `trust = []`)
- `[agent-mode]` block is current (not replaced by `[agent]`)
- Agent-mode entry points: `--agent` flag > `SKILLTAP_AGENT=1` env > `[agent-mode] enabled` (precedence chain)
- `state.json` is canonical (with v0.x fallback)
- `skilltap tap update` is NOT a subcommand — auto-refresh via `skilltap update`
- 15 doctor checks, not 9

## Session completion summary (2026-05-06)

After ~85 commits across two days, the v2.0 → v2.1 cutover is feature-complete and the codebase is in genuinely excellent shape:

**Code quality**
- Lint: 104 errors / 279 warnings (start) → **0 / 0** (end). Achieved via biome safe-fix pass (277 files), per-rule unsafe-fix passes for `noUnusedImports` / `noUnusedVariables` / `useTemplate`, file-level overrides for legitimate intentional patterns (TTY interop, ANSI escapes), and per-line `biome-ignore` comments documenting every remaining `!` non-null assertion with its runtime guard.
- Tests: **1608/1608 pass** (50 intentional skips for network-conditional / API-key tests). macOS `/tmp` → `/private/tmp` symlink bug in `makeTmpDir` fixed mid-session, resolving 6 long-standing failures.
- CLI builds clean: 603 modules, 2.97 MB.

**Code shipped**
- v2.0 phases 26–38 + 38d e2e test
- v2.1 cutover phases 31c-c-2a/b/c/d-1/d-2-orphan + 31c-c-2c-flag (per-command `--agent` wiring)
- Refactor 2: `agent-env.ts` (single-source `SKILLTAP_AGENT` check) + `dirs.ts` (leaf module breaks `config.ts` ↔ `state/` import cycle)
- Net –355 lines from removing the dual-write scaffolding (sync-from-v1.ts, read-bridge.ts) once state.json became canonical

**Documentation**
- All foundation docs (README, AGENTS, ARCH, VISION) reflect v2.1 reality
- All website guides (what-is, getting-started, installing-skills, configuration, doctor, taps, teams, shell-completions, security) updated for v2.1
- All website reference docs (cli, config-options, tap-format, skill-format) audited
- llms-full.txt regenerated (166 KB) for AI-assistant ingestion
- Specifically corrected: HTTP registry tap claims, `--agent` flag/env-var entry points, `installed.json`/`plugins.json` → `state.json`, doctor v2 check coverage
- New v2.0 surfacing: project manifest workflow in teams guide, `skilltap status` in getting-started, key features in "What is skilltap?"
- CLI hint at `skilltap config set agent-mode.enabled` corrected to point at all three entry points

**Remaining for release**
- v2.0 / v2.1 version bump (gated on user; autopilot mandate forbids `bun run bump`)
- Phase 31c-c-2d-2-final (delete v0.x read-fallback paths, drop `[agent-mode]` from ConfigSchema): explicitly deferred to v2.2 — needs a release window for users to run `skilltap doctor --fix` and clear orphans.

**v2.0 Final verification (2026-05-06):** 349 v2 core tests + 18 CLI e2e tests pass. `skilltap doctor` runs all 14 checks (9 v1 + 5 v2) end-to-end in a clean env.

**v2.0 release ready:** `bun run bump 2.0.0` + `git tag v2.0.0` + `git push --follow-tags`. (User runs the bump.)

**v2.1 progress:** Phase 31c-c-2a (state.json dual-write) shipped. Remaining cutover work (read-side, `[agent-mode]` retirement, v0.x schema deletion) tracked as 31c-c-2b/c/d.

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
| 31c-c-2a | state.json dual-write from saveInstalled/savePlugins | done | 2026-05-06 |
| 31c-c-2b | state.json reads cutover (install/update/remove + plugin) | done | 2026-05-06 |
| 31c-c-2c | --agent / SKILLTAP_AGENT precedence over config block | done | 2026-05-06 |
| 31c-c-2d-1 | state.json canonical store; installed.json/plugins.json no longer written | done | 2026-05-06 |
| 31c-c-2d-2-orphan | doctor check + --fix for v0.x file orphans | done | 2026-05-06 |
| 31c-c-2c-flag | --agent flag wired into per-command args (install/update/remove/tap install/skills enable+disable) | done | 2026-05-06 |
| 31c-c-2d-1-fix | doctor 'installed' check reads state.json post-cutover | done | 2026-05-06 |
| 31c-c-2d-1-msg | doctor messages + code comments updated 'installed.json' → 'state.json' | done | 2026-05-06 |
| 31c-c-2d-2-final | v0.x schema + read-fallback deletion (final cleanup) | deferred to v2.2 | — |
| 32  | Agent flag (superseded by 31c-c-2c)            | superseded | 2026-05-06 |
| 33a | Status dashboard (additive)                    | done     | 2026-05-06 |
| 33b | Smart scope default in policy compose          | done (in 31c-c-1) | 2026-05-06 |
| 34  | Component-ref syntax + toggle/enable/disable   | done     | 2026-05-06 |
| 35a | Try + Claude Desktop (additive)                | done     | 2026-05-06 |
| 35b-1 | mcp: install prefix (install side)           | done     | 2026-05-06 |
| 35b-2 | mcp: remove handling                         | done     | 2026-05-06 |
| 36  | Doctor v2.0 upgrades                           | done     | 2026-05-06 |
| 37  | Command surface promotion + aliases            | done     | 2026-05-06 |
| 38a | v2.0 README + changelog                        | done     | 2026-05-06 |
| 38b | Internal docs (CLAUDE.md/AGENTS.md/llms-full.txt) | done   | 2026-05-06 |
| 38d | v2.0 end-to-end test (38.5)                    | done     | 2026-05-06 |
| 38c | Version bump to 2.0.0(-rc.1) + tag + push      | ready for user | — |
| 39  | Plugin Capture (v2.2)                          | code complete; 39.11/39.12 user-gated | 2026-05-08 |
| 40  | Drop legacy fallbacks + agent-mode             | done     | 2026-05-08 |
| 41  | Output mode abstraction                        | done     | 2026-05-08 |
| 42  | Typed install/remove/update/toggle             | done     | 2026-05-08 |
| 43  | Claude Code plugin adoption                    | done     | 2026-05-08 |
| 44  | TUI dashboard (Ink)                            | done     | 2026-05-08 |
| 45  | Migrate command rewrite                        | done     | 2026-05-08 |
| 46  | Polish + docs + release                        | code complete; 46.10/46.11 user-gated | 2026-05-08 |

---

## Refactor Log

### Refactor 2 (after Phase 31c-c-2d-1)

Triggered by 15 phases since Refactor 1, plus concrete duplication left over from the v2.1 cutover.

**Consolidation 1: `SKILLTAP_AGENT === "1"` check.** Phase 31c-c-2c added the env-var check in 6 places across 4 files (cli/src/index.ts × 3, cli/src/ui/policy.ts, core/src/policy.ts × 2). Extracted to `core/src/agent-env.ts::isAgentEnv()`. Single source of truth; all 6 sites now call `isAgentEnv()`. The literal `"1"` lives in only one place.

**Consolidation 2: `getConfigDir` + `ensureDirs` extracted to leaf module.** These were defined in `config.ts` but needed by `state/save.ts`, `state/paths.ts`, and `plugin/state.ts` — creating a circular import that Phase 31c-c-2d-1 worked around with dynamic `await import("./state/load")` calls inside `saveInstalled`/`loadInstalled`. Moved both helpers to a new leaf module `core/src/dirs.ts` with no internal dependencies. `config.ts`, `state/*`, and `plugin/state.ts` now import statically. Re-exported from `config.ts` so external consumers keep working.

Result: `saveInstalled`, `loadInstalled`, `savePlugins`, `loadPlugins` use static imports — cleaner stacks, no module-load-time surprises. `core/src/index.ts` now exports `agent-env` and `dirs` modules so external consumers (CLI, future plugins) can use them.

Files added:
- `packages/core/src/agent-env.ts` — `isAgentEnv()` helper.
- `packages/core/src/dirs.ts` — `getConfigDir`, `ensureDirs` (relocated from config.ts).

Files touched: 7 (config.ts, policy.ts, state/save.ts, state/paths.ts, plugin/state.ts, cli/src/index.ts, cli/src/ui/policy.ts) plus 1 test (config.test.ts: assertion now checks state.json instead of installed.json after the canonical-store cutover).

Reduction: 26 lines of duplication consolidated; net minimal line count change but **6 inline literal "1" checks → 1**, **2 dynamic imports → 0**, **1 import cycle → 0**.

Tests: 523 pass across 41 files. CLI builds cleanly (603 modules, 2.97 MB).

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

Autonomous decisions D1–D5 (full text in `docs/designs/completed/phase-31b.md`):
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

### Phase 35b-1 complete — `skilltap install mcp:<source>`

Started v2.1 work. Phase 35b (mcp: install prefix) split into 35b-1 (install side, this) and 35b-2 (remove side, pending).

`skilltap install mcp:<source>` now installs MCP servers from a source without touching skill machinery. The `mcp:` prefix is detected at the top of the install command's run handler; if all sources have the prefix, dispatch to `runMcpInstall`. Mixing mcp: and regular sources in one invocation errors out with a clear hint.

For each `mcp:<inner>`:
- Resolve `<inner>` via existing source adapters (works for github, local, npm, etc.).
- Clone (or use local path) into a temp dir.
- Find servers: try `detectPlugin()` first (covers `.claude-plugin/`, `.codex-plugin/`, `.skilltap/` formats and extracts their `[[servers]]`). Fall back to a bare `.mcp.json` at the source root.
- Inject all found servers into the agent configs from `--also` (or `[claude-code]` by default), namespaced under `skilltap:<slug>:<server-name>` where slug is the last `/`-separated segment of `<inner>`.
- Write each server to state.json's `mcpServers[]` array (StoredMcpStandalone schema, populated for the first time since Phase 26).

Phase 35b-1 doesn't touch v0.x readers — installations land in `state.json` only (purely additive). Re-running with the same source replaces the existing entries (idempotent).

Smoke verified end-to-end: `skilltap install mcp:/tmp/mcp-test/source --project` writes `skilltap:source:db` into both `state.json` and `.claude/settings.json`. 11 new core tests + the existing 30 install tests still pass. Full v2 baseline 304/304.

Files:
- `core/src/mcp-install.ts`: `parseMcpRef(source)` + `installMcpOnly(source, options)` orchestrator. Reuses existing helpers (`resolveSource`, `clone`, `detectPlugin`, `parseMcpJson`, `injectMcpServers`, `loadState`/`saveState`).
- `core/src/mcp-install.test.ts`: 11 tests — parser cases (npm scoped, ssh form, missing prefix, etc.), local-source install path, idempotent re-install, no-servers-found error.
- `cli/src/commands/install.ts`: dispatch + `runMcpInstall` handler that calls `installMcpOnly` per source and renders the result list.

35b-2 (remove side) is pending — `skilltap remove mcp:<name>` should drop entries from state.mcpServers + agent configs. Smaller follow-up.

### Lint cleanup arc (commits b0880bd through 07cdd19)

After the v2.1 cutover settled, surfaced that `bun run lint` reported 104 errors and 279 warnings — most predating this work but never triaged. Multi-turn cleanup brought it to 0/0 without changing semantic behavior:

**Mechanical / safe-fix work** — `bun run check` ran biome's safe auto-fixes across 277 files (formatting, organize-imports, useImportType cascades). Lint dropped 104→98 errors / 279→264 warnings (`b0880bd`).

**Targeted unsafe-fix passes** — single-rule `biome check --write --unsafe --only=<rule>` invocations:
- `noUnusedImports`: 24 files cleaned (`e3899cc`)
- `noUnusedVariables`: 3 auto + 5 manual (test destructure-removal / `_`-prefix) (`eea0738`)
- `useTemplate` + `useLiteralKeys`: 12 files (`eea0738`)
- `useNodejsImportProtocol` + `noUnusedFunctionParameters`: 3 files (`eea0738`)

**File-level overrides in biome.json** for legitimate intentional patterns:
- Tests: `noConsole`, `noControlCharactersInRegex`, `noNonNullAssertion`, `noExplicitAny` (`1e40472` + earlier `79e7541`).
- `plugin/mcp-inject.ts` + tests: `noTemplateCurlyInString` (mcp `${VAR}` placeholders are runtime-substituted, not JS templates) (`ed42d6f`).
- `cli/src/completions/dynamic.ts` + benchmarks: `noConsole` (intentional output mechanism) (`3a81589`).

**Per-line documentation** of every remaining `!` non-null assertion in production. Each `biome-ignore` comment cites the specific runtime guard:
- `frontmatter.ts` (3 sites): `lines[i]!` after `while (i < lines.length)` (`8877528`)
- `taps.ts` (4 sites + 1 dead-code deletion): `parts[N]!` after length checks (`a3c3f6c`)
- `security/static.ts` (4 sites): `fileLines[N]!` inside arithmetic-checked guards, regex-group `!` after `m ?` (`98c49cf`)
- self-update / doctor/checks/git / skills/remove (9 sites batched): same pattern (`c373a58`)
- final batch across 10 files (11 sites): footer, install-callbacks, prompts, resolve, scan, doctor/symlinks, install, move, skill-check, update (`07cdd19`)

**Found and removed dead code** along the way:
- `const _tap = config.taps[idx]!;` in taps.ts (declared but never used, leftover from refactor) (`a3c3f6c`).
- 26 stale `// biome-ignore lint/style/noNonNullAssertion` comments in test files that became dead weight after the test-rule override (sed-removed in a single batch, `0d7e638`).

**Consistency wins** along the way:
- `cli.md` doc fix: `--agent <name>` (string) was documented but never existed; replaced with the actual boolean `--agent` flag (`d416ede`).
- `doctor/checks/installed.ts`: post-cutover, the check needed to read `state.json` first and fall back to `installed.json` (matching `loadInstalled`), otherwise fresh users got "0 skills (no installed.json)" falsely (`59c308b`).
- 5 doctor user-facing messages + 6 internal comments updated `installed.json` → `state.json` to match the canonical store (`5b7c01b`).
- macOS `/tmp` → `/private/tmp` symlink fix in `test-utils/src/tmp.ts::makeTmpDir()` resolved 6 long-standing test failures (`9e63393`).

### Post-cutover polish (commits d416ede, 59c308b, 5b7c01b)

After Phase 31c-c-2d-1 made state.json canonical, three small follow-ups landed across separate autopilot turns:

**31c-c-2c-flag** — wired `--agent` boolean flag into `args:` blocks of install, update, skills remove, tap install, and skills enable/disable. `composePolicy` already accepted `flags.agent`, but citty wasn't parsing it as a boolean — it was silently treated as a positional. Now the flag works on the command line, matching the v2.0 changelog promise. New e2e-v2 test 7 asserts `skilltap install ... --agent` from a non-TTY subprocess produces the agent-mode plain-text "OK: Installed" line.

**31c-c-2d-1-fix** — doctor's `installed` check (one of the original 9 v1 checks) only read `installed.json`. After the canonical-store cutover, fresh users had skills tracked in `state.json` but no `installed.json` — so doctor reported `0 skills (no installed.json)` falsely. Updated to read state.json first, fall back to installed.json (same pattern as `loadInstalled`). Also updated the related `--fix removes orphan records` test to seed and assert against state.json.

**31c-c-2d-1-msg** — 5 user-facing `skilltap doctor` messages in `checks/skills.ts` and 6 internal comments across install/orphan/move/adopt/link still referred to `installed.json`. Updated to `state.json` to match the cutover. Behavior unchanged; pure honesty pass.

Plus a doc fix on `website/reference/cli.md`: `--agent <name>` was incorrectly documented as a string flag for `skilltap update` (semantic-scan agent CLI selector). That flag never existed in the source — semantic-scan agent selection is config-only. Replaced with the actual boolean `--agent` (force agent mode) and added it to install + skills remove flag tables. Regenerated llms-full.txt.

Tests: 568 / 568 across the v2 surface remain green throughout.

### Phase 31c-c-2d-2 (orphan UX) complete — doctor detects and cleans up v0.x file orphans

After Phase 31c-c-2d-1 made state.json the canonical store, v0.x users who upgrade and run any install/update/remove get their data transparently transferred into state.json (via the read-fallback). The legacy `installed.json` / `plugins.json` files remain orphaned on disk — harmless but confusing.

Added `core/src/doctor/checks/v1-orphans.ts` as the 15th doctor check. Detects the orphan condition:
- state.json populated (skills or plugins) — i.e. user has migrated
- AND `installed.json` or `plugins.json` still on disk

When detected, emits a `warn` with one fixable issue per orphan. `skilltap doctor --fix` renames each orphan to `<file>.v1.bak`. Pre-migration users (state.json empty, installed.json populated) are intentionally NOT flagged — they need the fallback to keep working until they migrate or install something new.

Files:
- `packages/core/src/doctor/checks/v1-orphans.ts` — new check, ~80 lines.
- `packages/core/src/doctor/checks/v1-orphans.test.ts` — 6 tests covering: null state, empty state, populated state w/ no orphans, populated state w/ global orphan, populated state w/ multiple orphans, --fix renaming.
- `packages/core/src/doctor/index.ts` — wired in as check #15.

`skilltap doctor` now runs 15 checks (9 v1 + 6 v2). End-to-end smoke test in a clean env shows the new line: `v0.x file orphans: n/a (no populated v2 state)`.

What's left for full v0.x retirement (deferred to v2.2): delete the read-fallback in `loadInstalled`/`loadPlugins`, drop `[agent-mode]` from ConfigSchema, move `schemas/installed.ts` and `schemas/plugins.ts` to `schemas/v1/`. Requires a release window so users have time to run the orphan cleanup.

### Phase 31c-c-2d-1 complete — state.json is the canonical store

`saveInstalled()` and `savePlugins()` no longer write `installed.json`/`plugins.json`. They write directly to `state.json`. Reads still fall back to the legacy v0.x files when state.json is empty (handles unmigrated v0.x users for one read; the next save populates state.json and the fallback never fires again).

This subsumes the work that 31c-c-2a (the `syncV1ToV2State` shadow helper) and 31c-c-2b (the `loadActiveInstalled` / `loadActivePlugins` bridge helpers) did. With reads + writes both going through state.json natively, the bridge files are dead code:

Deleted:
- `packages/core/src/state/sync-from-v1.ts` + `sync-from-v1.test.ts`
- `packages/core/src/state/read-bridge.ts` + `read-bridge.test.ts`

install/update/remove/plugin paths reverted to plain `loadInstalled` / `loadPlugins` calls (which now do the right thing internally).

Tests touched:
- `packages/core/src/link.test.ts` — assertion changed from `installed.json` to `state.json`.
- `packages/core/src/plugin/install.test.ts` — same; assertion against `state.json`.
- `packages/cli/src/e2e-v2.test.ts` — dropped now-impossible `installed.json` checks; only state.json is asserted.

What's now possible: a fresh skilltap user has only `state.json` on disk — no `installed.json` or `plugins.json` ever appear. v0.x users who upgrade get one transparent read fallback, then their state.json is populated and the legacy files become orphaned (deletable on the user's schedule, or by a future `skilltap prune` command).

What remains for 31c-c-2d-2 (the final cleanup, deferred):
- Delete `packages/core/src/schemas/installed.ts` and `schemas/plugins.ts` v0.x schemas (still referenced by `migrate/run.ts` for one-shot upgrades; can move to `schemas/v1/`).
- Delete the read-fallback paths in `loadInstalled` / `loadPlugins` (after a release window).
- Drop `[agent-mode]` config block from ConfigSchema (after deprecation window).

Tests: 496 pass across 40 files.

### Phase 31c-c-2c-flag complete — `--agent` flag wired into CLI commands

The follow-up I deferred from 31c-c-2c. `composePolicy` already accepted `flags.agent` and `loadPolicyOrExit` already passed it through; what was missing was the per-command `args:` definition that lets users actually type `--agent` on the command line.

Wired `agent: { type: "boolean", default: false }` into `args` for:
- `install` (already passed `agent: args.agent` through to `loadPolicyOrExit`).
- `update`
- `skills remove`
- `tap install`
- `skills enable` / `skills disable` (shared via `makeToggleCommand` factory)

Each of those commands now accepts `--agent` and threads it through to `loadPolicyOrExit({ ..., agent: args.agent })`.

End-to-end test: `packages/cli/src/e2e-v2.test.ts::test 7` runs `skilltap install <local-skill> --project --agent` from a non-TTY subprocess and asserts the output contains `OK: Installed standalone-skill` (the agent-mode plain-text format, not the clack spinner output).

Tests: 7 / 7 in e2e-v2 pass. Full v2 surface still 523+ green.

### Phase 31c-c-2c complete — `--agent` flag + `SKILLTAP_AGENT` env var honored

The v2.0 changelog advertised the `--agent` flag and `SKILLTAP_AGENT=1` env var as the modern way to enter agent mode, but `composePolicy` only read the legacy `[agent-mode].enabled` config block — the env var did nothing and the flag was ignored. This phase makes both work.

`composePolicy` and `composePolicyForSource` now resolve `agentMode` with this precedence:
1. `flags.agent === true` (explicit `--agent`)
2. `process.env.SKILLTAP_AGENT === "1"` (env var override)
3. `config["agent-mode"].enabled` (legacy v0.x — still honored until 31c-c-2d's schema deletion)

CLI startup checks (`runTelemetryNotice`, `runUpdateCheck`, the skill-update reminder) and `isAgentMode()` in `cli/src/ui/policy.ts` also short-circuit on the env var so background output is suppressed for agent invocations regardless of config.

`CliFlags.agent` is now defined; per-command `--agent` flag wiring in `args:` blocks is deferred to a follow-up (the env var covers the most common use case — `SKILLTAP_AGENT=1 skilltap install ...`).

Files:
- `packages/core/src/policy.ts` — added agent precedence resolution; `flags.agent` field on `CliFlags`.
- `packages/core/src/policy.test.ts` — 4 new tests covering flag, env var (set/unset/non-"1" values), and back-compat with the config block.
- `packages/cli/src/index.ts` — env var short-circuits in 3 startup hooks.
- `packages/cli/src/ui/policy.ts` — `isAgentMode()` checks env var first.

Tests: 375 pass across 31 files (policy, policy-v2, state, install, lifecycle, manifest, sync, migrate, doctor, status, try, mcp, e2e-v2). Existing `[agent-mode]` config tests remain green — back-compat preserved.

What's now possible: `SKILLTAP_AGENT=1 skilltap install foo --project --skip-scan` works end-to-end without touching `~/.config/skilltap/config.toml`. CI scripts and AI agent harnesses no longer need to pre-mutate the config to get agent-mode behavior.

### Phase 31c-c-2b complete — state.json reads cutover + dual-write moved to source

Two improvements landed together because the first one surfaced a bug the second one fixed.

**Read-side cutover.** New helpers `loadActiveInstalled(scope, projectRoot)` and `loadActivePlugins(scope, projectRoot)` in `core/src/state/read-bridge.ts` read state.json first, fall back to v0.x `installed.json`/`plugins.json` only when state.json is empty (handles unmigrated v0.x users gracefully). install.ts, update.ts, remove.ts, plugin/install.ts, plugin/lifecycle.ts all now call these instead of `loadInstalled`/`loadPlugins` directly. The fallback is auto-healing — once any v0.x write fires, the next dual-write populates state.json and the fallback never fires again for that scope.

**Centralized dual-write.** Phase 31c-c-2a originally peppered `syncV1ToV2State()` calls across 6 site (install, update, remove, plugin/install, plugin/lifecycle remove + toggle). The lifecycle test surfaced the gap: `disable.ts`, `enable` (also in disable.ts), `move.ts`, `adopt.ts`, `link.ts` ALSO write `installed.json` and weren't dual-writing. Reads from state.json then returned stale `active` flags and update-while-disabled looked in the wrong directory.

Fix: moved the shadow-write into `saveInstalled()` (in `config.ts`) and `savePlugins()` (in `plugin/state.ts`) themselves. Every caller — install, update, remove, disable, enable, move, adopt, link, plugin install, plugin lifecycle — now gets the dual-write automatically. Removed the per-call-site `syncV1ToV2State` invocations from 31c-c-2a; the helper still exists for explicit migrations but isn't called on every install path anymore.

Both writes use dynamic `import()` to avoid circular-import risk between `config.ts` and `state/` modules.

Files:
- `packages/core/src/config.ts` — `saveInstalled` now shadows into state.json after writing installed.json (private helper `shadowSkillsIntoState`).
- `packages/core/src/plugin/state.ts` — `savePlugins` now shadows into state.json after writing plugins.json (private helper `shadowPluginsIntoState`).
- `packages/core/src/state/read-bridge.ts` — new `loadActiveInstalled`, `loadActivePlugins` helpers.
- `packages/core/src/state/read-bridge.test.ts` — 5 tests (state-first happy path, fallback path, empty path).
- `packages/core/src/install.ts`, `update.ts`, `remove.ts`, `plugin/install.ts`, `plugin/lifecycle.ts` — switched reads to read-bridge, removed per-call-site sync calls.
- `packages/cli/src/e2e-v2.test.ts` — assertions extended (state.json now appears post-install, post-sync).

Tests: 569 across 46 files pass (state, install, update, lifecycle, manifest, sync, migrate, doctor, status, try, mcp, policy, plugin-v2, schemas, e2e). The two pre-existing macOS `/private/tmp` symlink failures in `plugin/parse-claude.test.ts` and `plugin/e2e-plugin.test.ts` remain — unrelated to v2.1 work.

What's now possible: install/update/remove no longer depend on `installed.json`/`plugins.json` for reads. v0.x files become a write-only artifact maintained for backward compat. The destructive deletion (31c-c-2d) is now a clean cut — drop the writes, drop the schemas, done.

### Phase 31c-c-2a complete — state.json dual-write (v2.1 cutover begins)

After v2.0 shipped, the user re-invoked autopilot to push forward into v2.1. Phase 31c-c-2 was originally scoped as one big destructive batch (state.json reads cutover + `[agent-mode]` retirement + v0.x schema deletion). Splitting it into four sub-phases lets the safest piece land first:

**31c-c-2a (this) — write side.** Every install/update/remove/plugin-toggle path that writes `installed.json` or `plugins.json` now also writes `state.json` as a shadow. v2 readers (`status`, `doctor`, `sync`) see new installs without requiring `skilltap migrate`. v0.x readers (still active in `install`/`update`/`remove`) keep working unchanged.

The dual-write is non-fatal (`.catch(() => undefined)` on every call) — if the v2 shadow can't be written for any reason, the v0.x install already succeeded and the user isn't blocked.

Files:
- `packages/core/src/state/sync-from-v1.ts` — new `syncV1ToV2State(scope, projectRoot)` helper. Reads current `installed.json` + `plugins.json`, preserves any existing `state.mcpServers` (populated by Phase 35b's `mcp:` installs), writes the merged `state.json`.
- `packages/core/src/state/sync-from-v1.test.ts` — 4 unit tests (project scope, mcpServers preservation, no-files default, global scope).
- `packages/core/src/install.ts` — call after step 10 (saveInstalled).
- `packages/core/src/update.ts` — call after the global + project saveInstalled.
- `packages/core/src/remove.ts` — call after saveInstalled.
- `packages/core/src/plugin/install.ts` — call after savePlugins.
- `packages/core/src/plugin/lifecycle.ts` — call after savePlugins on remove + toggle.
- `packages/cli/src/e2e-v2.test.ts` — extended assertions: install AND sync now produce `state.json` v2 alongside `installed.json`.

What's now possible: a fresh v2.0 user who installs without ever running `skilltap migrate` still gets a working `state.json` for `status`/`doctor`/`sync` consumption. The migrate command becomes optional rather than required for v2 features.

Tests: 4 sync-from-v1 + 18 CLI e2e + 305 v2-surface tests + 241/243 lifecycle tests (2 pre-existing `/tmp` symlink failures in plugin/e2e-plugin unrelated to this change).

### Phase 38d complete — v2.0 end-to-end test (roadmap 38.5)

The last in-scope v2.0 work item from the roadmap. New `packages/cli/src/e2e-v2.test.ts` walks the canonical v2 journey as a real CLI subprocess:

1. Seed empty `skilltap.toml` in a fresh git project.
2. `install <local-skill> --project --skip-scan --yes` — assert manifest entry, lockfile entry with sha, and `installed.json` are written. (state.json cutover is deferred to v2.1; install still uses the v0.x dual-write path.)
3. `status` — assert dashboard mentions the installed skill name.
4. `doctor` — assert clean exit (warnings allowed, hard failures not).
5. **Fresh-clone sync** — copy `skilltap.toml` + `skilltap.lock` (no `.agents/`) into a brand-new git dir, run `sync --apply`, assert `installed.json` materializes with the skill.
6. **v1 → v2 migrate** — set up an old-shape `installed.json` + `.agents/skills/legacy-skill/SKILL.md` in a fresh dir, run `migrate`, assert `state.json` v2 appears with the legacy skill carried over.

Bug found and fixed in the process: `core/src/status/gather.ts::tryProjectRoot` used `Bun.file(".git").exists()` to verify project-root candidates, but `Bun.file().exists()` returns false for directories — so the CLI's `status` command was always reporting "no project root" when run inside a real git repo. Replaced with `lstat(...).catch(() => null)` to match the existing pattern in `paths.ts`, `orphan.ts`, `skill-check.ts`, and `update.ts`. Existing 24 status tests still pass (they bypass the buggy path by passing `projectRootHint` directly, which is why this had never been caught).

Files:
- `packages/cli/src/e2e-v2.test.ts` — new, 6 tests, ~250 lines.
- `packages/core/src/status/gather.ts` — `tryProjectRoot` fix (Bun.file → lstat).

Tests: 47 CLI tests (e2e-v2 + e2e + install + doctor) + 155 core/status/migrate/manifest/sync/state tests all green.

### Phase 35b-2 complete — `skilltap remove mcp:<source>`

Mirror of 35b-1 on the remove side. `skilltap remove mcp:<source>` drops every state.mcpServers entry whose `source` matches and prunes the namespaced `skilltap:<plugin>:<server>` keys from each target agent's MCP config.

Implementation:
- `core/src/mcp-install.ts`: new `removeMcpInstall(source, options)` — loads state, filters `mcpServers` by `source`, groups remaining matches by their parsed `pluginName` (via `parseNamespacedKey`), calls `removeMcpServers` once per pluginName/agents combination so each agent config gets pruned, then saves state without the matched entries. Returns a count + agent list + name list for CLI output. Errors with a "No MCP servers installed from source" UserError when the source has no matches.
- `cli/src/commands/skills/remove.ts`: dispatch on `mcp:` prefix at the top of `run`. Refuses mixed mcp: + regular sources in one invocation. Honors agent-mode for plain stdout output, otherwise emits one `successLine` per removed server.
- `core/src/mcp-install.test.ts`: 3 new tests — full round-trip install + remove (state pruned, agent config pruned), error on unknown source, multi-source install where only one is removed (selectivity check).

Tests: 14 mcp-install tests + 64 MCP-surface tests + 245 doctor/try/plugin-v2/schema tests all green. The macOS `/tmp` → `/private/tmp` symlink failures in `parse-claude.test.ts` (2 cases) are pre-existing and unrelated to MCP.

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

Decisions D1–D5 logged in `docs/designs/completed/phase-31c-a.md`.

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

Autonomous decisions D1–D7 (full text in `docs/designs/completed/phase-36.md`):
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

Decisions D1–D4 logged in `docs/designs/completed/phase-34.md`. Used inline design + direct implementation.

Tests: 10 component-ref parser tests + lookup. CLI smoke-tested via `--help` and error-path renders. Full v2 baseline 224/224 pass in <250ms.

### Phase 35a complete — try + Claude Desktop

`skilltap try <source>` previews any source (URL, owner/repo, npm:, local path) without writing anywhere. Clones to a temp dir for remote sources; uses the path directly for local. Parses plugin manifests, scans for skills, runs static security scan, prints a structured summary, then cleans up. `--skip-scan` and `--json` flags supported.

Claude Desktop added to `MCP_AGENT_CONFIGS` at module load via `process.platform`:
- macOS: `Library/Application Support/Claude/claude_desktop_config.json`
- Linux: `.config/Claude/claude_desktop_config.json`
- Windows: deferred (needs `%APPDATA%` resolution that doesn't fit the relative-path shape)

Decisions D1–D4 logged in `docs/designs/completed/phase-35a.md`. Used inline design + direct implementation (small scope: 6 files, mostly additive).

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
- **32**: dedicated agent-flag wire-up. Originally planned to swap v1 `composePolicy` for `composeV2`. Phase 31c-c-2c took the simpler path: extended v1 `composePolicy` directly with the same precedence (`flags.agent > SKILLTAP_AGENT > config`). The post-cutover doc-audit pass (this session) marks 32 as **superseded by 31c-c-2c** and documents `policy-v2/index.ts` as reserved-but-unwired infrastructure. The `composeV2` module retains a few v2-only concepts (`--no-agent`, source-level `trust` glob matching, `EnvV2` separation) that a future v2.x phase could pick up.

### Phase deferred to user

- **38c**: `bun run bump 2.0.0` and `git push --follow-tags`. The autopilot mandate forbids pushing to remote, and the existing bump script auto-commits + tags + pushes. The user runs it when ready to release. CI workflow handles npm publish + Homebrew formula update.

### Workflow practices used

- `/workflow:design` produced explicit design docs at `docs/designs/completed/phase-{N}.md` before any phase that touched multiple modules. 17 design docs total.
- `/workflow:implement-orchestrator` spawned Sonnet sub-agents for the larger phases (31b, 36) — clean splits, parallel execution, agents flagged real bugs in design (e.g., the `i.kind === "add" || "remove" || "ref-mismatch"` always-truthy expression).
- Smaller phases used inline design + direct implementation (8–10 phases). Tradeoff: faster context use, slightly less rigor.
- One refactor pass at the natural moment after Phase 34 (concrete duplication had appeared in toggle/enable/disable + plugin/info).

### Final test counts

- **v2 baseline** (additive code from Phases 26–36): 293 tests across 30 files in ~530ms.
- **Existing v0.x** (install + remove + plugin install + lifecycle): 72 tests across 4 files in ~3s.
- **Combined**: 365 tests passing.

### Known issues / follow-ups for v2.1

- ~~Cutover (31c-c-2) — install/update/remove still read v0.x `installed.json` + `plugins.json`.~~ **Done** in 31c-c-2d-1: `state.json` is canonical, v0.x reads remain only as one-time fallback for unmigrated users.
- ~~`mcp:` install prefix not yet implemented (35b).~~ **Done** in Phases 35b-1 (install) and 35b-2 (remove).
- ~~Phase 31c-c-2's split into 31c-c-2-a/b/c/d~~ **Done** — full split shipped (a/b/c/d-1/d-2-orphan).
- ~~The `componentLabel` function in `cli/src/commands/plugin/info.ts` has different semantics from the shared one~~ **Resolved (post-cutover doc-audit pass)**: local helper renamed `componentKind` so the collision is gone.
- ~~Bump script (`scripts/bump-version.ts`) doesn't accept pre-release versions; auto-pushes on tag.~~ **Resolved (post-cutover doc-audit pass)**: regex extended to `/^\d+\.\d+\.\d+(?:-[A-Za-z0-9.-]+)?$/` so prerelease tags like `2.0.0-rc.1`, `1.5.0-beta`, `0.9.0-alpha.2` parse. The patch/minor/major shortcuts now strip an existing prerelease tag before incrementing (so `bump patch` on `2.0.0-rc.1` yields `2.0.1`). Added `SKILLTAP_BUMP_NO_PUSH=1` env var that stages the commit + tag locally and prints "git push --follow-tags" for the user — autopilot-runs can now invoke the script for the version-bump step without violating the "never push to remote" mandate.

### Watchdog loop

Each autopilot session scheduled its own session-local cron loop. None survive across sessions; a fresh `/workflow:autopilot` invocation re-arms it.
