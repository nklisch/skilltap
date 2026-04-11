# Design: Tap-Defined Plugins

## Overview

Extend `tap.json` with a `plugins` array so tap authors can define plugins inline — skills, MCP servers, and agent definitions with content files living in the tap repo. When `skilltap install tap-name/plugin-name` resolves to a tap-defined plugin, the install flow builds a `PluginManifest` from the inline definition and hands off to the existing `installPlugin()`.

## Key Design Decisions

### New `"skilltap"` format

Tap-defined plugins use `format: "skilltap"` (distinct from `"claude-code"` and `"codex"`). This distinguishes the source format in `plugins.json` and `PluginManifest`.

### Tap-namespaced install: `tap-name/plugin-name`

When `skilltap install foo/bar` is invoked, the tap resolution step now checks: is `foo` a configured tap name? If yes, search that tap's `plugins` array for `bar`. If found, install as a tap plugin. If `foo` is not a configured tap, fall through to GitHub shorthand as before.

### Simple name in storage, tap field for provenance

The `PluginRecord.name` stays as the simple plugin name (`dev-toolkit`). The existing `tap` field records the source tap name. This is consistent with how `InstalledSkill` handles tap provenance.

### MCP: inline objects or file path reference

`TapPlugin.mcpServers` accepts either:
- An inline object: `{ "db": { "command": "npx", "args": [...] } }`
- A string path to a `.mcp.json` file in the tap repo: `"plugins/dev-toolkit/.mcp.json"`

The conversion function checks the type and calls `parseMcpObject` or `parseMcpJson` accordingly.

### Content comes from the tap repo's cloned directory

Skill paths and agent paths in the plugin definition are relative to the tap repo root. The tap is already cloned locally at `~/.config/skilltap/taps/{name}/`. The install flow uses that directory as `contentDir`.

---

## Implementation Units

### Unit 1: Extend TapSchema with plugins array

**File**: `packages/core/src/schemas/tap.ts`

```typescript
import { z } from "zod/v4";

// ... existing TapTrustSchema, TapSkillSchema ...

export const TapPluginSkillSchema = z.object({
  name: z.string(),
  path: z.string(),
  description: z.string().default(""),
});

export const TapPluginAgentSchema = z.object({
  name: z.string(),
  path: z.string(),
});

export const TapPluginSchema = z.object({
  name: z.string(),
  description: z.string().default(""),
  version: z.string().optional(),
  skills: z.array(TapPluginSkillSchema).default([]),
  mcpServers: z.union([
    z.string(),                                    // path to .mcp.json in tap repo
    z.record(z.string(), z.unknown()),             // inline MCP server definitions
  ]).optional(),
  agents: z.array(TapPluginAgentSchema).default([]),
  tags: z.array(z.string()).default([]),
});

export const TapSchema = z.object({
  name: z.string(),
  description: z.string().optional(),
  skills: z.array(TapSkillSchema),
  plugins: z.array(TapPluginSchema).default([]),  // NEW
});

// --- Inferred types ---
export type TapPluginSkill = z.infer<typeof TapPluginSkillSchema>;
export type TapPluginAgent = z.infer<typeof TapPluginAgentSchema>;
export type TapPlugin = z.infer<typeof TapPluginSchema>;
// ... existing Tap, TapSkill, TapTrust types unchanged
```

**Implementation Notes**:
- `plugins` field defaults to `[]` so existing tap.json without it still parses.
- `mcpServers` is optional. When a string, it's a path to `.mcp.json` in the tap repo. When an object, it's inline MCP definitions (same format as `.mcp.json` flat format).
- Skill and agent entries have `name` + `path` (relative to tap repo root). Skills also have optional `description`.

**Acceptance Criteria**:
- [ ] Existing tap.json without `plugins` field still parses (backward compatible)
- [ ] `TapSchema.safeParse()` accepts tap.json with inline plugins
- [ ] `TapPlugin` requires `name`; all other fields optional or have defaults
- [ ] `mcpServers` accepts both string and object

---

### Unit 2: Add `"skilltap"` to PLUGIN_FORMATS

**File**: `packages/core/src/schemas/plugin.ts`

```typescript
export const PLUGIN_FORMATS = ["claude-code", "codex", "skilltap"] as const;
```

**Implementation Notes**:
- Single line change. `PluginManifestSchema` and `PluginRecordSchema` both derive from `PLUGIN_FORMATS`, so they automatically accept `"skilltap"`.

**Acceptance Criteria**:
- [ ] `PluginManifestSchema` accepts `format: "skilltap"`
- [ ] `PluginRecordSchema` accepts `format: "skilltap"`

---

### Unit 3: Convert TapPlugin to PluginManifest

**File**: `packages/core/src/taps.ts` — add a conversion function

```typescript
import type { PluginManifest } from "./schemas/plugin";
import type { TapPlugin } from "./schemas/tap";
import { parseMcpJson, parseMcpObject } from "./plugin/mcp";

/**
 * Convert a tap-defined plugin into a PluginManifest.
 * @param plugin - The TapPlugin entry from tap.json
 * @param tapDir - Absolute path to the tap's cloned directory
 */
export async function tapPluginToManifest(
  plugin: TapPlugin,
  tapDir: string,
): Promise<Result<PluginManifest, UserError>>;
```

**Implementation Notes**:

Build a `PluginManifest` with `format: "skilltap"`, `pluginRoot: tapDir`:

**Skills**: Map each `TapPluginSkill` to a `PluginSkillComponent`:
```typescript
{ type: "skill", name: skill.name, path: skill.path, description: skill.description }
```
Skill paths in the TapPlugin are already relative to the tap root — pass them through directly.

**MCP**: If `plugin.mcpServers` is a string → call `parseMcpJson(join(tapDir, plugin.mcpServers))`. If it's an object → call `parseMcpObject(plugin.mcpServers as Record<string, unknown>)`. Wrap each result in `{ type: "mcp", server }`.

**Agents**: Map each `TapPluginAgent` to a `PluginAgentComponent`. Read the `.md` file at `join(tapDir, agent.path)` to extract frontmatter via `parseSkillFrontmatter`. If no frontmatter, use empty dict.
```typescript
{ type: "agent", name: agent.name, path: agent.path, frontmatter }
```

**Acceptance Criteria**:
- [ ] Converts skills with correct paths
- [ ] Handles inline MCP objects
- [ ] Handles file path MCP references
- [ ] Returns `ok([])` for missing MCP file (non-fatal)
- [ ] Reads agent frontmatter from .md files
- [ ] Returns error for malformed MCP
- [ ] Sets format to `"skilltap"`

---

### Unit 4: Extend loadTaps to include plugin entries

**File**: `packages/core/src/taps.ts` — modify `loadTaps()`

Currently `loadTaps()` returns `TapEntry[]` where each entry has `{ tapName, skill: TapSkill }`. Tap plugins need to be included in results so they appear in `find` and `tap install`.

Add a new entry type or extend `TapEntry`:

```typescript
export type TapEntry = {
  tapName: string;
  skill: TapSkill;
  /** If this entry represents a tap-defined plugin (not a skill) */
  tapPlugin?: TapPlugin;
};
```

In `loadTaps()`, after loading `tapResult.value.skills`, also iterate `tapResult.value.plugins`:
```typescript
for (const plugin of tapResult.value.plugins) {
  entries.push({
    tapName: tap.name,
    skill: {
      name: plugin.name,
      description: plugin.description,
      repo: tap.url,   // tap plugins don't have their own repo — use tap URL
      tags: plugin.tags,
      plugin: true,
    },
    tapPlugin: plugin,
  });
}
```

This makes plugins appear in search results automatically (the `searchTaps` function already works on `TapEntry[]`). The `plugin: true` flag on the synthesized `TapSkill` ensures the `[plugin]` badge shows in `find`.

**Implementation Notes**:
- The synthesized `TapSkill.repo` is set to the tap URL. This won't be used for cloning (tap plugins use the local tap dir), but it gives the `find` output a meaningful source reference.
- The `tapPlugin` field is optional — existing skill entries don't have it. Only tap-defined plugin entries set it.
- `searchTaps` already works on `TapEntry[]` and searches `skill.name`, `skill.description`, `skill.tags` — no changes needed.
- Also update `getTapInfo` to include plugin count alongside skill count.

**Acceptance Criteria**:
- [ ] `loadTaps()` includes plugin entries from `tap.json` `plugins` array
- [ ] Plugin entries have `plugin: true` on the synthesized TapSkill
- [ ] Plugin entries have `tapPlugin` populated
- [ ] `searchTaps` finds plugins by name, description, and tags
- [ ] Skills-only taps (no `plugins` field) still work unchanged

---

### Unit 5: Extend tap name resolution in install.ts

**File**: `packages/core/src/install.ts` — modify `resolveTapName()`

Currently `resolveTapName` checks `looksLikeTapName(source)` — which rejects anything containing `/`. For `tap-name/plugin-name`, add a new resolution step BEFORE `looksLikeTapName`:

```typescript
// Check for tap-name/plugin-name pattern
function parseTapPluginRef(source: string): { tapName: string; pluginName: string } | null {
  if (!source.includes("/")) return null;
  const parts = source.split("/");
  if (parts.length !== 2) return null;
  // Don't match URLs, local paths, or protocol prefixes
  if (/^(https?:\/\/|git@|ssh:\/\/|github:|npm:)/.test(source)) return null;
  if (source.startsWith("./") || source.startsWith("/") || source.startsWith("~/")) return null;
  return { tapName: parts[0]!, pluginName: parts[1]! };
}
```

In `installSkill`, after tap resolution (step 1.5) and before source resolution (step 2), add:

```typescript
// 1.6. Tap plugin resolution (tap-name/plugin-name)
const tapPluginRef = parseTapPluginRef(source);
if (tapPluginRef) {
  const tapsResult = await loadTaps();
  if (tapsResult.ok) {
    const match = tapsResult.value.find(
      (e) => e.tapName === tapPluginRef.tapName && e.tapPlugin?.name === tapPluginRef.pluginName,
    );
    if (match?.tapPlugin) {
      // Found a tap-defined plugin — build manifest and install
      const tapDirPath = join(getConfigDir(), "taps", tapPluginRef.tapName);
      const manifestResult = await tapPluginToManifest(match.tapPlugin, tapDirPath);
      if (!manifestResult.ok) return manifestResult;

      // Ask user if they want to install as plugin
      if (options.onPluginDetected) {
        const decision = await options.onPluginDetected(manifestResult.value);
        if (decision === "cancel") return err(new UserError("Install cancelled."));
        if (decision === "skills-only") {
          // Fall through — treat as a regular tap skill lookup
          // (the name resolution below will handle it)
        } else {
          // decision === "plugin"
          const result = await installPlugin(tapDirPath, manifestResult.value, {
            scope: options.scope,
            projectRoot: options.projectRoot,
            also,
            skipScan: options.skipScan,
            onWarnings: options.onPluginWarnings,
            onConfirm: options.onPluginConfirm,
            repo: match.skill.repo,
            ref: null,
            sha: null,
            tap: tapPluginRef.tapName,
          });
          if (!result.ok) return result;
          return ok({
            records: [],
            warnings: result.value.warnings,
            semanticWarnings: [],
            updates: [],
            pluginRecord: result.value.record,
          });
        }
      } else {
        // No callback — auto-install as plugin (agent mode behavior)
        const result = await installPlugin(tapDirPath, manifestResult.value, {
          scope: options.scope,
          projectRoot: options.projectRoot,
          also,
          skipScan: options.skipScan,
          repo: match.skill.repo,
          ref: null,
          sha: null,
          tap: tapPluginRef.tapName,
        });
        if (!result.ok) return result;
        return ok({
          records: [],
          warnings: result.value.warnings,
          semanticWarnings: [],
          updates: [],
          pluginRecord: result.value.record,
        });
      }
    }
    // No matching tap plugin — fall through to normal resolution
    // (might be a GitHub shorthand like owner/repo)
  }
}
```

**Implementation Notes**:
- The `parseTapPluginRef` check runs before `resolveTapName` (which handles bare names like `commit-helper`). It intercepts `foo/bar` patterns where `foo` matches a configured tap.
- When `onPluginDetected` is absent, auto-install as plugin (consistent with agent mode where callbacks are absent → auto-proceed).
- Import `tapPluginToManifest` from `./taps` and `installPlugin` from `./plugin/install`.
- The tap directory is at `join(getConfigDir(), "taps", tapName)` — same as `tapDir()` in taps.ts, but that function is private. Either export it or inline the path computation.

**Acceptance Criteria**:
- [ ] `skilltap install my-tap/dev-toolkit` resolves to tap plugin when `my-tap` is a configured tap with `dev-toolkit` in its `plugins` array
- [ ] Falls through to GitHub shorthand when tap name doesn't match
- [ ] Falls through to GitHub shorthand when plugin name doesn't match
- [ ] Plugin installed via existing `installPlugin` flow
- [ ] `pluginRecord` included in `InstallResult`
- [ ] Existing `skilltap install owner/repo` (GitHub shorthand) still works
- [ ] Existing `skilltap install bare-name` (tap skill resolution) still works

---

### Unit 6: Test Fixtures

**Directory**: `packages/test-utils/fixtures/tap-with-plugins/`

```
tap-with-plugins/
  tap.json
  plugins/
    dev-toolkit/
      skills/
        code-review/
          SKILL.md
      agents/
        reviewer.md
      .mcp.json
```

`tap.json`:
```json
{
  "name": "test-tap-plugins",
  "description": "Test tap with inline plugins",
  "skills": [
    { "name": "standalone-skill", "description": "A regular skill", "repo": "owner/standalone" }
  ],
  "plugins": [
    {
      "name": "dev-toolkit",
      "description": "Dev tools plugin",
      "version": "1.0.0",
      "skills": [
        { "name": "code-review", "path": "plugins/dev-toolkit/skills/code-review" }
      ],
      "mcpServers": {
        "test-db": { "command": "npx", "args": ["-y", "test-mcp"] }
      },
      "agents": [
        { "name": "reviewer", "path": "plugins/dev-toolkit/agents/reviewer.md" }
      ],
      "tags": ["dev", "tools"]
    }
  ]
}
```

Add fixture factory to `packages/test-utils/src/fixtures.ts`:
```typescript
export const createTapWithPlugins = () => createFixtureRepo("tap-with-plugins");
```

**Acceptance Criteria**:
- [ ] Fixture creates a valid git repo
- [ ] `TapSchema.safeParse()` accepts the tap.json
- [ ] Skill, agent, and MCP content files exist at referenced paths

---

### Unit 7: Barrel Exports

**File**: `packages/core/src/schemas/tap.ts` — new types already exported via existing barrel

**File**: `packages/core/src/taps.ts` — export `tapPluginToManifest`:
```typescript
export { tapPluginToManifest };
```
(Already in the module; just ensure it's exported.)

---

## Implementation Order

1. **Unit 2**: Add `"skilltap"` to `PLUGIN_FORMATS` (one-line, enables everything else)
2. **Unit 1**: Extend `TapSchema` with `plugins` array + `TapPluginSchema`
3. **Unit 6**: Test fixtures (needed by integration tests)
4. **Unit 3**: `tapPluginToManifest` conversion function
5. **Unit 4**: Extend `loadTaps` to include plugin entries
6. **Unit 5**: Extend install flow with tap plugin resolution
7. **Unit 7**: Barrel exports

---

## Testing

### Schema Tests: `packages/core/src/schemas/tap.test.ts`

```
describe("TapSchema with plugins")
  - accepts tap.json with plugins array
  - accepts tap.json without plugins (backward compat)
  - defaults plugins to [] when omitted
  - accepts plugin with all fields
  - accepts plugin with only name (minimal)
  - accepts mcpServers as inline object
  - accepts mcpServers as string path

describe("TapPluginSchema")
  - requires name
  - defaults description to ""
  - defaults skills to []
  - defaults agents to []
  - defaults tags to []
```

### Conversion Tests: `packages/core/src/taps.test.ts` (extend existing)

```
describe("tapPluginToManifest")
  - converts skills with correct paths
  - converts inline MCP servers
  - converts file path MCP references
  - reads agent frontmatter from .md files
  - returns empty components when no skills/mcp/agents
  - sets format to "skilltap"
  - sets pluginRoot to tapDir
```

### Integration Tests: `packages/core/src/plugin/tap-plugin.test.ts`

```
describe("tap plugin install flow")
  - install tap-name/plugin-name resolves and installs plugin
  - skills placed in .agents/skills/
  - MCP injected into agent configs
  - agents placed in .claude/agents/
  - recorded in plugins.json with tap field
  - falls through to GitHub shorthand for non-tap names
  - loadTaps includes plugin entries with plugin:true badge
  - searchTaps finds plugins by name and tags
```

---

## Verification Checklist

```bash
bun test packages/core/src/schemas/tap.test.ts
bun test packages/core/src/taps.test.ts
bun test packages/core/src/plugin/tap-plugin.test.ts
bun test  # full suite
```
