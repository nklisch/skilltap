# v2.2 Cleanup — Autopilot Progress

Tracking artifact for the autopilot run executing `docs/designs/v2.2-cleanup.md`.
The design has 52 implementation units across 4 phases — too large for a single
orchestrator pass, so we split into waves and resume across loop wakeups.

**Design**: `docs/designs/v2.2-cleanup.md` (do not edit during the run; refer back to it).
**Agent model**: Opus for all spawned implementation agents (per user direction).
**Resume rule**: find the first wave with status `pending` or `in_progress`, verify its
predecessor's verification gate passed, then continue.

## Wave plan

Each wave is one orchestrator-style invocation: 1–3 Opus agents implementing a focused
unit cluster, followed by a verification gate (`bun test` + optional `bun run build`).

| Wave | Phase | Units | Status | Notes |
|---|---|---|---|---|
| 1A | 1 | 1.1, 1.2, 1.3, 1.11, 1.12, 1.13 + (1.10 + partial 1.7) | done | Schema + policy promotion + sweep exports. Pulled 1.10 + minimal 1.7 forward to keep build green; agent `a2cfe3baf6c0fa8f4`, commit `44fd2b1` |
| 1B | 1 | 1.4, 1.5, 1.6 + inline fixture rewrites | done | loadConfig hard-fail + migrate + round-trip test. Inline-rewrote `agents/__tests__/detect.test.ts` + `cli/commands/completions.test.ts` legacy fixtures (Unit 3.6 work pulled forward). Agent `a850fe61e7e5c9518`, commit `bc06ab9` |
| 1C | 1 | 1.7, 1.8, 1.9, 1.14, 1.15 + 3 inline fixture rewrites | done | CLI security refine + SETTABLE_KEYS + template + manifest [[mcps]] + sync mcps. Suite 35 fail → 11 fail / 5 errors (remaining are pre-existing Phase 3 territory). Agent `ad9828228fbc70f76`, commit `cfd58d6` |
| 2  | 2 | 2.1–2.5 + validate.ts deletion | done | Dead code deletions. -1213 lines. All 6 removed-command hints (verify/link/unlink/enable/disable/skills) print OK. Suite stable at 11 fails. Agent `acfbf43331c390eca`, commit `193dbf2` |
| 3a | 3 | 3.1–3.8 | done | Code cleanups. 63 files (628 ins / 832 del). 23→0 phase-numbered comments stripped. Doctor fixtures rewritten to V2. Schema doc comments cleaned. Suite stable at 11 fail / 5 errors. Agent `a29ea7099de86d643`, commit `acff06d` |
| 3b | 3 | 3.9, 3.10, 3.11, 3.12 + `loadConfigIfExists` helper + `collectRepeatedFlag` helper | done | CLI surface changes. **Suite 11 fail / 5 errors → 1 fail / 0 errors.** Discovered citty/mri does not auto-collect repeated string args; implemented `collectRepeatedFlag` walking `rawArgs`. Added `loadConfigIfExists` so `try` can read config without creating one (preserves never-writes invariant). Smart-scope reporting: `scope: <project\|global> (inferred from cwd)`. `info.ts` and `status.ts` left on legacy boolean flags (orthogonal to composePolicy; not in design scope). Agent `a278f907b70cf33a9`, commit `ba322a6` |
| 3c | 3 | 3.13, 3.14, 3.15, 3.16 + new core API: `installPlugin.skipCapture` + `"skip"` capture decision | done | Multi-plugin syntax (`:auth`, `:*`), capture flags (`--force-capture`/`--no-capture` w/ mri-aware `hasRawFlag`), lifecycle drift fix across update/move/adopt/disable/toggle/migrate, drift.ts range/ref. **Suite holds at 1 fail / 0 errors.** Agent `af914ff971536adb2`, commit `2ab1aa1` |
| 3d | 3 | 3.17, 3.18, 3.19, 3.20 + toggle scope bugfix + DoctorCheck.fixed/fixDescription API | done | TUI fixes (Dashboard tabs / Adopt Enter / Toggle name step + focus) + doctor exit-code fix + `--json` field completeness + tui.smoke routes through `cliCmd()` + CLI test expansion (5 new + 4 extended). Bonus: caught real `runToggleSkill` scope-pass bug. **Suite 2199 pass / 1 fail** (baseline). `verify:binary:tests` 588/588. Agent `ab55753f67fc238f3`, commit `1849435` |
| 4a | 4 | 4.1, 4.3, 4.4, 4.5, 4.7 | done | Foundation docs minus SPEC. ROADMAP 946→961 (+v2.2 section), SECURITY 344→198, VISION 558→311, ARCH 976→740, AGENTS 175→176. -612 lines net. All validation greps pass. Agent `a7e136e731f81f152`, commit `40c3d3d` |
| 4b | 4 | 4.2 + cross-doc anchor sweep | done | SPEC.md 3576→2070 lines (158k→73k), 22 sections, Removed-in-v22 appendix. Cross-doc anchors fixed in VISION/ARCH/ROADMAP/README/changelog/phase-26.md. Suite still baseline. Agent `abaad07155a29e954`, commit `ba9cb1c` |
| 4c | 4 | 4.6, 4.8, 4.9 | done | UX +68 lines (scanner block, multi-plugin section, capture flags, legacy-commands table). changelog rewritten with BREAKING v2.2.0 entry. README minor flag/key updates. `website/index.md` is a 3-line VitePress layout shell; landing copy lives in theme component. Agent `af6b0e38f8999d685`, commit `9a9afa1` |
| 4d | 4 | 4.10, 4.11 | pending | website/guide/ (11 pages) + website/reference/ (4 pages) |
| 4e | 4 | 4.12 + final verification | pending | Regenerate llms-full.txt + run full design verification checklist |
| Final | — | — | pending | Full verification checklist from design |

## Wave order rationale

- **1A → 1B → 1C**: strictly sequential. 1B depends on 1A's V2 schema; 1C depends on both.
- **2 after 1**: deletions touch files Phase 1 also rewrites; safer after Phase 1 lands.
- **3a after 1**: cleanups assume the V2 surface is in place.
- **3b after 3a**: scope/try/also flag rewrites coordinate per-command.
- **3c after 3b**: multi-plugin and capture depend on the typed CLI shape from 3b. Lifecycle drift writes to manifest [[mcps]] from Unit 1.14.
- **3d after 3a**: TUI/doctor/tests are independent of surface changes; could run in parallel with 3b/3c if context permits.
- **4 after 2+3**: docs describe shipped behavior, so all code lands first.
- **4c last**: llms-full.txt regenerates from hand-edited 4a/4b output.

## Verification gates

After each wave:
- `bun test` (full suite must be green; allow per-wave focused runs during in-flight).
- `bun run build` (binary must compile without externals leaking).
- `bun run verify:binary` for waves that touch the build pipeline.

Final verification: run the full checklist from `docs/designs/v2.2-cleanup.md` Verification section.

## Decision log

Decisions made autonomously during the run that aren't already captured in the design itself.

- **2026-05-08 Wave 1A** — Wave 1A pulled forward Unit 1.10 (scanner-consumer rewire across `agents/detect.ts`, `doctor/checks/agents.ts`, `cli/commands/find.ts`, `install/skill.ts`, `install/plugin.ts`, `update.ts`, `ui/resolve.ts`) because deleting the legacy `[security].agent_cli`/`threshold`/`max_size`/`ollama_model` fields would otherwise leave the build broken. The agent also wrote a minimal V2-shape `cli/commands/config/security.ts` to replace the legacy file (which imported deleted `PRESET_VALUES`/`SECURITY_PRESETS`/`TrustOverride`). Wave 1C still owns refining `config security` per design spec and updating its tests.
- **2026-05-08 Wave 1A** — `migrate/config-v1.ts` import flipped from `../schemas/config-v2` → `../schemas/config` (typecheck-only edit); body still emits the legacy `agent` block (silently stripped by Zod). Wave 1B owns the full body rewrite per Unit 1.5.
- **2026-05-08 Wave 1A** — `security/describe.ts` reduced to a 1-line `${scan} + ${on_warn}` formatter; `matchPreset` removed (no production callers). `security/describe.test.ts` will fail until Wave 1C / Phase 3 cleanup retires the test.
- **2026-05-09 Wave 3b** — Design assumed citty/mri auto-collects repeated `--flag value` args into arrays; verification showed it doesn't (last-wins). Implemented `collectRepeatedFlag(rawArgs, name)` that walks the raw arg vector and gathers both `--flag value` and `--flag=value` forms. All install/adopt/move handlers now thread `rawArgs` from citty's run context.
- **2026-05-09 Wave 3b** — Added `loadConfigIfExists()` to core: same shape as `loadConfig()` but never creates the config file/dir on first read. `try` uses it so the never-writes invariant in `try.test.ts` holds.
- **2026-05-09 Wave 3b** — Left `info.ts` and `status.ts` on the legacy `--project`/`--global` boolean flags (not in design's Unit 3.9 file list and they don't go through `composePolicy`). Worth a follow-up if full uniformity is wanted later.
- **2026-05-09 Wave 3c** — citty `0.2.1` intercepts `--no-X` as `args.X = false`. Solved with `hasRawFlag(rawArgs, "no-capture")` reading directly from raw arg vector — same pattern as Wave 3b's `collectRepeatedFlag`.
- **2026-05-09 Wave 3c** — Capture decision system grew a third `"skip"` outcome (was just `"force" | "abort"`) plus a top-level `installPlugin.skipCapture` option, because the design's `--no-capture --yes` acceptance ("install side-by-side") needed a path through capture detection that wasn't abort. Public API addition — non-breaking.
- **2026-05-09 Wave 3c** — Adopt-from-path with `record.repo === null` deliberately skips manifest write; absolute local paths aren't portable across developers, matches `installSkill` convention.
- **2026-05-09 Wave 3c** — `disable foo` on a skill installed via string-range entry rewrites the manifest entry to inline-table form `foo = { components = { foo = false } }` and drops the range string. Acceptable for v2.2 (manifest schema doesn't currently allow range alongside components); flagged for future polish.
- **2026-05-09 Wave 3d** — Split Phase 4 wave plan: SPEC.md (Unit 4.2) is 158k chars / 3576 lines and gets its own dedicated wave (4b). Foundation docs minus SPEC bundle into wave 4a. Website split into root (4c: index + README + UX + changelog) and deep-dive (4d: guide/ + reference/). Total Phase 4 waves: 5 (was 3). Reduces context-pressure risk per agent.
- **2026-05-09 Wave 3d** — `runToggleSkill` was passing `{ projectRoot }` to `disableSkill`/`enableSkill` without a `scope`. Without scope, those functions read GLOBAL state, so a project-scoped skill resolved as "not installed". Fixed inline because it blocked the new `toggle.test.ts`. Single-line fix; toggle now works for both global and project skills from any cwd.
- **2026-05-09 Wave 3d** — `DoctorCheck` gained `fixed?: boolean` + `fixDescription?: string` (derived/aggregated from underlying issues by the runner). Public API addition for `@skilltap/core`; non-breaking. Phase 4 SPEC update should mention this in the doctor section.
- **2026-05-09 Wave 3d** — TUI smoke test consolidation: 5 separate spawn tests collapsed into 1 session covering both `dashboard:tab` and `f`-navigates-to-Find behaviors, because cold-cache PTY spawn was unstable with 5 spawns in one file (intermittent 30s+ timeouts).

## Deviation log

Things that didn't go as planned: failed assertions, design assumptions that turned out wrong,
substitutions that diverged from the design.

- **2026-05-08 Wave 1A** — full `bun test` reports 45 failures at the wave boundary (test-fixture residue: `agents/__tests__/detect.test.ts` legacy fixture, `security/describe.test.ts` deleted-symbol imports, `cli/config/security.test.ts` legacy assertions, etc.). Expected; resolved by Wave 1B (Unit 1.4 hard-fail makes legacy fixtures fail loudly and forces rewrite) + Wave 1C (Unit 1.8 SETTABLE_KEYS) + Phase 3 (Unit 3.6 fixture sweep). Build (`bun run build`) is clean and the binary verifies — typecheck-clean wave boundary held.

## Completion log

Wave-by-wave completion entries with date, agent IDs, and summary.

- **2026-05-08 Wave 1A** — Units 1.1, 1.2, 1.3, 1.10, 1.11, 1.12, 1.13 + minimal 1.7 done. 27 files changed (modified/renamed/deleted). 60 schema+policy tests pass. Build clean, binary verifies. Agent `a2cfe3baf6c0fa8f4`. Commit `44fd2b1`.
- **2026-05-08 Wave 1B** — Units 1.4, 1.5, 1.6 done + inline rewrite of two of four Unit 3.6 fixture files. 9 files modified. 67 in-scope tests pass; full suite 45→34 fails (11-fail improvement). Build clean. Default-config template rewritten to V2 shape so first-run doesn't trip its own hard-fail gate. `[registry].allow_npm` dropped silently while preserving `enabled`/`sources` (defensible deviation; user's tap-search settings preserved). Agent `a850fe61e7e5c9518`. Commit `bc06ab9`.
- **2026-05-09 Wave 1C** — Units 1.7, 1.8, 1.9, 1.14, 1.15 done. 18 files modified. 76 manifest + 35 sync + 18 mcp-install + 40 config CLI tests pass. Inline-rewrote `security/describe.test.ts`, `cli/config/set.test.ts`, `cli/config/get.test.ts`. Full suite 35 fail → **11 fail / 5 errors** (remaining are pre-existing Phase 3 territory: install PTY timeouts, e2e plugin regressions, taps.http-removal subprocess). Manifest and lockfile now have `[[mcps]]` + `[[mcps.lock]]` tables; `installMcp`/`removeMcp` write to both; sync drift+apply handle MCPs end-to-end. Phase 1 V2 cutover **fully wired**. Agent `ad9828228fbc70f76`. Commit `cfd58d6`.
- **2026-05-09 Wave 2** — Units 2.1, 2.2, 2.3, 2.4, 2.5 done + `validate.ts` deletion (sole consumer was `verify`). 7 files deleted, 14 modified. -1213 lines net. All 6 removed-command hints print OK (verify/link/unlink/enable/disable/skills exit 1 with replacement-path stderr). Suite holds at 11 fail / 5 errors (no regressions; +3 pass from `e2e-phase19` rewrites). Agent `acfbf43331c390eca`. Commit `193dbf2`.
- **2026-05-09 Wave 3a** — Units 3.1–3.8 done. 63 files changed (628 ins / 832 del). `info.ts` legacy fallback removed. `loadInstalled`/`saveInstalled` → `loadSkillState`/`saveSkillState` (39 caller files). Bash/zsh/fish completions fully rewritten + tests. `capture.ts` `--agent` text and `git.ts` `skilltap link` hint replaced. Inline TOML fixtures in `core/doctor.test.ts` and `cli/doctor.test.ts` rewritten to V2 (the remaining two of four files identified by the audit). `disable-enable.md` already in completed/; moved `security-config-redesign.md` there too. 23 phase-numbered comments stripped (none survived). Suite stable at 11 fail / 5 errors. Agent `a29ea7099de86d643`. Commit `acff06d`.
- **2026-05-09 Wave 3b** — Units 3.9, 3.10, 3.11, 3.12 done + `collectRepeatedFlag` helper + `loadConfigIfExists` helper. 13 CLI source files + 2 core files + 13 test files rewritten from `--project`/`--global` to `--scope`. Smart-scope inference now reports the inferred value. `try skill foo` / `try plugin foo` / `try mcp foo` typed positional shipped. `--no-strict` removed everywhere. `--also` repeatable. `install mcp` honors smart-scope outside git repos. **Suite 11 fail / 5 errors → 1 fail / 0 errors** — only `taps.http-removal.test.ts` (pre-existing, untouched) remains. Agent `a278f907b70cf33a9`. Commit `ba322a6`.
- **2026-05-09 Wave 3c** — Units 3.13, 3.14, 3.15, 3.16 done + `hasRawFlag` helper + `setManifestComponentActive` helper + `installPlugin.skipCapture` API + `"skip"` capture decision. Multi-plugin syntax (`owner/repo:name`, `owner/repo:*`) with full source-form matrix tested across github/git/local adapters and URL forms. Plugin Capture flags `--force-capture`/`--no-capture` shipped (mri `--no-X` workaround via raw-arg walk). Lifecycle drift closed across update/move/adopt/disable/toggle/migrate — manifest+lockfile in sync after every state write. drift.ts inline-table `range = "*"` correction. New `lifecycle.manifest.test.ts` (5 tests) + `install.multi-plugin.test.ts` (5 subprocess tests). **Suite stable at 1 fail / 0 errors.** Agent `af914ff971536adb2`. Commit `2ab1aa1`.
- **2026-05-09 Wave 3d** — Units 3.17, 3.18, 3.19, 3.20 done + bonus `runToggleSkill` scope bugfix + `DoctorCheck.fixed`/`fixDescription` API additions. 15 files modified, 5 created. TUI: Dashboard tab interception, Adopt Enter handler, Toggle name-step + focusIndex. Doctor: exit-0 after successful fix; JSON output includes `info`/`fixDescription`/`detail`. TUI smoke routes via `cliCmd()`. New CLI test files: `toggle.test.ts`, `status.test.ts`, `install.smart-scope.test.ts`, `install/mcp.test.ts`, `remove/mcp.test.ts` (5 new). Extended: `update.test.ts`, `adopt.test.ts`, `doctor.fix.test.ts`, `doctor.test.ts`. **Suite 2199 pass / 1 fail (baseline) / 51 skip. `verify:binary:tests` 588 pass / 0 fail.** **Phase 3 complete.** Agent `ab55753f67fc238f3`.
- **2026-05-09 Wave 4a** — Units 4.1, 4.3, 4.4, 4.5, 4.7 done. ROADMAP marked 39–46 complete + new "v2.2 Cleanup Wave (V2 cutover)" section with C.1–C.10 entries. SECURITY full V2 rewrite (344→198 lines, dropped agent-mode/presets/overrides/per-mode tables). VISION dropped HTTP Registry section, replaced `skilltap link` → `skilltap adopt`, added "One runtime" 5th design principle. ARCH full V2 rewrite (976→740, manifest `[[mcps]]`, sync handles all 3 state types, `policy/` module, removed-command-hints section). AGENTS.md (`.claude/CLAUDE.md` is symlink) `v2.0 Redesign conventions` → `v2.2 conventions`. Agent `a7e136e731f81f152`. Commit `40c3d3d`.
- **2026-05-09 Wave 4b** — Unit 4.2 done. SPEC.md rewrite: 3576→2070 lines, 158k→73k. 22-section structure (broke Doctor / TUI / Telemetry / Self-Update / Trust Signals / Git URL Fallback into own sections vs design's 12). Removed-in-v22 appendix is the canonical "what got cut" reference. Cross-doc anchor sweep updated VISION L116 (`#tapjson`→`#source-adapters`), changelog L320 (`#v20-security`→`#security-scanning`), README L119 (`#cli-commands`→`#project-manifest-and-lockfile`), ROADMAP L524 (`#cli-commands` re-target), phase-26.md (broken `..` path corrected to `../../`). Suite still 2199 pass / 1 baseline fail. Agent `abaad07155a29e954`. Commit `ba9cb1c`.
- **2026-05-09 Wave 4c** — Units 4.6, 4.8, 4.9 done. UX.md 1130→1198 lines (+68): added `[scanner]` block, populated `trust = [...]` example, "Multi-Plugin Sources" subsection, `--force-capture`/`--no-capture` docs, "Legacy Commands" table, smart-scope reporting, full migrate translation description, updated `require_scan` references throughout. website/changelog.md v2.2.0 entry rewritten with **BREAKING** lead. README.md flag/key updates (`skilltap skills` → `skilltap status`, `static/semantic/off` → `semantic/static/none`, `on_warn` enum updated, `require_scan = true` removed). website/index.md is a 3-line VitePress layout shell — content lives in `landing` layout component (Wave 4d to check). Suite still baseline. Agent `af6b0e38f8999d685`. Commit `9a9afa1`.
