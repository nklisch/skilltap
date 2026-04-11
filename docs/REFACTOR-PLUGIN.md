# Refactor Plan: Plugin Code (Phases 20-25)

## Overview

The plugin feature was built across 6 phases, each by a delegated agent. The agents produced clean, well-tested code, but the incremental process created duplicated logic that a single author would have extracted into shared helpers. This plan consolidates the most impactful duplications — each step is safe, testable, and commits independently.

## Refactor Steps

### Step 1: Extract `scopeBase` helper for scope-based path resolution

**Priority**: High
**Risk**: Low
**Files**: `packages/core/src/paths.ts`, `packages/core/src/symlink.ts`, `packages/core/src/plugin/mcp-inject.ts`, `packages/core/src/plugin/install.ts`, `packages/core/src/plugin/lifecycle.ts`

**Current State** (8 occurrences across 5 files):
```typescript
const base = scope === "global" ? globalBase() : (projectRoot ?? process.cwd());
```

**Target State**:

In `packages/core/src/paths.ts`, add:
```typescript
export function scopeBase(
  scope: "global" | "project",
  projectRoot?: string,
): string {
  return scope === "global" ? globalBase() : (projectRoot ?? process.cwd());
}
```

Then replace all 8 inline ternaries with `scopeBase(scope, projectRoot)`.

**Implementation Notes**:
- `skillInstallDir` and `skillDisabledDir` already import from `./fs` — just add the helper and refactor.
- `symlink.ts` uses the same ternary inside `symlinkPath` but with `"global" | "project"` from a broader scope type — verify the types align.
- `mcp-inject.ts`, `install.ts`, and `lifecycle.ts` all import from `../paths` already or can trivially add the import.

**Acceptance Criteria**:
- [ ] `bun test` passes (all 1680+ tests)
- [ ] `scopeBase` exported from `@skilltap/core`
- [ ] No remaining `scope === "global" ? globalBase() :` ternaries in plugin/ files
- [ ] `grep -r 'globalBase()' packages/core/src/plugin/` returns zero results

---

### Step 2: Extract `mcpServerToStored` conversion function

**Priority**: High
**Risk**: Low
**Files**: `packages/core/src/plugin/state.ts`, `packages/core/src/plugin/install.ts`

**Current State** — same conversion duplicated in two places:

`state.ts:127-139` (inside `manifestToRecord`):
```typescript
const server = component.server;
if (server.type === "http") {
  debug("plugin", { skipped: "HTTP MCP server", name: server.name });
  continue;
}
components.push({
  type: "mcp", name: server.name, active: true,
  command: server.command, args: server.args ?? [], env: server.env ?? {},
});
```

`install.ts:114-125` (inside `installPlugin`):
```typescript
const server = component.server;
if (server.type === "http") continue;
storedMcpComponents.push({
  type: "mcp", name: server.name, active: true,
  command: server.command, args: server.args ?? [], env: server.env ?? {},
});
```

**Target State**:

In `packages/core/src/plugin/state.ts`, add:
```typescript
import type { McpStdioServer } from "../schemas/plugin";

export function mcpServerToStored(server: McpStdioServer): StoredMcpComponent {
  return {
    type: "mcp",
    name: server.name,
    active: true,
    command: server.command,
    args: server.args ?? [],
    env: server.env ?? {},
  };
}
```

Then both `manifestToRecord` and `installPlugin` call `mcpServerToStored(server)` instead of building the object inline. The HTTP skip guard stays at each call site (it's a control flow decision, not a conversion concern).

**Implementation Notes**:
- Export from `plugin/index.ts` barrel.
- The `install.ts` import changes from only importing `StoredMcpComponent` type to also importing `mcpServerToStored`.
- Update the barrel in `plugin/index.ts`.

**Acceptance Criteria**:
- [ ] `bun test` passes
- [ ] No inline `{ type: "mcp", name: server.name, active: true, command: server.command, ... }` construction in plugin/ files
- [ ] `mcpServerToStored` exported from `@skilltap/core`

---

### Step 3: Extract generic `loadJsonState` / `saveJsonState` helpers

**Priority**: High
**Risk**: Medium (touches config.ts — a critical module)
**Files**: `packages/core/src/config.ts`, `packages/core/src/plugin/state.ts`

**Current State** — `loadInstalled`/`saveInstalled` and `loadPlugins`/`savePlugins` are nearly identical:

```typescript
// loadInstalled / loadPlugins (same pattern):
const f = Bun.file(path);
if (!await f.exists()) return ok(defaultValue);
let raw;
try { raw = await f.json(); } catch (e) { return err(new UserError(`Invalid JSON in ${label}: ${e}`)); }
return parseWithResult(Schema, raw, label);

// saveInstalled / savePlugins (same pattern):
if (projectRoot) {
  try { await mkdir(join(projectRoot, ".agents"), { recursive: true }); }
  catch (e) { return err(new UserError(`Failed to create .agents directory: ${e}`)); }
} else {
  const dirsResult = await ensureDirs();
  if (!dirsResult.ok) return dirsResult;
}
try { await Bun.write(path, JSON.stringify(data, null, 2)); return ok(undefined); }
catch (e) { return err(new UserError(`Failed to save ${label}: ${e}`)); }
```

**Target State**:

Create `packages/core/src/json-state.ts`:
```typescript
import { z } from "zod/v4";
import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import { ensureDirs } from "./config";
import { parseWithResult } from "./schemas";
import { err, ok, type Result, UserError } from "./types";

export async function loadJsonState<T>(
  path: string,
  schema: z.ZodType<T>,
  label: string,
  defaultValue: T,
): Promise<Result<T, UserError>> {
  const f = Bun.file(path);
  if (!(await f.exists())) return ok(defaultValue);
  let raw: unknown;
  try { raw = await f.json(); }
  catch (e) { return err(new UserError(`Invalid JSON in ${label}: ${e}`)); }
  return parseWithResult(schema, raw, label);
}

export async function saveJsonState(
  path: string,
  data: unknown,
  label: string,
  projectRoot?: string,
): Promise<Result<void, UserError>> {
  if (projectRoot) {
    try { await mkdir(join(projectRoot, ".agents"), { recursive: true }); }
    catch (e) { return err(new UserError(`Failed to create .agents directory: ${e}`)); }
  } else {
    const dirsResult = await ensureDirs();
    if (!dirsResult.ok) return dirsResult;
  }
  try {
    await Bun.write(path, JSON.stringify(data, null, 2));
    return ok(undefined);
  } catch (e) { return err(new UserError(`Failed to save ${label}: ${e}`)); }
}
```

Then `loadInstalled` becomes:
```typescript
export async function loadInstalled(projectRoot?: string): Promise<Result<InstalledJson>> {
  return loadJsonState(getInstalledPath(projectRoot), InstalledJsonSchema, "installed.json", { version: 1 as const, skills: [] });
}
```

And `loadPlugins`, `saveInstalled`, `savePlugins` similarly reduce to one-liners.

**Implementation Notes**:
- Add `export * from "./json-state"` to `index.ts` barrel.
- `ensureDirs` is imported from `./config` — watch for circular dependency. `json-state.ts` imports from `./config`, and `config.ts` would import from `./json-state`. Break the cycle by keeping `ensureDirs` in `config.ts` but having `json-state.ts` accept it as a parameter, or move `ensureDirs` + `getConfigDir` to a separate module. The cleanest approach: `saveJsonState` accepts an `ensureDirs` function as a parameter (injectable dep pattern).
- Actually, simpler: `saveJsonState` just takes a `globalDirsReady: () => Promise<Result<void>>` parameter. `saveInstalled` passes `ensureDirs`, `savePlugins` passes `ensureDirs`. No circular import.

**Acceptance Criteria**:
- [ ] `bun test` passes
- [ ] `loadInstalled`/`saveInstalled` delegate to `loadJsonState`/`saveJsonState`
- [ ] `loadPlugins`/`savePlugins` delegate to `loadJsonState`/`saveJsonState`
- [ ] No duplicate load/save boilerplate
- [ ] No circular imports

---

### Step 4: Add `AGENT_DEF_PATHS` constant and `agentDefPath` / `agentDefDisabledPath` helpers

**Priority**: Medium
**Risk**: Low
**Files**: `packages/core/src/paths.ts`, `packages/core/src/plugin/install.ts`, `packages/core/src/plugin/lifecycle.ts`

**Current State** — `.claude/agents/` path hardcoded in 3+ locations:
```typescript
// install.ts:153
join(base, ".claude", "agents", component.name + ".md")

// lifecycle.ts:91-92
join(base, ".claude", "agents", name + ".md")
join(base, ".claude", "agents", ".disabled", name + ".md")
```

**Target State**:

In `packages/core/src/symlink.ts` (alongside `AGENT_PATHS`):
```typescript
export const AGENT_DEF_PATHS: Record<string, string> = {
  "claude-code": ".claude/agents",
};
```

In `packages/core/src/paths.ts`:
```typescript
import { AGENT_DEF_PATHS } from "./symlink";

export function agentDefPath(
  name: string, platform: string, scope: "global" | "project", projectRoot?: string,
): string | null {
  const relDir = AGENT_DEF_PATHS[platform];
  if (!relDir) return null;
  return join(scopeBase(scope, projectRoot), relDir, name + ".md");
}

export function agentDefDisabledPath(
  name: string, platform: string, scope: "global" | "project", projectRoot?: string,
): string | null {
  const relDir = AGENT_DEF_PATHS[platform];
  if (!relDir) return null;
  return join(scopeBase(scope, projectRoot), relDir, ".disabled", name + ".md");
}
```

Then `install.ts` and `lifecycle.ts` call these instead of constructing paths inline.

**Acceptance Criteria**:
- [ ] `bun test` passes
- [ ] No `.claude/agents` string literals in plugin/ source files
- [ ] `AGENT_DEF_PATHS` exported from `@skilltap/core`
- [ ] Adding a new agent platform's agent def path = one line in the map

---

### Step 5: Extract `SKILLTAP_MCP_PREFIX` constant

**Priority**: Medium
**Risk**: Low
**Files**: `packages/core/src/plugin/mcp-inject.ts`

**Current State** — `"skilltap:"` repeated 4 times:
```typescript
return `skilltap:${pluginName}:${serverName}`;
key.startsWith("skilltap:");
if (!key.startsWith("skilltap:")) return null;
const prefix = `skilltap:${pluginName}:`;
```

**Target State**:
```typescript
const SKILLTAP_MCP_PREFIX = "skilltap:";

export function namespaceMcpServer(pluginName: string, serverName: string): string {
  return `${SKILLTAP_MCP_PREFIX}${pluginName}:${serverName}`;
}
export function isNamespacedKey(key: string): boolean {
  return key.startsWith(SKILLTAP_MCP_PREFIX);
}
// etc.
```

**Acceptance Criteria**:
- [ ] `bun test` passes
- [ ] `"skilltap:"` literal appears exactly once (the constant definition)

---

### Step 6: Extract shared `componentSummary` CLI helper

**Priority**: Medium
**Risk**: Low
**Files**: `packages/cli/src/commands/plugin/index.ts`, `packages/cli/src/commands/plugin/remove.ts`

**Current State** — two identical `componentSummary` functions with slightly different MCP labels.

**Target State**:

Create `packages/cli/src/ui/plugin-format.ts`:
```typescript
import type { PluginRecord } from "@skilltap/core";

export function componentSummary(record: PluginRecord): string {
  const counts = { skill: 0, mcp: 0, agent: 0 };
  for (const c of record.components) counts[c.type]++;
  const parts: string[] = [];
  if (counts.skill > 0) parts.push(`${counts.skill} ${counts.skill === 1 ? "skill" : "skills"}`);
  if (counts.mcp > 0) parts.push(`${counts.mcp} ${counts.mcp === 1 ? "MCP" : "MCPs"}`);
  if (counts.agent > 0) parts.push(`${counts.agent} ${counts.agent === 1 ? "agent" : "agents"}`);
  return parts.join(", ") || "no components";
}
```

Then both CLI commands import from `../../ui/plugin-format`.

**Acceptance Criteria**:
- [ ] `bun test` passes
- [ ] No `componentSummary` function in plugin command files
- [ ] Shared helper used by both list and remove commands

---

### Step 7: Extract shared skill scan + component construction helper for parsers

**Priority**: Low
**Risk**: Low
**Files**: `packages/core/src/plugin/parse-claude.ts`, `packages/core/src/plugin/parse-codex.ts`

**Current State** — skill scanning and `PluginSkillComponent` construction is duplicated between both parsers (4 identical `.push()` blocks, same `try/catch` for scanner).

**Target State**:

Add to `packages/core/src/plugin/parse-common.ts` (or inline in an existing file):
```typescript
import { relative, resolve } from "node:path";
import { scan } from "../scanner";
import type { PluginManifest } from "../schemas/plugin";

export async function discoverSkills(
  pluginDir: string,
  skillPaths?: string | string[],
): Promise<PluginManifest["components"]> {
  const components: PluginManifest["components"] = [];
  const paths = skillPaths
    ? (Array.isArray(skillPaths) ? skillPaths : [skillPaths])
    : [pluginDir];

  for (const p of paths) {
    const absDir = p === pluginDir ? pluginDir : resolve(pluginDir, p);
    let skills: Awaited<ReturnType<typeof scan>> = [];
    try { skills = await scan(absDir); } catch {}
    for (const skill of skills) {
      components.push({
        type: "skill", name: skill.name,
        path: relative(pluginDir, skill.path), description: skill.description,
      });
    }
  }
  return components;
}
```

Then both parsers call `discoverSkills(pluginDir, manifest.skills)`.

Similarly, the MCP fallback pattern (`parseMcpJson(join(pluginDir, ".mcp.json"))` + wrap in `{ type: "mcp", server }`) could be a `discoverMcpServers(pluginDir, mcpServersField)` helper.

**Acceptance Criteria**:
- [ ] `bun test` passes
- [ ] `parse-claude.ts` and `parse-codex.ts` each reduced by ~30 lines

---

## Implementation Order

1. **Step 1**: `scopeBase` helper (no dependencies, unlocks Steps 4)
2. **Step 2**: `mcpServerToStored` (independent)
3. **Step 5**: `SKILLTAP_MCP_PREFIX` constant (independent, trivial)
4. **Step 6**: `componentSummary` CLI helper (independent, trivial)
5. **Step 4**: `AGENT_DEF_PATHS` + path helpers (depends on Step 1 for `scopeBase`)
6. **Step 3**: `loadJsonState`/`saveJsonState` (most complex, do last)
7. **Step 7**: Parser deduplication (lowest priority, do if time permits)

Steps 1, 2, 5, 6 are independent and could be done in parallel. Steps 4 depends on 1. Step 3 is the most impactful but riskiest. Step 7 is optional polish.
