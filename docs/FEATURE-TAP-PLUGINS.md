# Feature: Tap-Defined Plugins

## Summary

Allow tap authors to define plugins inline in `tap.json` — a `plugins` array where each entry declares skills, MCP servers, and agent definitions with component files living in the tap repo itself. When `skilltap install <name>` resolves to a tap-defined plugin, it builds a `PluginManifest` from the inline definition and hands off to the existing `installPlugin()` flow. No separate plugin repo or `plugin.json` needed.

This extends the tap format to be a complete plugin distribution channel, complementing the existing plugin detection from cloned repos with `.claude-plugin/plugin.json`.

## Requirements

### R1: `tap.json` supports a `plugins` array

The `TapSchema` accepts an optional `plugins` field alongside `skills`:

```json
{
  "name": "my-tap",
  "skills": [
    { "name": "commit-helper", "description": "...", "repo": "user/commit-helper" }
  ],
  "plugins": [
    {
      "name": "dev-toolkit",
      "description": "Development productivity tools",
      "version": "1.0.0",
      "skills": [
        { "name": "code-review", "path": "plugins/dev-toolkit/skills/code-review" },
        { "name": "test-gen", "path": "plugins/dev-toolkit/skills/test-gen" }
      ],
      "mcpServers": {
        "database": { "command": "npx", "args": ["-y", "@corp/db-mcp"] }
      },
      "agents": [
        { "name": "reviewer", "path": "plugins/dev-toolkit/agents/reviewer.md" }
      ],
      "tags": ["development", "productivity"]
    }
  ]
}
```

**Acceptance criteria:**
- `TapSchema.safeParse()` accepts tap.json with and without `plugins` field
- Existing tap.json files without `plugins` still parse (backward compatible)
- Plugin entries require `name`; `description`, `version`, `skills`, `mcpServers`, `agents`, `tags` are optional

### R2: Tap plugin entries resolve during `skilltap install <name>`

When `skilltap install dev-toolkit` does tap name resolution, it searches both `skills` and `plugins` arrays across all configured taps. If the name matches a plugin entry, the install flow:

1. Uses the tap's cloned directory as the content root
2. Builds a `PluginManifest` from the inline definition (mapping paths relative to the tap repo root)
3. Calls the existing `installPlugin(contentDir, manifest, options)` — no new install logic needed

**Acceptance criteria:**
- `skilltap install <name>` finds tap-defined plugins by name
- Plugin skills are placed in `.agents/skills/` with symlinks (existing flow)
- MCP servers are injected into agent configs (existing flow)
- Agent definitions are placed in `.claude/agents/` (existing flow)
- Plugin is recorded in `plugins.json` with `repo` pointing to the tap URL

### R3: Tap plugin entries appear in `skilltap find`

Tap-defined plugins appear in search results with the `[plugin]` badge (already implemented for `plugin: true` tap entries in Phase 25). The tap loader needs to synthesize plugin entries into the `TapEntry[]` results with `plugin: true`.

**Acceptance criteria:**
- `skilltap find <query>` returns matching plugins from tap `plugins` arrays
- Results show `[plugin]` badge
- `skilltap tap install` interactive picker includes tap plugins

### R4: Tap plugin content validated at tap-add time

When `skilltap tap add <name> <url>` clones a tap with plugins, validate that the referenced skill/agent paths exist in the tap directory. Warn (don't error) if paths are missing — the tap author may add them later.

**Acceptance criteria:**
- `skilltap tap add` warns about missing plugin content paths
- Warning is non-fatal (tap is still added)

### R5: Tap plugins work with existing plugin management commands

Installed tap plugins are managed identically to repo-detected plugins:
- `skilltap plugin` lists them
- `skilltap plugin info <name>` shows details
- `skilltap plugin toggle <name>` toggles components
- `skilltap plugin remove <name>` removes everything

**Acceptance criteria:**
- No changes needed to plugin management commands (they operate on `plugins.json` which the install flow already populates)

## Scope

**In scope:**
- `TapSchema` extension with `plugins` array
- Tap-defined plugin schema (`TapPluginSchema`) for inline component definitions
- Tap name resolution extended to search `plugins` alongside `skills`
- Conversion from `TapPlugin` → `PluginManifest` for handoff to `installPlugin`
- Tap `find` and `tap install` integration
- Validation at tap-add time

**Out of scope:**
- Tap-defined plugins with content in a _separate_ repo (that's the existing `plugin: true` + `repo` field on `TapSkillSchema`)
- Plugin update from taps (tap pull + re-detect would be a future feature)
- Plugin-specific tap commands (e.g., `skilltap tap plugins`)
- Inline skill content in tap.json (skills still need SKILL.md files on disk)

## Technical Context

- **Existing code touched:**
  - `packages/core/src/schemas/tap.ts` — add `TapPluginSchema` and `plugins` field to `TapSchema`
  - `packages/core/src/taps.ts` — `loadTaps()`/`searchTaps()` extended to include plugin entries
  - `packages/core/src/install.ts` — tap resolution extended to check `plugins` array; when a plugin is found, build `PluginManifest` and call `installPlugin` using tap dir as content root
  - `packages/core/src/plugin/install.ts` — already handles `installPlugin(contentDir, manifest, options)` — no changes needed
  - `packages/core/src/plugin/mcp.ts` — `parseMcpObject()` already handles inline MCP configs

- **Dependencies:** All plugin infrastructure from Phases 20-25 (detection, storage, MCP injection, lifecycle). The tap system (`taps.ts`, `loadTaps`, `searchTaps`). The install flow (`install.ts`).

- **Constraints:**
  - Backward compatible: existing tap.json without `plugins` must keep working
  - Tap plugins use the tap's cloned directory as the content source — no extra clone step
  - The `PluginManifest.pluginRoot` for tap plugins points to the tap directory, not a standalone plugin repo

## Open Questions

- Should tap-defined plugin names be namespaced by tap (e.g., `my-tap/dev-toolkit`) to avoid cross-tap name collisions? Or is the simple name sufficient since plugins.json tracks the source tap?
- When a tap is updated (`skilltap tap update`), should existing installed tap plugins be checked for changes? (Likely deferred to a future update feature.)
- Should `TapPlugin.mcpServers` accept both inline objects and file path references (like `.mcp.json` within the tap repo)?
