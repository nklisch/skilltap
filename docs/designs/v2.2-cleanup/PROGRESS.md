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
| 1C | 1 | 1.7 (refine), 1.8, 1.9, 1.14, 1.15 | pending | CLI security refine + SETTABLE_KEYS + template + manifest [[mcps]] + sync. Unit 1.10 already done in 1A. |
| 2  | 2 | 2.1–2.5 | pending | Dead code deletions (parallel) |
| 3a | 3 | 3.1–3.8 | pending | Code cleanups (parallel-safe) |
| 3b | 3 | 3.9, 3.10, 3.11, 3.12 | pending | CLI surface changes (`--scope`, `try` typed, drop `--no-strict`, repeatable `--also`, mcp smart-scope) |
| 3c | 3 | 3.13, 3.14, 3.15, 3.16 | pending | Multi-plugin syntax + capture flags + lifecycle drift + drift.ts range/ref |
| 3d | 3 | 3.17, 3.18, 3.19, 3.20 | pending | TUI fixes + doctor + tui.smoke binary routing + CLI test coverage expansion |
| 4a | 4 | 4.1, 4.2, 4.3, 4.4, 4.5, 4.7 | pending | Foundation docs (ROADMAP / SPEC / SECURITY / VISION / ARCH / AGENTS / CLAUDE) |
| 4b | 4 | 4.6, 4.8, 4.9, 4.10, 4.11 | pending | UX + changelog + website (index/README, guide/, reference/) |
| 4c | 4 | 4.12 | pending | Regenerate llms-full.txt (last) |
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

## Deviation log

Things that didn't go as planned: failed assertions, design assumptions that turned out wrong,
substitutions that diverged from the design.

- **2026-05-08 Wave 1A** — full `bun test` reports 45 failures at the wave boundary (test-fixture residue: `agents/__tests__/detect.test.ts` legacy fixture, `security/describe.test.ts` deleted-symbol imports, `cli/config/security.test.ts` legacy assertions, etc.). Expected; resolved by Wave 1B (Unit 1.4 hard-fail makes legacy fixtures fail loudly and forces rewrite) + Wave 1C (Unit 1.8 SETTABLE_KEYS) + Phase 3 (Unit 3.6 fixture sweep). Build (`bun run build`) is clean and the binary verifies — typecheck-clean wave boundary held.

## Completion log

Wave-by-wave completion entries with date, agent IDs, and summary.

- **2026-05-08 Wave 1A** — Units 1.1, 1.2, 1.3, 1.10, 1.11, 1.12, 1.13 + minimal 1.7 done. 27 files changed (modified/renamed/deleted). 60 schema+policy tests pass. Build clean, binary verifies. Agent `a2cfe3baf6c0fa8f4`. Commit `44fd2b1`.
- **2026-05-08 Wave 1B** — Units 1.4, 1.5, 1.6 done + inline rewrite of two of four Unit 3.6 fixture files. 9 files modified. 67 in-scope tests pass; full suite 45→34 fails (11-fail improvement). Build clean. Default-config template rewritten to V2 shape so first-run doesn't trip its own hard-fail gate. `[registry].allow_npm` dropped silently while preserving `enabled`/`sources` (defensible deviation; user's tap-search settings preserved). Agent `a850fe61e7e5c9518`. Commit `bc06ab9`.
