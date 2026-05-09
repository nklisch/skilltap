# Design: Phase 21 — Plugin Storage and Data Model

## Overview

This phase adds persistent storage for installed plugins via `plugins.json`. It defines the stored record schema (distinct from Phase 20's parsing-time manifest schema), implements load/save I/O, and provides CRUD operations: add, remove, and per-component toggle.

Phase 21 creates 1 new schema file, 1 state module, updates 2 barrels, and adds 2 test files.

## Key Design Decisions

### Separate schema file from Phase 20

Phase 20 defined `PluginComponentSchema` in `schemas/plugin.ts` for the parsing-time manifest — it has `path`, `server` envelope, and `frontmatter`. The storage schema has different fields: `active`, flattened MCP config (`command`/`args`/`env`), and `platform` for agents. To avoid naming conflicts and keep the single-source-definitions pattern, the storage schemas live in `schemas/plugins.ts` (plural) with `Stored*` prefixes.

### Follow `loadInstalled`/`saveInstalled` pattern exactly

The config-io pattern from `config.ts` is well-tested:
- `loadX(projectRoot?)` → reads from global or project path, returns `Result<T>`, returns default if missing
- `saveX(data, projectRoot?)` → ensures dirs, writes JSON, returns `Result<void>`

`loadPlugins`/`savePlugins` follow this identically. Storage paths mirror `installed.json`:
- Global: `~/.config/skilltap/plugins.json`
- Project: `{projectRoot}/.agents/plugins.json`

### CRUD on the state module, not the schema

The state module (`plugin/state.ts`) owns all mutation logic: `addPlugin`, `removePlugin`, `toggleComponent`. These are pure functions that take a `PluginsJson` and return a new `PluginsJson` — no I/O. The caller (Phase 23's install flow) handles load → mutate → save. This keeps the state module testable with pure function tests.

### Conversion function from PluginManifest to PluginRecord

Phase 23 will need to convert a `PluginManifest` (from `detectPlugin`) into a `PluginRecord` (for `plugins.json`). This conversion bridges the two schemas and belongs in the state module. It takes the manifest plus install-time metadata (repo, ref, sha, scope, also, tap) and produces a ready-to-store record.

---

## Implementation Units

### Unit 1: Storage Schemas

**File**: `packages/core/src/schemas/plugins.ts`

```typescript
import { z } from "zod/v4";
import { PLUGIN_FORMATS } from "./plugin";

// --- Stored component schemas (different from manifest components) ---

export const StoredSkillComponentSchema = z.object({
  type: z.literal("skill"),
  name: z.string(),
  active: z.boolean().default(true),
});

export const StoredMcpComponentSchema = z.object({
  type: z.literal("mcp"),
  name: z.string(),
  active: z.boolean().default(true),
  command: z.string(),
  args: z.array(z.string()).default([]),
  env: z.record(z.string(), z.string()).default({}),
});

export const StoredAgentComponentSchema = z.object({
  type: z.literal("agent"),
  name: z.string(),
  active: z.boolean().default(true),
  platform: z.string().default("claude-code"),
});

export const StoredComponentSchema = z.discriminatedUnion("type", [
  StoredSkillComponentSchema,
  StoredMcpComponentSchema,
  StoredAgentComponentSchema,
]);

export const PluginRecordSchema = z.object({
  name: z.string(),
  description: z.string().default(""),
  format: z.enum(PLUGIN_FORMATS),
  repo: z.string().nullable(),
  ref: z.string().nullable(),
  sha: z.string().nullable(),
  scope: z.enum(["global", "project"]),
  also: z.array(z.string()).default([]),
  tap: z.string().nullable().default(null),
  components: z.array(StoredComponentSchema),
  installedAt: z.iso.datetime(),
  updatedAt: z.iso.datetime(),
  active: z.boolean().default(true),
});

export const PluginsJsonSchema = z.object({
  version: z.literal(1),
  plugins: z.array(PluginRecordSchema).default([]),
});

// --- Inferred types ---

export type StoredSkillComponent = z.infer<typeof StoredSkillComponentSchema>;
export type StoredMcpComponent = z.infer<typeof StoredMcpComponentSchema>;
export type StoredAgentComponent = z.infer<typeof StoredAgentComponentSchema>;
export type StoredComponent = z.infer<typeof StoredComponentSchema>;
export type PluginRecord = z.infer<typeof PluginRecordSchema>;
export type PluginsJson = z.infer<typeof PluginsJsonSchema>;
```

**Implementation Notes**:
- `PLUGIN_FORMATS` is imported from `./plugin` (Phase 20) — single-source-definitions pattern.
- `StoredMcpComponentSchema` flattens the MCP server config (command, args, env) directly onto the component. No `server` wrapper. This is the on-disk format from the SPEC. Note: only stdio servers are stored — HTTP MCP servers also have `command`/`args`/`env` fields for storage. Wait — actually, HTTP MCP servers have `type: "http"` and `url` in Phase 20. For storage, the SPEC only shows stdio fields. Since HTTP servers need different fields, add a `serverType` field to distinguish: actually, re-reading the SPEC example, MCP components only store `command`/`args`/`env`. HTTP servers will need to be handled when we get to Phase 22 (MCP injection). For now, follow the SPEC exactly — HTTP servers would store their config differently in Phase 22, but Phase 21's schema matches the SPEC's `plugins.json` format which only shows stdio. The conversion function in the state module will only store stdio servers' command/args/env.
- `StoredAgentComponentSchema` has `platform` defaulting to `"claude-code"` — extensible for future platforms.
- `PluginRecordSchema` follows `InstalledSkillSchema`'s field naming (`repo`, `ref`, `sha`, `scope`, `also`, `tap`, `installedAt`, `updatedAt`, `active`).

**Acceptance Criteria**:
- [ ] `PluginsJsonSchema.safeParse()` accepts the SPEC example JSON
- [ ] `PluginRecordSchema` requires `name`, `format`, `repo`, `ref`, `sha`, `scope`, `installedAt`, `updatedAt`
- [ ] `StoredComponentSchema` discriminates on `type`
- [ ] `StoredMcpComponentSchema` requires `command`
- [ ] Default values: `active=true`, `also=[]`, `tap=null`, `description=""`
- [ ] All types exported and inferrable

---

### Unit 2: Plugin State I/O

**File**: `packages/core/src/plugin/state.ts`

```typescript
import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import { getConfigDir, ensureDirs } from "../config";
import { parseWithResult } from "../schemas";
import { PluginsJsonSchema, type PluginsJson, type PluginRecord, type StoredComponent } from "../schemas/plugins";
import type { PluginManifest, PluginComponent } from "../schemas/plugin";
import { err, ok, type Result, UserError } from "../types";

// --- I/O ---

function getPluginsPath(projectRoot?: string): string;

export async function loadPlugins(
  projectRoot?: string,
): Promise<Result<PluginsJson, UserError>>;

export async function savePlugins(
  plugins: PluginsJson,
  projectRoot?: string,
): Promise<Result<void, UserError>>;

// --- Pure mutation functions ---

export function addPlugin(
  state: PluginsJson,
  record: PluginRecord,
): PluginsJson;

export function removePlugin(
  state: PluginsJson,
  pluginName: string,
): PluginsJson;

export function toggleComponent(
  state: PluginsJson,
  pluginName: string,
  componentType: StoredComponent["type"],
  componentName: string,
): Result<PluginsJson, UserError>;

export function findPlugin(
  state: PluginsJson,
  pluginName: string,
): PluginRecord | undefined;

// --- Conversion ---

export type PluginInstallMeta = {
  repo: string | null;
  ref: string | null;
  sha: string | null;
  scope: "global" | "project";
  also: string[];
  tap: string | null;
};

export function manifestToRecord(
  manifest: PluginManifest,
  meta: PluginInstallMeta,
): PluginRecord;
```

**Implementation Notes**:

**`getPluginsPath`**: Private. Returns `{projectRoot}/.agents/plugins.json` for project scope, `{configDir}/plugins.json` for global. Same pattern as `getInstalledPath` in `config.ts`.

**`loadPlugins`**: Follow `loadInstalled` exactly:
1. Compute path via `getPluginsPath(projectRoot)`.
2. Check if file exists via `Bun.file().exists()`.
3. If missing → return `ok({ version: 1 as const, plugins: [] })`.
4. Read JSON via `Bun.file().json()`.
5. Validate with `parseWithResult(PluginsJsonSchema, raw, "plugins.json")`.

**`savePlugins`**: Follow `saveInstalled` exactly:
1. If `projectRoot`, ensure `{projectRoot}/.agents/` exists via `mkdir({ recursive: true })`.
2. If global, call `ensureDirs()`.
3. Write via `Bun.write(path, JSON.stringify(plugins, null, 2))`.
4. Return `ok(undefined)`.

**`addPlugin`**: Pure function. Returns new `PluginsJson` with the record appended. If a plugin with the same name already exists, replace it (idempotent reinstall).

**`removePlugin`**: Pure function. Returns new `PluginsJson` with the named plugin filtered out. If not found, returns unchanged state (no error).

**`toggleComponent`**: Pure function. Finds the plugin by name, finds the component by type+name, flips its `active` field, updates the plugin's `updatedAt`. Returns `err` if plugin or component not found.

**`findPlugin`**: Pure helper. Returns the record or `undefined`.

**`manifestToRecord`**: Converts Phase 20's `PluginManifest` to a `PluginRecord`. For each `PluginComponent`:
- `type: "skill"` → `{ type: "skill", name, active: true }`
- `type: "mcp"` → extract server config: if `server.type === "stdio"`, store `{ type: "mcp", name: server.name, active: true, command: server.command, args: server.args, env: server.env }`. If `server.type === "http"`, store `{ type: "mcp", name: server.name, active: true, command: "", args: [], env: { _http_url: server.url } }` — a sentinel encoding that Phase 22 will understand. (Alternative: skip HTTP servers for now. I'll go with skip — simpler, and Phase 22 will handle HTTP MCP injection. The conversion function emits a debug log for skipped HTTP servers.)
- `type: "agent"` → `{ type: "agent", name, active: true, platform: "claude-code" }`

Sets `installedAt` and `updatedAt` to `new Date().toISOString()`.

**Acceptance Criteria**:
- [ ] `loadPlugins()` returns default empty state when file missing
- [ ] `loadPlugins()` parses valid plugins.json
- [ ] `loadPlugins()` returns error for invalid JSON
- [ ] `loadPlugins()` returns error for invalid schema (version: 99)
- [ ] `loadPlugins(projectRoot)` reads from project path
- [ ] `savePlugins()` writes valid JSON that round-trips through `loadPlugins()`
- [ ] `savePlugins(_, projectRoot)` creates `.agents/` dir and writes to project path
- [ ] `addPlugin()` appends a new plugin record
- [ ] `addPlugin()` replaces existing plugin with same name (idempotent)
- [ ] `removePlugin()` removes by name
- [ ] `removePlugin()` returns unchanged state if name not found
- [ ] `toggleComponent()` flips `active` on a specific component
- [ ] `toggleComponent()` updates `updatedAt` timestamp
- [ ] `toggleComponent()` returns error if plugin not found
- [ ] `toggleComponent()` returns error if component not found
- [ ] `findPlugin()` returns the record or undefined
- [ ] `manifestToRecord()` converts skills correctly
- [ ] `manifestToRecord()` converts stdio MCP servers correctly
- [ ] `manifestToRecord()` skips HTTP MCP servers
- [ ] `manifestToRecord()` converts agents correctly
- [ ] `manifestToRecord()` sets installedAt and updatedAt

---

### Unit 3: Barrel Updates

**File**: `packages/core/src/schemas/index.ts` — add:
```typescript
export * from "./plugins";
```

**File**: `packages/core/src/plugin/index.ts` — add:
```typescript
export {
  loadPlugins,
  savePlugins,
  addPlugin,
  removePlugin,
  toggleComponent,
  findPlugin,
  manifestToRecord,
  type PluginInstallMeta,
} from "./state";
```

**Acceptance Criteria**:
- [ ] All schemas importable from `@skilltap/core`
- [ ] All state functions importable from `@skilltap/core`

---

## Implementation Order

1. **Unit 1: Storage Schemas** (`schemas/plugins.ts`) — types needed by state module
2. **Unit 2: Plugin State I/O** (`plugin/state.ts`) — depends on Unit 1 + existing `config.ts`
3. **Unit 3: Barrel Updates** — wire up after Units 1-2

---

## Testing

### Schema Tests: `packages/core/src/schemas/plugins.test.ts`

Follow existing schema test pattern from `marketplace.test.ts`.

```
describe("PluginsJsonSchema")
  - accepts the SPEC example JSON (full round-trip)
  - accepts empty plugins array
  - rejects invalid version (99)
  - defaults plugins to [] when omitted

describe("PluginRecordSchema")
  - accepts valid record with all fields
  - defaults active to true
  - defaults also to []
  - defaults tap to null
  - defaults description to ""
  - rejects missing name
  - rejects missing format
  - rejects invalid scope

describe("StoredComponentSchema")
  - discriminates on type
  - accepts skill component (defaults active to true)
  - accepts mcp component with command/args/env
  - accepts agent component (defaults platform to "claude-code")
  - rejects unknown type
  - rejects mcp component without command
```

### State Module Tests: `packages/core/src/plugin/state.test.ts`

Uses temp dirs with `SKILLTAP_HOME`/`XDG_CONFIG_HOME` env isolation (same pattern as `config.test.ts`).

```
describe("loadPlugins")
  - returns default empty state when file missing
  - parses valid plugins.json
  - returns error for invalid JSON
  - returns error for invalid schema
  - reads from project path when projectRoot given

describe("savePlugins")
  - writes valid JSON that round-trips through loadPlugins
  - creates .agents/ dir for project scope
  - returns error on write failure (read-only dir)

describe("addPlugin")
  - appends a new plugin
  - replaces existing plugin with same name

describe("removePlugin")
  - removes by name
  - returns unchanged state if name not found

describe("toggleComponent")
  - flips active on a skill component
  - flips active on an mcp component
  - flips active on an agent component
  - updates updatedAt timestamp
  - returns error if plugin not found
  - returns error if component not found

describe("findPlugin")
  - returns plugin record by name
  - returns undefined if not found

describe("manifestToRecord")
  - converts manifest with skills, mcp, and agents
  - sets correct format and name
  - skips HTTP MCP servers
  - sets installedAt and updatedAt to current time
  - applies meta fields (repo, ref, sha, scope, also, tap)
```

---

## Verification Checklist

```bash
# Run all plugin-related tests
bun test packages/core/src/schemas/plugins.test.ts
bun test packages/core/src/plugin/state.test.ts

# Verify exports
bun -e "import { PluginsJsonSchema, loadPlugins, savePlugins, addPlugin, removePlugin, toggleComponent, manifestToRecord } from './packages/core/src/index'; console.log('all exports ok')"

# Full suite
bun test
```
