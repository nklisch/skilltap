# Design: Phase 24 — Plugin Management Commands

## Overview

CLI subcommand group `skilltap plugin` for listing, inspecting, toggling, and removing plugins. Also creates core functions for plugin removal and component toggling that handle filesystem + MCP config operations alongside `plugins.json` state changes.

Phase 24 creates 2 new core modules, 4 CLI command files, updates 2 barrels, and adds test files.

## Key Design Decisions

### Core functions for removal and toggle

Phase 21 provided pure state functions (`removePlugin`, `toggleComponent`) that only modify `plugins.json` data. Phase 24 needs full-lifecycle operations:
- **`removeInstalledPlugin`**: Remove skill directories, agent symlinks, MCP config entries, agent definition files, AND the plugins.json record.
- **`toggleInstalledComponent`**: Move skills to/from `.disabled/`, add/remove MCP config entries, move agent files to/from `.disabled/`, AND update plugins.json state.

These live in `core/src/plugin/lifecycle.ts` — they compose the state functions with filesystem and MCP operations.

### Simplified CLI commands

Each subcommand is thin — loads state, calls core, formats output. The `plugin/index.ts` command lists plugins (the `run` handler) and routes to subcommands.

### Plugin update deferred

Full plugin update (re-clone, re-detect, diff components, apply changes) is complex. Phase 24 only implements list, info, toggle, and remove. Update is tracked as Phase 25 polish.

### JSON output on all commands

Every command supports `--json` for agent-mode and scripting. The JSON format mirrors the `PluginRecord` structure from plugins.json.

---

## Implementation Units

### Unit 1: Core Plugin Lifecycle Functions

**File**: `packages/core/src/plugin/lifecycle.ts`

```typescript
import type { Result } from "../types";
import type { PluginRecord, StoredComponent, PluginsJson } from "../schemas/plugins";
import { UserError } from "../types";

export type RemovePluginOptions = {
  scope?: "global" | "project";
  projectRoot?: string;
};

/**
 * Remove an installed plugin: delete skill dirs + agent symlinks,
 * remove MCP entries from agent configs, delete agent definition files,
 * and remove the record from plugins.json.
 */
export async function removeInstalledPlugin(
  pluginName: string,
  options?: RemovePluginOptions,
): Promise<Result<PluginRecord, UserError>>;

export type ToggleComponentOptions = {
  projectRoot?: string;
};

export type ToggleResult = {
  component: StoredComponent;
  nowActive: boolean;
  /** Agents where MCP was added/removed (for MCP components) */
  mcpAgents: string[];
};

/**
 * Toggle a single component within an installed plugin.
 * Handles filesystem moves for skills and agents, MCP injection/removal for MCPs.
 * Updates plugins.json state.
 */
export async function toggleInstalledComponent(
  pluginName: string,
  componentType: StoredComponent["type"],
  componentName: string,
  options?: ToggleComponentOptions,
): Promise<Result<ToggleResult, UserError>>;
```

**Implementation Notes**:

**`removeInstalledPlugin`**:
1. Load `plugins.json` (try both global and project if no scope specified; if scope given, load that one).
2. Find the plugin record. Return error if not found.
3. For each component:
   - **skill**: `rm -rf` the skill dir at `skillInstallDir(name, scope, projectRoot)` or `skillDisabledDir` if inactive. Remove agent symlinks via `removeAgentSymlinks(name, record.also, scope, projectRoot)`.
   - **mcp**: Call `removeMcpServers({ pluginName, agents: record.also, scope, projectRoot })`.
   - **agent**: Delete the `.md` file at `join(base, ".claude", "agents", name + ".md")` or from `.disabled/` if inactive.
4. Remove record from state: `removePlugin(state, pluginName)`.
5. Save plugins.json.
6. Return the removed record.

**`toggleInstalledComponent`**:
1. Load plugins.json. Find plugin. Find component. Error if not found.
2. Based on component type and current `active` state:
   - **skill (deactivating)**: Move from `skillInstallDir` to `skillDisabledDir`. Remove agent symlinks.
   - **skill (activating)**: Move from `skillDisabledDir` to `skillInstallDir`. Recreate agent symlinks.
   - **mcp (deactivating)**: Call `removeMcpServers` for this plugin (only entries matching this server name).
   - **mcp (activating)**: Call `injectMcpServers` with just this one server.
   - **agent (deactivating)**: Move `.md` from `.claude/agents/` to `.claude/agents/.disabled/`.
   - **agent (activating)**: Move from `.disabled/` back to `.claude/agents/`.
3. Update state: `toggleComponent(state, pluginName, type, name)`.
4. Save plugins.json.
5. Return `ToggleResult`.

**Acceptance Criteria**:
- [ ] `removeInstalledPlugin` removes skill directories
- [ ] `removeInstalledPlugin` removes agent symlinks
- [ ] `removeInstalledPlugin` removes MCP entries from agent configs
- [ ] `removeInstalledPlugin` removes agent definition files
- [ ] `removeInstalledPlugin` removes plugins.json record
- [ ] `removeInstalledPlugin` returns error if plugin not found
- [ ] `removeInstalledPlugin` handles disabled components (skills in .disabled/)
- [ ] `toggleInstalledComponent` moves skills to .disabled/ on deactivate
- [ ] `toggleInstalledComponent` moves skills from .disabled/ on activate
- [ ] `toggleInstalledComponent` removes MCP on deactivate
- [ ] `toggleInstalledComponent` injects MCP on activate
- [ ] `toggleInstalledComponent` moves agents to .disabled/ on deactivate
- [ ] `toggleInstalledComponent` moves agents from .disabled/ on activate
- [ ] `toggleInstalledComponent` updates plugins.json state

---

### Unit 2: CLI Plugin Command Group

**File**: `packages/cli/src/commands/plugin/index.ts`

The `plugin` command lists plugins when run directly, and routes to subcommands.

```typescript
import { defineCommand } from "citty";

export default defineCommand({
  meta: { name: "plugin", description: "Manage installed plugins" },
  args: {
    global: { type: "boolean", description: "Show only global plugins", default: false },
    project: { type: "boolean", description: "Show only project plugins", default: false },
    json: { type: "boolean", description: "Output as JSON", default: false },
  },
  subCommands: {
    info: () => import("./info").then((m) => m.default),
    toggle: () => import("./toggle").then((m) => m.default),
    remove: () => import("./remove").then((m) => m.default),
  },
  async run({ args }) { /* list plugins */ },
});
```

**Implementation Notes**:
- Load plugins.json for global and project (via `tryFindProjectRoot`).
- If `--global` or `--project`, filter accordingly.
- Display table: Name, Components summary ("3 skills, 2 MCPs, 1 agent"), Source.
- If `--json`, output the raw plugins array.
- If no plugins, print empty state message.

---

### Unit 3: CLI Plugin Info Command

**File**: `packages/cli/src/commands/plugin/info.ts`

```typescript
export default defineCommand({
  meta: { name: "info", description: "Show plugin details" },
  args: {
    name: { type: "positional", description: "Plugin name", required: true },
    json: { type: "boolean", description: "Output as JSON", default: false },
  },
  async run({ args }) { /* show plugin details */ },
});
```

**Implementation Notes**:
- Load plugins.json (both scopes), find by name.
- Display: name, source, format, ref, scope, installed/updated dates.
- List components grouped by type with ✓/✗ for active/inactive.
- JSON mode: output the full `PluginRecord`.

---

### Unit 4: CLI Plugin Toggle Command

**File**: `packages/cli/src/commands/plugin/toggle.ts`

```typescript
export default defineCommand({
  meta: { name: "toggle", description: "Enable/disable plugin components" },
  args: {
    name: { type: "positional", description: "Plugin name", required: true },
    skills: { type: "boolean", description: "Toggle all skills", default: false },
    mcps: { type: "boolean", description: "Toggle all MCP servers", default: false },
    agents: { type: "boolean", description: "Toggle all agent definitions", default: false },
    json: { type: "boolean", description: "Output as JSON", default: false },
  },
  async run({ args }) { /* toggle components */ },
});
```

**Implementation Notes**:
- If category flags (`--skills`, `--mcps`, `--agents`): toggle all components of that type. For each active component of the type, deactivate it; for each inactive, activate it. (Bulk flip.)
- If no category flags: interactive mode — show `@clack/prompts` multiselect with all components, pre-checked for currently active ones. Diff selection vs current state to determine what to toggle.
- Call `toggleInstalledComponent` for each changed component.
- Display results: what was enabled, what was disabled.
- Agent mode: require category flags (no interactive picker).

---

### Unit 5: CLI Plugin Remove Command

**File**: `packages/cli/src/commands/plugin/remove.ts`

```typescript
export default defineCommand({
  meta: { name: "remove", description: "Remove a plugin and all components" },
  args: {
    name: { type: "positional", description: "Plugin name", required: true },
    yes: { type: "boolean", alias: "y", description: "Skip confirmation", default: false },
    json: { type: "boolean", description: "Output as JSON", default: false },
  },
  async run({ args }) { /* remove plugin */ },
});
```

**Implementation Notes**:
- Find plugin in plugins.json.
- Show summary: "Remove plugin X? N skills, M MCP servers, K agents will be removed."
- Confirm unless `--yes`.
- Call `removeInstalledPlugin`.
- Display: "Plugin X removed."
- Agent mode: auto-confirm, plain text output.

---

### Unit 6: Wire Into Main CLI

**File**: `packages/cli/src/index.ts` — add to `subCommands`:
```typescript
plugin: defineCommand({
  meta: { name: "plugin", description: "Manage installed plugins" },
  subCommands: {
    info: () => import("./commands/plugin/info").then((m) => m.default),
    toggle: () => import("./commands/plugin/toggle").then((m) => m.default),
    remove: () => import("./commands/plugin/remove").then((m) => m.default),
  },
}),
plugins: () => import("./commands/plugin/index").then((m) => m.default),
```

Wait — looking at how `skills` is done (the `skills` command has both `run` for listing and `subCommands`), the pattern is a single `defineCommand` with both. Let me follow that exact pattern:

```typescript
plugin: () => import("./commands/plugin/index").then((m) => m.default),
plugins: () => import("./commands/plugin/index").then((m) => m.default),  // alias
```

---

### Unit 7: Core Barrel Update

**File**: `packages/core/src/plugin/index.ts` — add:
```typescript
export { removeInstalledPlugin, toggleInstalledComponent, type RemovePluginOptions, type ToggleComponentOptions, type ToggleResult } from "./lifecycle";
```

---

## Implementation Order

1. **Unit 1**: `core/src/plugin/lifecycle.ts` — core removal and toggle functions
2. **Unit 1 tests**: `core/src/plugin/lifecycle.test.ts`
3. **Unit 2**: `cli/src/commands/plugin/index.ts` — list command
4. **Unit 3**: `cli/src/commands/plugin/info.ts`
5. **Unit 4**: `cli/src/commands/plugin/toggle.ts`
6. **Unit 5**: `cli/src/commands/plugin/remove.ts`
7. **Unit 6**: Wire into `cli/src/index.ts`
8. **Unit 7**: Core barrel update

---

## Testing

### Core Tests: `packages/core/src/plugin/lifecycle.test.ts`

```
describe("removeInstalledPlugin")
  - removes skill directories and agent symlinks
  - removes MCP entries from agent configs
  - removes agent definition files
  - removes plugin from plugins.json
  - returns error if plugin not found
  - handles disabled skills (in .disabled/)
  - handles plugin with no MCP or agents (skills only)

describe("toggleInstalledComponent")
  - deactivates a skill (moves to .disabled/, removes symlinks)
  - activates a skill (moves from .disabled/, creates symlinks)
  - deactivates an MCP server (removes from agent configs)
  - activates an MCP server (injects into agent configs)
  - deactivates an agent (moves to .disabled/)
  - activates an agent (moves from .disabled/)
  - updates plugins.json state
  - returns error if plugin not found
  - returns error if component not found
```

### CLI Tests: subprocess tests via `runSkilltap`

```
describe("skilltap plugin (list)")
  - shows empty state when no plugins
  - lists plugins with component counts
  - --json outputs JSON array

describe("skilltap plugin info")
  - shows plugin details with component status
  - exits 1 for unknown plugin
  - --json outputs plugin record

describe("skilltap plugin remove")
  - removes plugin and all components
  - --yes skips confirmation
  - exits 1 for unknown plugin
```

---

## Verification Checklist

```bash
bun test packages/core/src/plugin/lifecycle.test.ts
bun test  # full suite
bun run dev plugin           # test list
bun run dev plugin info X    # test info
bun run dev plugin remove X --yes  # test remove
```
