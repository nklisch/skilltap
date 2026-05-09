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
| 1A | 1 | 1.1, 1.2, 1.3, 1.11, 1.12, 1.13 | pending | Schema + policy promotion + sweep exports |
| 1B | 1 | 1.4, 1.5, 1.6 | pending | loadConfig hard-fail + migrate + round-trip test |
| 1C | 1 | 1.7, 1.8, 1.9, 1.10, 1.14, 1.15 | pending | CLI security + SETTABLE_KEYS + template + scanner consumers + manifest [[mcps]] + sync |
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

(empty)

## Deviation log

Things that didn't go as planned: failed assertions, design assumptions that turned out wrong,
substitutions that diverged from the design.

(empty)

## Completion log

Wave-by-wave completion entries with date, agent IDs, and summary.

(empty)
