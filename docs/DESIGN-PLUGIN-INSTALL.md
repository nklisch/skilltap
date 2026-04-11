# Design: Phase 23 — Plugin Install Flow

## Overview

Wire plugin detection into `skilltap install`. After cloning a repo, detect plugin manifests before skill scanning. If a plugin is found, branch to a plugin-specific install flow that places skills, injects MCP configs, and places agent definitions — then records everything in `plugins.json`.

Phase 23 creates 1 new core module, extends 1 existing module, and adds tests.

## Key Design Decisions

### Callback-driven plugin detection in `installSkill`

Following the callback-driven-options pattern, add an `onPluginDetected` callback to `InstallOptions`. After cloning, `installSkill` calls `detectPlugin`. If a plugin is found, it calls `onPluginDetected(manifest)`. The callback returns `"plugin"` (install as plugin), `"skills-only"` (ignore plugin, install skills normally), or `"cancel"`. This keeps the extension minimal and lets the CLI layer handle the prompt.

### `installPlugin` as a standalone core function

`installPlugin(contentDir, manifest, options)` handles the entire plugin install: skill placement, MCP injection, agent definition placement, security scanning, and plugins.json recording. It does NOT clone — the caller provides `contentDir` (already cloned by `installSkill`'s fetch step). This avoids duplicating clone logic.

### Skills in a plugin go through simplified placement

Plugin skills skip conflict checking against `installed.json` (the plugin owns them). They go through security scanning and are placed in `.agents/skills/` with agent symlinks, same as standalone skills. But they're recorded in `plugins.json` only, not `installed.json`.

### Agent definitions placed in `.claude/agents/`

Agent `.md` files from the plugin's `agents/` directory are copied to `.claude/agents/` (global: `~/.claude/agents/`, project: `.claude/agents/`). Claude Code-only for now.

---

## Implementation Units

### Unit 1: Plugin Install Core Function

**File**: `packages/core/src/plugin/install.ts`

```typescript
import type { Result } from "../types";
import type { PluginManifest } from "../schemas/plugin";
import type { PluginRecord, StoredMcpComponent } from "../schemas/plugins";
import type { StaticWarning } from "../security";
import { UserError, ScanError } from "../types";

export type PluginInstallOptions = {
  scope: "global" | "project";
  projectRoot?: string;
  also?: string[];
  skipScan?: boolean;
  /** Called when static security warnings found. Return true to proceed. */
  onWarnings?: (warnings: StaticWarning[], pluginName: string) => Promise<boolean>;
  /** Called before placement for confirmation. Return false to cancel. */
  onConfirm?: (manifest: PluginManifest) => Promise<boolean>;
  /** Repo URL for recording */
  repo: string | null;
  /** Git ref */
  ref: string | null;
  /** Git SHA */
  sha: string | null;
  /** Tap name if installed from a tap */
  tap: string | null;
};

export type PluginInstallResult = {
  record: PluginRecord;
  warnings: StaticWarning[];
  /** List of agents where MCP was injected */
  mcpAgents: string[];
  /** Number of agent definitions placed */
  agentDefsPlaced: number;
};

/**
 * Install a plugin from a pre-cloned directory.
 *
 * 1. Security scan all plugin content (skills dirs + agent .md files)
 * 2. Place skills in .agents/skills/ with agent symlinks
 * 3. Inject MCP server configs into target agent config files
 * 4. Place agent definitions in .claude/agents/
 * 5. Record plugin in plugins.json
 */
export async function installPlugin(
  contentDir: string,
  manifest: PluginManifest,
  options: PluginInstallOptions,
): Promise<Result<PluginInstallResult, UserError | ScanError>>;
```

**Implementation Notes**:

**Step 1 — Security scan**: If `!skipScan`, run `scanStatic(contentDir)` on the entire plugin directory. This catches malicious content in skills, agent .md files, and MCP command strings. If warnings found, call `onWarnings`. If callback returns false or callback absent, abort.

**Step 2 — Place skills**: For each `PluginSkillComponent` in the manifest:
- Source: `join(contentDir, component.path)` (the path is relative to plugin root)
- Dest: `skillInstallDir(component.name, scope, projectRoot)`
- Copy via `cp -a`. Use `wrapShell` for error handling.
- Create agent symlinks via `createAgentSymlinks(name, destDir, also, scope, projectRoot)`.

**Step 3 — Inject MCP**: Collect all MCP components from the manifest record. Call `injectMcpServers({ pluginName: manifest.name, servers: mcpComponents, agents: also, scope, projectRoot, vars })`. The `vars` context:
- `pluginRoot`: the install directory (where the plugin content ends up — same as contentDir for standalone, but use the first skill's install parent for multi-skill)
- `pluginData`: `join(getConfigDir(), "plugin-data", manifest.name)`

Actually, for plugin root, use `contentDir` during install since that's where the plugin files live. The `CLAUDE_PLUGIN_ROOT` variable is meant to reference the installed plugin location. Since we copy skills individually (not the whole plugin dir), the plugin root after install should be the canonical `.agents/skills/` parent. But this variable is more relevant for MCP commands that need to reference files within the plugin — so use `contentDir` for now, and Phase 25 can refine this.

**Step 4 — Place agent definitions**: For each `PluginAgentComponent`:
- Source: `join(contentDir, component.path)` (e.g., `agents/reviewer.md`)
- Dest: `join(base, ".claude", "agents", component.name + ".md")` where base is `globalBase()` for global, `projectRoot` for project
- Ensure dest dir exists: `mkdir(dirname(dest), { recursive: true })`
- Copy via `Bun.write(dest, Bun.file(src))`

**Step 5 — Record**: Use `manifestToRecord(manifest, { repo, ref, sha, scope, also, tap })` from the state module. Then `loadPlugins(projectRoot) → addPlugin(state, record) → savePlugins(newState, projectRoot)`.

**Return**: `PluginInstallResult` with the record, warnings, list of MCP-injected agents, and agent def count.

**Acceptance Criteria**:
- [ ] Places skills in `.agents/skills/{name}/` with correct scope
- [ ] Creates agent symlinks for skills
- [ ] Injects MCP servers into agent config files (namespaced)
- [ ] Places agent .md files in `.claude/agents/`
- [ ] Creates `.claude/agents/` directory if it doesn't exist
- [ ] Records plugin in `plugins.json`
- [ ] Does not record skills in `installed.json`
- [ ] Runs security scan on entire plugin directory
- [ ] Calls `onWarnings` when static warnings found
- [ ] Skips security scan when `skipScan=true`
- [ ] Returns error when security callback returns false
- [ ] Handles plugin with only skills (no MCP, no agents)
- [ ] Handles plugin with only MCP (no skills, no agents)

---

### Unit 2: Extend `installSkill` with Plugin Detection

**File**: `packages/core/src/install.ts` — modify existing

Add to `InstallOptions`:
```typescript
/** Called when a plugin manifest is detected after cloning. Return "plugin" to install as plugin,
 *  "skills-only" to ignore the plugin and install skills normally, or "cancel" to abort. */
onPluginDetected?: (manifest: PluginManifest) => Promise<"plugin" | "skills-only" | "cancel">;
/** Called when static security warnings found during plugin install. Return true to proceed. */
onPluginWarnings?: (warnings: StaticWarning[], pluginName: string) => Promise<boolean>;
/** Called before plugin placement for confirmation. Return false to cancel. */
onPluginConfirm?: (manifest: PluginManifest) => Promise<boolean>;
```

Add to `InstallResult`:
```typescript
/** If a plugin was installed, the plugin record. */
pluginRecord?: PluginRecord;
```

**Insertion point**: After the content is fetched and `contentDir` is resolved (after line ~491 `debug("content fetched", ...)`), insert:

```typescript
// 4. Plugin detection — before skill scanning
const pluginResult = await detectPlugin(contentDir);
if (!pluginResult.ok) return pluginResult;

if (pluginResult.value && options.onPluginDetected) {
  const decision = await options.onPluginDetected(pluginResult.value);
  if (decision === "cancel") return err(new UserError("Install cancelled."));
  if (decision === "plugin") {
    const pluginInstallResult = await installPlugin(contentDir, pluginResult.value, {
      scope: options.scope,
      projectRoot: options.projectRoot,
      also,
      skipScan: options.skipScan,
      onWarnings: options.onPluginWarnings,
      onConfirm: options.onPluginConfirm,
      repo: cloneUrl ?? resolved.url,
      ref: finalRef ?? null,
      sha,
      tap: effectiveTap,
    });
    if (!pluginInstallResult.ok) return pluginInstallResult;
    return ok({
      records: [],
      warnings: pluginInstallResult.value.warnings,
      semanticWarnings: [],
      updates: [],
      pluginRecord: pluginInstallResult.value.record,
    });
  }
  // decision === "skills-only" → fall through to normal skill scanning
}
```

If `onPluginDetected` is not provided (e.g., existing callers), plugin detection is silently skipped and the normal flow continues. This maintains backward compatibility.

**Acceptance Criteria**:
- [ ] Existing `installSkill` behavior unchanged when `onPluginDetected` not provided
- [ ] Plugin detected and `onPluginDetected` returns `"plugin"` → runs `installPlugin`, returns `pluginRecord`
- [ ] Plugin detected and `onPluginDetected` returns `"skills-only"` → normal skill install
- [ ] Plugin detected and `onPluginDetected` returns `"cancel"` → returns error
- [ ] No plugin detected → normal skill install (regardless of callback)
- [ ] `pluginRecord` is undefined when no plugin was installed

---

### Unit 3: Barrel Update

**File**: `packages/core/src/plugin/index.ts` — add:
```typescript
export { installPlugin, type PluginInstallOptions, type PluginInstallResult } from "./install";
```

---

## Implementation Order

1. **Unit 1**: `plugin/install.ts` — core plugin install function
2. **Unit 2**: Modify `install.ts` — add plugin detection + callback
3. **Unit 3**: Barrel update

---

## Testing

### Unit Tests: `packages/core/src/plugin/install.test.ts`

Uses temp dirs with env isolation (SKILLTAP_HOME, XDG_CONFIG_HOME). Creates plugin directory structures in temp dirs, calls `installPlugin` directly.

```
describe("installPlugin")
  - places skills in .agents/skills/ with correct names
  - creates agent symlinks for skills when also specified
  - injects MCP servers into agent config files
  - places agent .md files in .claude/agents/
  - creates .claude/agents/ directory when missing
  - records plugin in plugins.json
  - does not record skills in installed.json
  - runs security scan and calls onWarnings
  - skips security scan when skipScan=true
  - aborts when onWarnings returns false
  - handles plugin with only skills (no MCP, no agents)
  - handles plugin with only MCP servers
  - handles empty plugin (no components)
  - returns correct mcpAgents list
  - returns correct agentDefsPlaced count
```

### Integration Tests: `packages/core/src/plugin/install-integration.test.ts`

Uses fixture repos (createClaudePluginRepo, createCodexPluginRepo). Tests the full flow from `installSkill` with `onPluginDetected` callback.

```
describe("installSkill with plugin detection")
  - detects Claude Code plugin and installs via callback
  - detects Codex plugin and installs via callback
  - falls through to skill install when callback returns "skills-only"
  - cancels when callback returns "cancel"
  - normal skill install when no onPluginDetected callback
  - plugin record included in InstallResult
```

---

## Verification Checklist

```bash
bun test packages/core/src/plugin/install.test.ts
bun test packages/core/src/plugin/install-integration.test.ts
bun test  # full suite
```
