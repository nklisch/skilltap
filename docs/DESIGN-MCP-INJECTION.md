# Design: Phase 22 — MCP Config Injection

## Overview

This phase adds the ability to inject MCP server entries into agent platform config files (Claude Code, Cursor, Codex, Gemini, Windsurf). It provides `injectMcpServers` and `removeMcpServers` functions that Phase 23's install flow will call. The design is data-driven: a single `MCP_AGENT_CONFIGS` registry maps agent IDs to file paths, and generic read/write functions handle all agents uniformly.

Phase 22 creates 1 new source file, updates 1 barrel, and adds 1 test file.

## Key Design Decisions

### Data-driven, not adapter classes

All five agents use the same JSON structure for MCP: `{ "mcpServers": { "name": { command, args, env? } } }`. The only difference is the file path. A per-agent adapter class hierarchy would be over-engineered for what amounts to a path lookup. Instead, `MCP_AGENT_CONFIGS` is a typed constant (like `AGENT_PATHS`) that maps agent IDs to their config file relative paths. One generic implementation handles all agents.

Adding a new agent means adding one line to the registry.

### Two config file structures, same logic

- **Settings files** (Claude Code `settings.json`, Gemini `settings.json`): Multi-purpose JSON objects where `mcpServers` is one key among many. Read the whole object, merge the `mcpServers` key, write back — preserving all other keys.
- **MCP-only files** (Cursor `mcp.json`, Codex `mcp.json`, Windsurf `mcp.json`): JSON where `mcpServers` is the primary content.

Both types are handled identically: read JSON → get/create `mcpServers` object → add/remove entries → write back. No special-casing needed.

### Namespacing prevents collisions

Injected keys use `skilltap:<plugin-name>:<server-name>`. This is unambiguous — `removeMcpServers` can delete only skilltap-owned entries by matching the `skilltap:` prefix. User-configured servers are never touched.

### Backup before first write

Before modifying any agent config file for the first time (in a session), create a `.skilltap.bak` copy. Only one backup per file — if a backup already exists, don't overwrite it (the original pre-skilltap state is the most valuable backup).

### Variable substitution at injection time

MCP configs from plugins may contain `${CLAUDE_PLUGIN_ROOT}` and `${CLAUDE_PLUGIN_DATA}`. These are resolved when servers are injected, not when parsed. The resolved values depend on install-time context (plugin install path, plugin data dir).

---

## Implementation Units

### Unit 1: MCP Config Injection Module

**File**: `packages/core/src/plugin/mcp-inject.ts`

```typescript
import { mkdir } from "node:fs/promises";
import { dirname, join } from "node:path";
import { getConfigDir } from "../config";
import { globalBase } from "../fs";
import { debug } from "../debug";
import type { StoredMcpComponent } from "../schemas/plugins";
import { err, ok, type Result, UserError } from "../types";

// --- Agent MCP config registry ---

/**
 * Maps agent IDs to their MCP config file path (relative to base).
 * Base is globalBase() for global scope, projectRoot for project scope.
 */
export const MCP_AGENT_CONFIGS: Record<string, string> = {
  "claude-code": ".claude/settings.json",
  cursor: ".cursor/mcp.json",
  codex: ".codex/mcp.json",
  gemini: ".gemini/settings.json",
  windsurf: ".windsurf/mcp.json",
};

// --- Namespacing ---

export function namespaceMcpServer(
  pluginName: string,
  serverName: string,
): string;

export function isNamespacedKey(key: string): boolean;

export function parseNamespacedKey(
  key: string,
): { pluginName: string; serverName: string } | null;

// --- Variable substitution ---

export type McpVarContext = {
  pluginRoot: string;
  pluginData: string;
};

export function substituteMcpVars(
  component: StoredMcpComponent,
  ctx: McpVarContext,
): StoredMcpComponent;

// --- Config file I/O ---

export function mcpConfigPath(
  agent: string,
  scope: "global" | "project",
  projectRoot?: string,
): string | null;

async function readConfigJson(
  path: string,
): Promise<Result<Record<string, unknown>, UserError>>;

async function writeConfigJson(
  path: string,
  data: Record<string, unknown>,
): Promise<Result<void, UserError>>;

async function backupIfNeeded(path: string): Promise<void>;

// --- Public API ---

export type InjectOptions = {
  pluginName: string;
  servers: StoredMcpComponent[];
  agents: string[];
  scope: "global" | "project";
  projectRoot?: string;
  vars?: McpVarContext;
};

/**
 * Inject MCP server entries into target agent config files.
 * Creates the config file if it doesn't exist.
 * Backs up before first modification.
 * Idempotent — re-injection replaces existing entries with same key.
 *
 * Returns the list of agents that were successfully injected into.
 */
export async function injectMcpServers(
  options: InjectOptions,
): Promise<Result<string[], UserError>>;

export type RemoveOptions = {
  pluginName: string;
  agents: string[];
  scope: "global" | "project";
  projectRoot?: string;
};

/**
 * Remove all MCP server entries for a plugin from target agent config files.
 * Only removes entries with the skilltap: namespace prefix.
 * If no skilltap entries remain and the config was MCP-only, leaves an empty mcpServers object.
 */
export async function removeMcpServers(
  options: RemoveOptions,
): Promise<Result<string[], UserError>>;

/**
 * List all skilltap-managed MCP server keys in an agent's config file.
 * Returns empty array if config file doesn't exist.
 */
export async function listMcpServers(
  agent: string,
  scope: "global" | "project",
  projectRoot?: string,
): Promise<Result<string[], UserError>>;
```

**Implementation Notes**:

**`namespaceMcpServer(pluginName, serverName)`**: Returns `"skilltap:" + pluginName + ":" + serverName`. Pure string concatenation.

**`isNamespacedKey(key)`**: Returns `key.startsWith("skilltap:")`.

**`parseNamespacedKey(key)`**: Splits on `:` — `["skilltap", pluginName, ...serverNameParts]`. The server name may contain colons (unlikely but safe), so rejoin everything after the second colon. Returns null if key doesn't start with `"skilltap:"` or has fewer than 3 segments.

**`substituteMcpVars(component, ctx)`**: Returns a new `StoredMcpComponent` with `${CLAUDE_PLUGIN_ROOT}` and `${CLAUDE_PLUGIN_DATA}` replaced in `command`, each item of `args`, and each value of `env`. Use a simple string replace — no regex needed. The keys are fixed strings.

**`mcpConfigPath(agent, scope, projectRoot?)`**: Looks up `MCP_AGENT_CONFIGS[agent]`. If not found, returns `null`. Computes full path: `join(base, configRelPath)` where base is `globalBase()` for global, `projectRoot ?? process.cwd()` for project.

**`readConfigJson(path)`**: 
1. Check if file exists via `Bun.file(path).exists()`.
2. If not → return `ok({})` (empty config — file will be created on write).
3. Read text, `JSON.parse`. If malformed → `err`.
4. If not a plain object → `err`.
5. Return `ok(parsed)`.

**`writeConfigJson(path, data)`**:
1. Ensure parent directory exists: `mkdir(dirname(path), { recursive: true })`.
2. `Bun.write(path, JSON.stringify(data, null, 2))`.
3. Return `ok(undefined)`.

**`backupIfNeeded(path)`**: 
1. Check if `path` exists via `Bun.file(path).exists()`.
2. If yes, check if `path + ".skilltap.bak"` exists.
3. If backup doesn't exist → copy via `Bun.write(backupPath, Bun.file(path))`.
4. If backup already exists or file doesn't exist → no-op.

**`injectMcpServers(options)`**:
1. Apply variable substitution to each server if `vars` provided.
2. For each agent in `options.agents`:
   a. Compute config path via `mcpConfigPath`. Skip if agent not in registry (debug log, continue).
   b. Call `readConfigJson`.
   c. Call `backupIfNeeded` (before any modification).
   d. Get or create `mcpServers` object from config: `config.mcpServers ??= {}`.
   e. For each server: set `config.mcpServers[namespaceMcpServer(pluginName, server.name)] = { command, args, env }`. Only include `env` if non-empty. Don't include `type`, `name`, or `active` — those are storage fields, not config file fields.
   f. Call `writeConfigJson`.
3. Return `ok(injectedAgents)`.

**`removeMcpServers(options)`**:
1. For each agent in `options.agents`:
   a. Compute config path. Skip if not in registry.
   b. Call `readConfigJson`. If no file → skip (nothing to remove).
   c. Get `mcpServers` from config. If not an object → skip.
   d. Delete all keys that start with `skilltap:${pluginName}:`.
   e. Call `writeConfigJson`.
2. Return `ok(removedAgents)`.

**`listMcpServers(agent, scope, projectRoot?)`**:
1. Compute config path. Return `ok([])` if agent not in registry.
2. Read config. Return `ok([])` if file doesn't exist.
3. Get `mcpServers` object. Return all keys that `isNamespacedKey`.

**Acceptance Criteria**:
- [ ] `namespaceMcpServer("dev-toolkit", "database")` → `"skilltap:dev-toolkit:database"`
- [ ] `isNamespacedKey("skilltap:dev-toolkit:database")` → `true`
- [ ] `isNamespacedKey("my-server")` → `false`
- [ ] `parseNamespacedKey("skilltap:dev-toolkit:database")` → `{ pluginName: "dev-toolkit", serverName: "database" }`
- [ ] `parseNamespacedKey("user-server")` → `null`
- [ ] `substituteMcpVars` replaces `${CLAUDE_PLUGIN_ROOT}` in command, args, and env values
- [ ] `substituteMcpVars` replaces `${CLAUDE_PLUGIN_DATA}` in command, args, and env values
- [ ] `substituteMcpVars` returns component unchanged when no variables present
- [ ] `mcpConfigPath("claude-code", "global")` → `~/.claude/settings.json`
- [ ] `mcpConfigPath("cursor", "project", "/my/project")` → `/my/project/.cursor/mcp.json`
- [ ] `mcpConfigPath("unknown-agent", "global")` → `null`
- [ ] `injectMcpServers` creates config file when it doesn't exist
- [ ] `injectMcpServers` adds mcpServers key to existing settings.json without losing other keys
- [ ] `injectMcpServers` creates backup before first modification
- [ ] `injectMcpServers` does not overwrite existing backup
- [ ] `injectMcpServers` is idempotent — re-injection produces same result
- [ ] `injectMcpServers` only includes env when non-empty
- [ ] `injectMcpServers` skips unknown agent IDs gracefully
- [ ] `injectMcpServers` applies variable substitution when vars provided
- [ ] `removeMcpServers` removes only skilltap-prefixed entries for the given plugin
- [ ] `removeMcpServers` preserves user-configured servers
- [ ] `removeMcpServers` preserves other skilltap plugin entries
- [ ] `removeMcpServers` handles missing config file gracefully
- [ ] `listMcpServers` returns all skilltap-namespaced keys
- [ ] `listMcpServers` returns empty array for missing config

---

### Unit 2: Barrel Update

**File**: `packages/core/src/plugin/index.ts` — add:
```typescript
export {
  MCP_AGENT_CONFIGS,
  namespaceMcpServer,
  isNamespacedKey,
  parseNamespacedKey,
  substituteMcpVars,
  mcpConfigPath,
  injectMcpServers,
  removeMcpServers,
  listMcpServers,
  type InjectOptions,
  type RemoveOptions,
  type McpVarContext,
} from "./mcp-inject";
```

**Acceptance Criteria**:
- [ ] All functions and types importable from `@skilltap/core`

---

## Implementation Order

1. **Unit 1: MCP Config Injection Module** (`plugin/mcp-inject.ts`) — all logic in one file
2. **Unit 2: Barrel Update** — wire up exports

---

## Testing

### Tests: `packages/core/src/plugin/mcp-inject.test.ts`

Uses temp dirs with env isolation for global scope tests, regular temp dirs for project scope tests.

**Pure function tests (no I/O):**

```
describe("namespaceMcpServer")
  - formats "skilltap:plugin:server"
  - handles plugin names with hyphens

describe("isNamespacedKey")
  - returns true for skilltap-prefixed key
  - returns false for plain key
  - returns false for empty string

describe("parseNamespacedKey")
  - parses valid namespaced key
  - returns null for non-skilltap key
  - handles server name containing colons

describe("substituteMcpVars")
  - replaces ${CLAUDE_PLUGIN_ROOT} in command
  - replaces ${CLAUDE_PLUGIN_ROOT} in args
  - replaces ${CLAUDE_PLUGIN_DATA} in env values
  - replaces multiple variables in same string
  - returns unchanged component when no variables present
  - handles both variables in same component
```

**I/O tests (temp dirs):**

```
describe("mcpConfigPath")
  - returns correct path for each agent + scope combo
  - returns null for unknown agent

describe("injectMcpServers")
  - creates new mcp.json with mcpServers key
  - adds to existing settings.json preserving other keys
  - namespaces server names correctly
  - creates backup before first modification
  - does not overwrite existing backup
  - is idempotent (same result on re-injection)
  - skips unknown agent IDs
  - applies variable substitution
  - omits env when empty
  - handles multiple servers for multiple agents
  - creates parent directories when needed

describe("removeMcpServers")
  - removes only entries matching plugin name
  - preserves user-configured servers
  - preserves other skilltap plugin entries
  - handles missing config file
  - handles config with no mcpServers key

describe("listMcpServers")
  - lists skilltap-namespaced keys
  - returns empty array for missing config
  - excludes non-skilltap keys

describe("round-trip integration")
  - inject → list → remove → list (empty) for all 5 agents
```

---

## Verification Checklist

```bash
# Run Phase 22 tests
bun test packages/core/src/plugin/mcp-inject.test.ts

# Verify exports
bun -e "import { injectMcpServers, removeMcpServers, MCP_AGENT_CONFIGS, namespaceMcpServer } from './packages/core/src/index'; console.log('all exports ok')"

# Full suite
bun test
```
