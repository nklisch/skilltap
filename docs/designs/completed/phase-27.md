# Phase 27 — State Consolidation + Migration

## Goal

`state.json` exists as v2.0's unified state file. `skilltap migrate` reads v1.0 (`installed.json` + `plugins.json` + `config.toml` with v1.0 keys) and writes the v2.0 layout. Existing v1.0 paths keep working until Phase 31 cuts over — Phase 27 is additive plus a soft startup hint.

## Decisions

### Soft startup gate (deviation from ROADMAP 27.7)

The roadmap says "v2.0 startup detection — error with hint and exit." Doing that *now* would break every v1.0 user immediately, since v1.0 code paths still read `installed.json` / `plugins.json` (until Phase 31). Compromise: print a one-time soft warning to stderr if v1.0 state exists and `state.json` doesn't; exit 0 and continue normally. The hard error gates land in Phase 31 when v1.0 reads are removed.

### Co-existence of state.json and v1.0 files

Phase 27 writes `state.json` (via migrate) but doesn't make any v1.0 code path read it. `installed.json` / `plugins.json` continue to be read by v1.0 install/list/etc. until Phase 31. After `migrate`, the v1.0 files are renamed to `.v1.bak` so they're preserved but inert.

If a user runs `migrate` and then runs `skilltap install` (still v1.0 logic), the `installed.json` file is gone — the v1.0 code's `loadInstalled` will return its default empty state (file-not-exists branch already exists in `loadJsonState`). New installs will write a fresh `installed.json` alongside `state.json`. **This means v1.0 commands run after migrate will diverge from `state.json`.** That's accepted for this phase — the migrate command is intended as a one-shot before users adopt v2.0 commands. Phase 31's cutover replaces v1.0 readers with v2.0 readers, restoring consistency.

### What `migrate` translates

| v1.0 input | v2.0 output | Notes |
|---|---|---|
| `~/.config/skilltap/installed.json` | `state.skills` | Schemas are compatible |
| `~/.config/skilltap/plugins.json` | `state.plugins` | Schemas are compatible |
| `<projectRoot>/.agents/installed.json` | `<projectRoot>/.agents/state.json` (skills) | Per-scope migration |
| `<projectRoot>/.agents/plugins.json` | merged into the same `state.json` (plugins) | One file per scope |
| `[security.human]` + `[security.agent]` | `[security]` | Take stricter on conflict; warn user |
| `[security.threshold]`, `.max_size`, `.ollama_model`, `.agent_cli` | dropped | Not represented in v2.0 simple model — log warning |
| `[[security.overrides]]` with `preset = "none"` | `[security].trust = [match, ...]` | Trust list replaces preset overrides |
| `[[security.overrides]]` with other presets | dropped | Warn, log lossy items |
| `[agent-mode] enabled` | `[agent].default` | `enabled = true` → `default = true` |
| `[agent-mode] scope` | dropped | v2.0 has no per-mode scope |
| `[[taps]]` with `type = "http"` | error, list affected | User must convert or remove |
| Everything else under `[security]`, `[updates]`, `[telemetry]` | passed through | Identical schemas |

### Migration is opt-in via the command

`skilltap migrate` is the only entry point. v2.0 install/sync/etc. don't auto-migrate (per roadmap). `skilltap migrate` re-running on already-migrated state is a no-op with a clear message.

## Implementation Units

### Unit 1 — `core/src/state/paths.ts`

```typescript
import { join } from "node:path";
import { getConfigDir } from "../config";

export function getStatePath(projectRoot?: string): string {
  return projectRoot
    ? join(projectRoot, ".agents", "state.json")
    : join(getConfigDir(), "state.json");
}
```

### Unit 2 — `core/src/state/load.ts`

`loadState(projectRoot?)` → `Result<State>` using existing `loadJsonState` helper. Default: `{ version: 2, skills: [], plugins: [], mcpServers: [] }`.

### Unit 3 — `core/src/state/save.ts`

`saveState(state, projectRoot?)` → `Result<void>` using `saveJsonState` + `ensureDirs`.

### Unit 4 — `core/src/state/migrate-v1.ts`

Pure function: given v1.0 `InstalledJson` + `PluginsJson`, produce v2.0 `State`.

```typescript
export function migrateV1State(
  installed: InstalledJson,
  plugins: PluginsJson,
): State {
  return {
    version: 2,
    skills: installed.skills,
    plugins: plugins.plugins,
    mcpServers: [],
  };
}
```

`InstalledSkillSchema` and `PluginRecordSchema` shapes are reused unchanged in `StateSchema`, so this is a structural merge with no field translation.

### Unit 5 — `core/src/migrate/config-v1.ts`

```typescript
export interface ConfigMigrationResult {
  v2: ConfigV2;                         // new config to write
  warnings: string[];                   // user-visible warnings about lossy translation
  httpTapsRejected: TapEntry[];         // listed HTTP taps; migration errors if non-empty
}

export function migrateV1Config(rawV1: unknown): Result<ConfigMigrationResult, UserError>
```

Translation logic per the table above. Returns `Result.err` only on truly unparseable input. HTTP taps are returned in `httpTapsRejected` so the orchestrator can decide to error.

### Unit 6 — `core/src/migrate/detect.ts`

```typescript
export interface V1StateMarkers {
  scope: "global" | "project";
  installedJson: string | null;          // present path if file exists
  pluginsJson: string | null;
  configToml: string | null;             // present + flagged as v1.0 by content
  configHasV1Keys: boolean;              // [security.human], [security.agent], [agent-mode], [[security.overrides]]
}

export async function detectV1State(projectRoot?: string): Promise<V1StateMarkers>
```

### Unit 7 — `core/src/migrate/run.ts`

Orchestrator: detect → translate config → translate state → write files → rename originals to `.v1.bak`. Returns a structured `MigrationReport` for the CLI to print.

```typescript
export interface MigrationReport {
  scope: "global" | "project" | "both";
  files: {
    written: string[];                   // state.json, config.toml
    renamed: { from: string; to: string }[];  // *.v1.bak
  };
  warnings: string[];                    // lossy translations
  alreadyMigrated: boolean;              // true = no-op run
}

export async function runMigrate(options: { projectRoot?: string }): Promise<Result<MigrationReport, UserError>>
```

If `httpTapsRejected.length > 0`, returns an error before any writes (no partial migration).

### Unit 8 — `cli/src/commands/migrate.ts`

Citty command wrapping `runMigrate()`. Renders the `MigrationReport` to stdout with @clack/prompts log/section helpers. Supports `--json` for machine-readable output. Exits 0 on success, 1 on error.

### Unit 9 — Soft startup hint in `cli/src/index.ts`

After `SKIP_STARTUP_ARGS` filtering, also call a new `runV1Detection()` that:

- Calls `detectV1State()` for global and the inferred project root.
- If any v1.0 markers exist AND the corresponding `state.json` doesn't exist, prints a single dim line to stderr:
  ```
  ↑  v1.0 state detected. Run 'skilltap migrate' to upgrade to v2.0 (preview).
  ```
- Exits silently otherwise. No hard error. Skips for `migrate`, `--version`, `--help`, etc.

### Unit 10 — Tests

- `state/load.test.ts`, `state/save.test.ts` — round-trip via tmp dir.
- `state/migrate-v1.test.ts` — fixture v1 state → expected v2 state (incl. empty plugins.json case).
- `migrate/config-v1.test.ts` — every translation row in the table; warning emission; HTTP tap rejection path.
- `migrate/run.test.ts` — integration: synthesize v1 fs, run migrate, verify renames + state.json contents + .v1.bak presence.

Where helpful, reuse `@skilltap/test-utils` for tmp dirs.

## Verification

```bash
bun test packages/core/src/state/
bun test packages/core/src/migrate/
bun test packages/cli/src/commands/migrate.test.ts  # if added
```

Plus manual sanity:

```bash
# Synthesize a v1.0 setup
mkdir -p ~/.config/skilltap
cat > ~/.config/skilltap/installed.json <<'EOF'
{ "version": 1, "skills": [] }
EOF
echo '{ "version": 1, "plugins": [] }' > ~/.config/skilltap/plugins.json

bun run dev migrate
# Expect: state.json written, *.v1.bak files present, friendly summary
```

## Out of Scope

- Wiring v2.0 reader/writer paths into install/sync/list — Phase 28+.
- Removing v1.0 schemas — Phase 31+.
- Hard error on v1.0 detection (currently soft) — Phase 31.
- v2.0 config writes from `skilltap config set` — Phase 32 wires the agent.* keys; broader v2.0 config writing comes with Phase 31.
