# Design: Plugin Detection for Direct Install + HTTP MCP Server Support

## Overview

Two fixes:

1. **Plugin detection is silently broken for direct installs** (`skilltap install ../skills`, `skilltap install user/repo`, etc.). The `onPluginDetected` callback exists in `InstallOptions` (line 90) but neither `runAgentMode()` nor `runInteractiveMode()` provides it. Plugin manifests are detected and then silently ignored — code falls through to skill scanning. Only tap-based plugin resolution works.

2. **HTTP MCP servers are silently dropped.** The manifest schema parses them correctly (`McpHttpServerSchema`), but `installPlugin()` skips them (`install.ts:115`), `manifestToRecord()` drops them (`state.ts:111`), `StoredMcpComponentSchema` has no `url` field, and `injectMcpServers()` only writes stdio shapes. HTTP MCP servers are just a URL endpoint — the agent config tells the harness where to connect, not how to run the process.

---

## Implementation Units

### Unit 1: Add `headers` field to `McpHttpServerSchema`

**File**: `packages/core/src/schemas/plugin.ts`

The manifest schema currently has no `headers` for HTTP servers. All major agent platforms (Claude Code, Cursor, Gemini) support `headers` for auth on HTTP MCP servers.

```typescript
// Before (line 21-25):
export const McpHttpServerSchema = z.object({
  type: z.literal("http"),
  name: z.string(),
  url: z.string(),
});

// After:
export const McpHttpServerSchema = z.object({
  type: z.literal("http"),
  name: z.string(),
  url: z.string(),
  headers: z.record(z.string(), z.string()).default({}),
});
```

Update `McpHttpServer` type (auto-inferred, no action needed — `z.infer` picks it up).

**Acceptance Criteria**:
- [ ] `McpHttpServerSchema` includes `headers` with default `{}`
- [ ] Existing manifests without `headers` parse successfully (default applies)

---

### Unit 2: Split `StoredMcpComponentSchema` into stdio + http variants

**File**: `packages/core/src/schemas/plugins.ts`

The stored schema must represent both server types. Add a `serverType` discriminator field with `default("stdio")` for backward compatibility with existing `plugins.json` files.

```typescript
import { z } from "zod/v4";
import { PLUGIN_FORMATS } from "./plugin";

export const StoredSkillComponentSchema = z.object({
  type: z.literal("skill"),
  name: z.string(),
  active: z.boolean().default(true),
});

export const StoredMcpStdioComponentSchema = z.object({
  type: z.literal("mcp"),
  serverType: z.literal("stdio").default("stdio"),
  name: z.string(),
  active: z.boolean().default(true),
  command: z.string(),
  args: z.array(z.string()).default([]),
  env: z.record(z.string(), z.string()).default({}),
});

export const StoredMcpHttpComponentSchema = z.object({
  type: z.literal("mcp"),
  serverType: z.literal("http"),
  name: z.string(),
  active: z.boolean().default(true),
  url: z.string(),
  headers: z.record(z.string(), z.string()).default({}),
});

// Union — can't use discriminatedUnion("type") because both have type: "mcp".
// Zod v4 union tries each schema in order; stdio (with default serverType) will
// match existing records that lack serverType.
export const StoredMcpComponentSchema = z.union([
  StoredMcpStdioComponentSchema,
  StoredMcpHttpComponentSchema,
]);

export const StoredAgentComponentSchema = z.object({
  type: z.literal("agent"),
  name: z.string(),
  active: z.boolean().default(true),
  platform: z.string().default("claude-code"),
});

// Changed from z.discriminatedUnion("type") to z.union because MCP now has
// two sub-schemas that both share type: "mcp".
export const StoredComponentSchema = z.union([
  StoredSkillComponentSchema,
  StoredMcpStdioComponentSchema,
  StoredMcpHttpComponentSchema,
  StoredAgentComponentSchema,
]);

// ... PluginRecordSchema and PluginsJsonSchema unchanged ...

export type StoredSkillComponent = z.infer<typeof StoredSkillComponentSchema>;
export type StoredMcpStdioComponent = z.infer<typeof StoredMcpStdioComponentSchema>;
export type StoredMcpHttpComponent = z.infer<typeof StoredMcpHttpComponentSchema>;
export type StoredMcpComponent = z.infer<typeof StoredMcpComponentSchema>;
export type StoredAgentComponent = z.infer<typeof StoredAgentComponentSchema>;
export type StoredComponent = z.infer<typeof StoredComponentSchema>;
export type PluginRecord = z.infer<typeof PluginRecordSchema>;
export type PluginsJson = z.infer<typeof PluginsJsonSchema>;
```

**Implementation Notes**:
- `serverType: z.literal("stdio").default("stdio")` is the backward-compat key — existing `plugins.json` entries without `serverType` parse as stdio.
- `StoredComponentSchema` changes from `z.discriminatedUnion` to `z.union`. This is necessary because both MCP sub-schemas share `type: "mcp"`. Zod v4's `z.union` tries schemas in order, which works correctly here since the schemas have distinct required fields (`command` vs `url`).
- Export both specific types (`StoredMcpStdioComponent`, `StoredMcpHttpComponent`) and the union type (`StoredMcpComponent`) since callers need to narrow by `serverType` at various points.

**Acceptance Criteria**:
- [ ] Existing plugins.json with stdio MCP entries (no `serverType`) parses successfully
- [ ] New HTTP MCP entries with `serverType: "http"` parse successfully
- [ ] `StoredMcpComponent` type is the union of both
- [ ] All existing tests that reference `StoredMcpComponent` still compile

---

### Unit 3: Extend `mcpServerToStored()` for HTTP servers

**File**: `packages/core/src/plugin/state.ts`

```typescript
// Before (line 37-46):
export function mcpServerToStored(server: McpStdioServer): StoredMcpComponent { ... }

// After — accept McpServerEntry (the union), return the appropriate variant:
import type { McpServerEntry, McpStdioServer, McpHttpServer } from "../schemas/plugin";
import type { StoredMcpComponent } from "../schemas/plugins";

export function mcpServerToStored(server: McpServerEntry): StoredMcpComponent {
  if (server.type === "http") {
    return {
      type: "mcp",
      serverType: "http",
      name: server.name,
      active: true,
      url: server.url,
      headers: server.headers ?? {},
    };
  }
  return {
    type: "mcp",
    serverType: "stdio",
    name: server.name,
    active: true,
    command: server.command,
    args: server.args ?? [],
    env: server.env ?? {},
  };
}
```

**Implementation Notes**:
- The function signature broadens from `McpStdioServer` to `McpServerEntry` (the union). This is backward-compatible — existing callers that pass stdio servers still work.

**Acceptance Criteria**:
- [ ] `mcpServerToStored({ type: "stdio", ... })` returns a stored stdio component with `serverType: "stdio"`
- [ ] `mcpServerToStored({ type: "http", ... })` returns a stored HTTP component with `serverType: "http"`

---

### Unit 4: Remove HTTP skip in `manifestToRecord()`

**File**: `packages/core/src/plugin/state.ts`

```typescript
// Before (lines 109-114):
if (server.type === "http") {
  debug("plugin", { skipped: "HTTP MCP server", name: server.name });
  continue;
}

// After — remove the skip, mcpServerToStored now handles both:
// (just delete the if block, keep the mcpServerToStored call)
```

The `for` loop at line 106 already calls `mcpServerToStored(server)` for stdio. After Unit 3, it handles HTTP too. Just delete the skip.

**Acceptance Criteria**:
- [ ] HTTP MCP servers appear in `manifestToRecord()` output `components[]`
- [ ] Stdio servers still work as before

---

### Unit 5: Remove HTTP skip in `installPlugin()` and inject HTTP servers

**File**: `packages/core/src/plugin/install.ts`

```typescript
// Before (lines 113-117):
for (const component of mcpComponents) {
  const server = component.server;
  if (server.type === "http") continue;
  storedMcpComponents.push(mcpServerToStored(server));
}

// After — remove the skip:
for (const component of mcpComponents) {
  storedMcpComponents.push(mcpServerToStored(component.server));
}
```

**Implementation Notes**:
- `mcpServerToStored` (Unit 3) now handles both types. The `InjectOptions.servers` type changes from `StoredMcpComponent[]` to `StoredMcpComponent[]` (same name, broader union). The injection function (Unit 6) handles both shapes.

**Acceptance Criteria**:
- [ ] HTTP MCP servers are included in `storedMcpComponents`
- [ ] HTTP MCP servers are passed to `injectMcpServers()` alongside stdio servers
- [ ] `PluginInstallResult.mcpAgents` includes agents where HTTP servers were injected

---

### Unit 6: Extend `injectMcpServers()` and `substituteMcpVars()` for HTTP

**File**: `packages/core/src/plugin/mcp-inject.ts`

**6a. Update `substituteMcpVars()`**:

```typescript
// Before (line 60-72):
export function substituteMcpVars(
  component: StoredMcpComponent,
  ctx: McpVarContext,
): StoredMcpComponent {
  return {
    ...component,
    command: substituteVars(component.command, ctx),
    args: component.args.map((a) => substituteVars(a, ctx)),
    env: Object.fromEntries(
      Object.entries(component.env).map(([k, v]) => [k, substituteVars(v, ctx)]),
    ),
  };
}

// After — branch on serverType:
export function substituteMcpVars(
  component: StoredMcpComponent,
  ctx: McpVarContext,
): StoredMcpComponent {
  if (component.serverType === "http") {
    return {
      ...component,
      url: substituteVars(component.url, ctx),
      headers: Object.fromEntries(
        Object.entries(component.headers).map(([k, v]) => [k, substituteVars(v, ctx)]),
      ),
    };
  }
  return {
    ...component,
    command: substituteVars(component.command, ctx),
    args: component.args.map((a) => substituteVars(a, ctx)),
    env: Object.fromEntries(
      Object.entries(component.env).map(([k, v]) => [k, substituteVars(v, ctx)]),
    ),
  };
}
```

**6b. Update injection loop in `injectMcpServers()`**:

```typescript
// Before (lines 195-204):
for (const server of servers) {
  const key = namespaceMcpServer(pluginName, server.name);
  const entry: Record<string, unknown> = {
    command: server.command,
    args: server.args,
  };
  if (Object.keys(server.env).length > 0) {
    entry.env = server.env;
  }
  mcpServers[key] = entry;
}

// After — branch on serverType:
for (const server of servers) {
  const key = namespaceMcpServer(pluginName, server.name);
  let entry: Record<string, unknown>;

  if (server.serverType === "http") {
    entry = { url: server.url };
    if (Object.keys(server.headers).length > 0) {
      entry.headers = server.headers;
    }
  } else {
    entry = { command: server.command, args: server.args };
    if (Object.keys(server.env).length > 0) {
      entry.env = server.env;
    }
  }

  mcpServers[key] = entry;
}
```

**Implementation Notes**:
- The written config shape matches what agents expect: stdio = `{ command, args, env? }`, http = `{ url, headers? }`.
- Variable substitution on `url` handles `${CLAUDE_PLUGIN_ROOT}` or `${CLAUDE_PLUGIN_DATA}` appearing in URLs (e.g., local dev servers referencing plugin data paths). Header values also get substituted (e.g., auth tokens referencing plugin data dir).

**Acceptance Criteria**:
- [ ] `injectMcpServers()` writes `{ url }` for HTTP servers
- [ ] `injectMcpServers()` writes `{ url, headers }` when headers are non-empty
- [ ] `injectMcpServers()` still writes `{ command, args, env? }` for stdio servers
- [ ] `substituteMcpVars()` substitutes variables in `url` and `headers` for HTTP
- [ ] `substituteMcpVars()` still works for stdio components

---

### Unit 7: Extend toggle lifecycle for HTTP MCP components

**File**: `packages/core/src/plugin/lifecycle.ts`

No structural changes needed. The toggle code at lines 157-196 already operates on `StoredMcpComponent` and delegates to `injectMcpServers()`/`removeMcpServers()`. Since:

- `removeMcpServers()` removes by namespace prefix — works for any server type
- `injectMcpServers()` (Unit 6) now handles HTTP
- The component lookup at line 121 casts to `StoredMcpComponent` — the union type works

However, the `as StoredMcpComponent` cast at line 186 needs verification that it works with the broadened type. Since `component` is already a `StoredComponent` narrowed to `type === "mcp"`, and `StoredMcpComponent` is now the union, this works without changes.

**Acceptance Criteria**:
- [ ] Toggling an HTTP MCP component off removes its entry from agent configs
- [ ] Toggling an HTTP MCP component on re-injects its `{ url, headers? }` entry
- [ ] Toggling a stdio MCP component still works as before

---

### Unit 8: Add `onPluginDetected` to `createInstallCallbacks()`

**File**: `packages/cli/src/ui/install-callbacks.ts`

Add three plugin callbacks to the `Pick` type and implement them:

```typescript
export function createInstallCallbacks(ctx: CallbackContext): {
  callbacks: Pick<
    InstallOptions,
    | "onWarnings"
    | "onSelectSkills"
    | "onSelectTap"
    | "onAlreadyInstalled"
    | "onSemanticWarnings"
    | "onOfferSemantic"
    | "onSemanticProgress"
    | "onStaticScanStart"
    | "onSemanticScanStart"
    | "onConfirmInstall"
    | "onDeepScan"
    | "onPluginDetected"     // NEW
    | "onPluginWarnings"     // NEW
    | "onPluginConfirm"      // NEW
  >;
  logScanResults(): void;
}
```

**New callback implementations** (add to the `callbacks` object):

```typescript
onPluginDetected: async (manifest: PluginManifest): Promise<"plugin" | "skills-only" | "cancel"> => {
  return withSpinnerPaused(s, async () => {
    const { select: selectPrompt, isCancel: isCancelPrompt } = await import("@clack/prompts");
    const summary = pluginComponentSummary(manifest);

    const decision = await selectPrompt({
      message: `Plugin detected: ${manifest.name} (${manifest.format}) — ${summary}`,
      options: [
        { value: "plugin" as const, label: "Install as plugin", hint: "skills + MCP servers + agents" },
        { value: "skills-only" as const, label: "Install skills only", hint: "ignore MCP servers and agents" },
        { value: "cancel" as const, label: "Cancel" },
      ],
    });
    if (isCancelPrompt(decision)) process.exit(130);
    return decision as "plugin" | "skills-only" | "cancel";
  });
},

onPluginWarnings: skipScan
  ? undefined
  : async (warnings: StaticWarning[], pluginName: string): Promise<boolean> => {
      return withSpinnerPaused(s, async () => {
        printWarnings(warnings, pluginName);
        if (onWarn === "fail") {
          errorLine(`Security warnings found in plugin ${pluginName} — aborting (--strict / on_warn=fail)`);
          process.exit(1);
        }
        if (onWarn === "allow") return true;
        const proceed = await confirmInstall(pluginName);
        if (proceed === false) process.exit(2);
        return true;
      });
    },

onPluginConfirm: yes
  ? undefined
  : async (manifest: PluginManifest): Promise<boolean> => {
      return withSpinnerPaused(s, async () => {
        const proceed = await confirmReadyInstall([manifest.name]);
        if (proceed === false) process.exit(2);
        return true;
      });
    },
```

**Implementation Notes**:
- `pluginComponentSummary(manifest)` is a new helper — counts components from the manifest and returns e.g. "3 skills, 2 MCPs, 1 agent". Similar to `componentSummary()` in `plugin-format.ts` but operates on `PluginManifest` instead of `PluginRecord`.
- When `--yes` is set, `onPluginDetected` should auto-accept. Extend the `yes` check: if `ctx.yes`, return `"plugin"` directly without prompting.
- `selectPrompt` from `@clack/prompts` — needs to use `footerSelect` from `./footer` to match the project's customized select wrapper.

**Acceptance Criteria**:
- [ ] Interactive mode shows a select prompt when a plugin manifest is detected
- [ ] User can choose "plugin", "skills-only", or "cancel"
- [ ] `--yes` auto-selects "plugin" (no prompt)
- [ ] Plugin security warnings are shown using the same `printWarnings` as skills
- [ ] Ctrl+C exits cleanly (via `isCancel` handling)

---

### Unit 9: Add `onPluginDetected` to agent mode

**File**: `packages/cli/src/commands/install.ts`

Add the three plugin callbacks to `runAgentMode()`'s `installSkill()` call:

```typescript
// Inside the installSkill() options object in runAgentMode() (after line 187):
onPluginDetected: async () => "plugin" as const,
onPluginWarnings: async (warnings: StaticWarning[]): Promise<boolean> => {
  agentSecurityBlock(warnings, []);
  process.exit(1);
  return false;
},
```

`onPluginConfirm` is omitted in agent mode (auto-proceed, per the callback-driven-options pattern).

**Also add plugin result handling** to both modes. After the `installSkill()` call succeeds, check for `pluginRecord`:

```typescript
// Agent mode — after the existing records loop (line 213-216):
if (result.value.pluginRecord) {
  const pr = result.value.pluginRecord;
  const summary = componentSummary(pr);
  process.stdout.write(`OK: Installed plugin ${pr.name} (${summary})\n`);
}

// Interactive mode — after the existing records loop (line 371-374):
if (result.value.pluginRecord) {
  const pr = result.value.pluginRecord;
  const summary = componentSummary(pr);
  successLine(`Installed plugin ${pr.name} (${summary})`);
}
```

**Implementation Notes**:
- Agent mode auto-accepts plugins (returns `"plugin"`) and hard-fails on security warnings — consistent with how agent mode handles skills.
- `componentSummary` already exists in `packages/cli/src/ui/plugin-format.ts` and works on `PluginRecord`.
- Import `componentSummary` and the necessary types.

**Acceptance Criteria**:
- [ ] Agent mode auto-installs detected plugins without prompting
- [ ] Agent mode hard-exits on plugin security warnings
- [ ] Agent mode prints `OK: Installed plugin <name> (<summary>)` on success
- [ ] Interactive mode prints `Installed plugin <name> (<summary>)` on success
- [ ] When user chooses "skills-only", normal skill scanning proceeds (existing behavior)

---

### Unit 10: Add `pluginComponentSummary()` helper

**File**: `packages/cli/src/ui/plugin-format.ts`

```typescript
import type { PluginManifest, PluginRecord } from "@skilltap/core";

export function componentSummary(record: PluginRecord): string {
  // ... existing implementation unchanged ...
}

/** Summarize components from a manifest (pre-install, for detection prompt). */
export function pluginComponentSummary(manifest: PluginManifest): string {
  const counts = { skill: 0, mcp: 0, agent: 0 };
  for (const c of manifest.components) counts[c.type]++;
  const parts: string[] = [];
  if (counts.skill > 0) parts.push(`${counts.skill} ${counts.skill === 1 ? "skill" : "skills"}`);
  if (counts.mcp > 0) parts.push(`${counts.mcp} ${counts.mcp === 1 ? "MCP" : "MCPs"}`);
  if (counts.agent > 0) parts.push(`${counts.agent} ${counts.agent === 1 ? "agent" : "agents"}`);
  return parts.join(", ") || "no components";
}
```

**Acceptance Criteria**:
- [ ] `pluginComponentSummary(manifest)` returns correct counts
- [ ] Works with manifests containing any mix of component types

---

## Implementation Order

1. **Unit 1** — `McpHttpServerSchema` headers field (schema change, no dependents yet)
2. **Unit 2** — `StoredMcpComponentSchema` split (schema change, core dependency for everything below)
3. **Unit 3** — `mcpServerToStored()` broadened (depends on Units 1-2)
4. **Unit 4** — Remove HTTP skip in `manifestToRecord()` (depends on Unit 3)
5. **Unit 5** — Remove HTTP skip in `installPlugin()` (depends on Unit 3)
6. **Unit 6** — `injectMcpServers()` + `substituteMcpVars()` HTTP support (depends on Unit 2)
7. **Unit 7** — Verify toggle lifecycle works (depends on Units 2, 6 — likely no code changes)
8. **Unit 10** — `pluginComponentSummary()` helper (no dependencies, needed by Unit 8)
9. **Unit 8** — `createInstallCallbacks()` plugin callbacks (depends on Unit 10)
10. **Unit 9** — Agent mode + CLI output for plugins (depends on Unit 8 patterns)

Units 1-7 (HTTP MCP) and Units 8-10 (plugin detection) are independent tracks and can be implemented in parallel.

---

## Testing

### Existing tests that need updates

**`packages/core/src/plugin/state.test.ts`**:
- `manifestToRecord` test "skips HTTP MCP servers" → update to verify HTTP servers ARE included with `serverType: "http"`
- Add test: existing plugins.json without `serverType` field parses as stdio (backward compat)

**`packages/core/src/plugin/install.test.ts`**:
- Test "skips HTTP MCP servers" → update to verify HTTP servers ARE injected
- Add test: plugin with mixed stdio + HTTP MCP servers injects both correctly

**`packages/core/src/plugin/mcp-inject.test.ts`**:
- Add test: `injectMcpServers()` writes `{ url }` for HTTP server
- Add test: `injectMcpServers()` writes `{ url, headers }` when headers non-empty
- Add test: mixed stdio + HTTP servers in same injection call
- Add test: `substituteMcpVars()` substitutes `${CLAUDE_PLUGIN_ROOT}` in `url`
- Add test: `substituteMcpVars()` substitutes vars in `headers` values

**`packages/core/src/plugin/lifecycle.test.ts`**:
- Add test: toggle HTTP MCP component off removes entry from agent config
- Add test: toggle HTTP MCP component on re-injects `{ url }` entry

### New tests

**`packages/core/src/plugin/install-integration.test.ts`**:
- Add test: `installSkill()` with `onPluginDetected` returning `"plugin"` for plugin with HTTP MCP servers — verify HTTP entries in agent config
- Existing tests already cover the callback flow; just add HTTP fixture variant

**`packages/core/src/plugin/e2e-lifecycle.test.ts`** (or new test in same file):
- Add test: full lifecycle with HTTP MCP server — install → verify `{ url }` in agent config → toggle off → verify removed → toggle on → verify re-injected → remove → verify clean

### CLI tests (if CLI subprocess tests exist for install)

- No new CLI subprocess tests needed for this change — the core integration tests cover the logic. The CLI changes are purely callback wiring and output formatting, which are covered by the existing interactive/agent mode patterns.

---

## Verification Checklist

```bash
# All existing tests pass (backward compat)
bun test

# Specific test files for the changed areas
bun test packages/core/src/plugin/state.test.ts
bun test packages/core/src/plugin/install.test.ts
bun test packages/core/src/plugin/mcp-inject.test.ts
bun test packages/core/src/plugin/lifecycle.test.ts
bun test packages/core/src/plugin/install-integration.test.ts
bun test packages/core/src/plugin/e2e-lifecycle.test.ts

# Type check
bunx tsc --noEmit

# Manual verification: install a local plugin with HTTP MCP server
bun run dev install ../skills  # should prompt "Plugin detected: ..."
```
