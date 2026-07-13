---
source_handle: pi-hooks-health-installed-package
fetched: 2026-07-12
source_path: /home/nathan/.pi/agent/npm/node_modules/@hsingjui/pi-hooks/
provenance: source-direct
substrate_confidence: source-direct
---

# Installed `@hsingjui/pi-hooks` package artifact

The package is materialized under Pi's user npm checkout root at
`~/.pi/agent/npm/node_modules/@hsingjui/pi-hooks/`. It is a single Pi extension
whose entry point is declared in `package.json` under `pi.extensions`. The
artifact bundles the extension source plus English and Chinese READMEs. The
extension adapts Claude Code's hook configuration format to Pi's extension
event system; it does not ship a standalone binary and exports only its own
package manifest.

## Anchored excerpts

**`package.json` (installed), identity block:**

```json
{
  "name": "@hsingjui/pi-hooks",
  "version": "0.0.2",
  "description": "Claude Code-compatible command hooks for the Pi coding agent",
  "license": "MIT",
  "repository": { "type": "git", "url": "git+https://github.com/hsingjui/pi-hooks.git" },
  "pi": { "extensions": ["./src/pi-hooks.ts"] },
  "peerDependencies": { "@earendil-works/pi-coding-agent": "*" }
}
```

The `files` field declares `["src", "README.md"]`, yet the installed tree also
contains root-level duplicates (`config.ts`, `executor.ts`, `pi-hooks.ts`, etc.)
and an empty top-level `hooks/` directory. The canonical entry the manifest
points at is `src/pi-hooks.ts`; the root-level `pi-hooks.ts` is empty (zero
bytes). Treat `src/` as the authoritative layout.

**`src/pi-hooks.ts` (registered extension entry):**

```ts
export default function (pi: ExtensionAPI) {
  const shared = createHookContext(pi);
  registerSessionHooks(pi, shared);
  registerCompactHooks(pi, shared);
  registerPromptHooks(pi, shared);
  registerStopHooks(pi, shared);
  registerToolHooks(pi, shared);
}
```

**`src/config.ts`, read-only configuration model with no writes:**

```ts
export const GLOBAL_SETTINGS_PATH = path.join(os.homedir(), ".pi", "agent", "settings.json");
// ...
export function loadSettings(cwd: string) {
  const projectSettingsPath = path.join(cwd, ".pi", "settings.json");
  const globalSettings = readSettingsFile(GLOBAL_SETTINGS_PATH);
  const projectSettings = readSettingsFile(projectSettingsPath);
  const sourcePaths = [GLOBAL_SETTINGS_PATH, projectSettingsPath].filter((p) => existsSync(p));
  const hooks = mergeHooks(globalSettings?.hooks, projectSettings?.hooks);
  if (!hooks) return { settings: undefined, sourcePaths };
  return { settings: { hooks }, sourcePaths };
}
```

`readSettingsFile` uses `existsSync` + `readFileSync` only; no write path exists
in `config.ts`. When neither scope defines a `hooks` key, `loadSettings`
returns `{ settings: undefined }`, and the registered hook callbacks find no
groups to dispatch.

**`src/config.ts`, merge semantics (global then project concatenation):**

```ts
function mergeHooks(globalHooks, projectHooks) {
  const merged = {};
  for (const key of HOOK_KEYS) {
    const groups = [...(globalHooks?.[key] ?? []), ...(projectHooks?.[key] ?? [])];
    if (groups.length > 0) { merged[key] = groups; hasAnyHook = true; }
  }
  return hasAnyHook ? merged : undefined;
}
```

`HOOK_KEYS` accepts both Claude-style PascalCase (`SessionStart`, `PreToolUse`,
`Stop`) and Pi-style snake_case (`session_start`, `pre_tool_use`, `stop`) and
concatenates both into the same dispatch.

**`src/hook-context.ts`, in-memory-only extension state (no persisted files):**

```ts
export type HookModuleContext = {
  // ...
  firedSessionStartKeys: Set<string>;
  pendingUserPromptContext?: string;
  stopHookActive: boolean;
  // ...
};
```

The shared context holds a `Set<string>` for session-start dedupe, a transient
prompt-context string, and a `stopHookActive` flag. No field is persisted to
disk by the extension; nothing is written under `~/.pi/` or the project by this
package.

**`README.md`, configuration source and scope:**

> Configure hooks in `~/.pi/agent/settings.json` or `.pi/settings.json`.

**`README.md`, merge and cwd behavior:**

> Hook commands run in the current session `cwd`. Global config and project
> config are merged by concatenating event arrays.

**`README.md`, `if` and matcher reach into MCP tool names:**

> PreToolUse / PostToolUse / PostToolUseFailure: Matches `tool_name`. … names
> are usually lowercase, for example: `bash`, `read`, `write`, `edit`, `grep`,
> `find`, `ls`, `mcp__.*`.

The `mcp__.*` matcher means tool-event hooks observe and may deny/rewrite
tools whose Pi names begin with `mcp__`, i.e. tools surfaced by an MCP adapter
extension. This is the behavioral coupling point with `pi-mcp-adapter` in a
compound profile.

## Key passages and anchors

- **`package.json` identity:** version `0.0.2`; MIT; repository
  `git+https://github.com/hsingjui/pi-hooks.git`; `pi.extensions`
  `["./src/pi-hooks.ts"]`; peer-depends on `@earendil-works/pi-coding-agent` (`*`).
- **`src/config.ts` config model:** reads global `~/.pi/agent/settings.json`
  and `<cwd>/.pi/settings.json`; no writes; returns undefined settings when no
  `hooks` key exists in either scope.
- **`src/config.ts` merge:** per-event array concatenation, global first then
  project; accepts PascalCase and snake_case event keys.
- **`src/hook-context.ts` state:** in-memory `Set` for dedupe, transient prompt
  context, `stopHookActive` flag — nothing persisted.
- **`README.md` scope:** hooks configured in `~/.pi/agent/settings.json` or
  `.pi/settings.json`; commands run in session cwd; arrays concatenated.
- **`README.md` tool-name matcher:** includes `mcp__.*`, coupling hook dispatch
  to MCP-tool calls.

## Structural metadata

- Publisher: `hsingjui` (npm scope); GitHub user `hsingjui`
- Document type: installed npm package artifact (TypeScript extension)
- Surface: Pi extension registered via `package.json` `pi.extensions`
- Retrieval depth: full package directory read; key source files quoted
