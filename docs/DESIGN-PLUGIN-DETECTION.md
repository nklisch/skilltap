# Design: Phase 20 — Plugin Detection and Parsing

## Overview

This phase adds the ability to detect and parse Claude Code and Codex plugin formats, extracting the portable subset (skills, MCP servers, agent definitions) into a normalized internal representation. No install/storage logic — this phase is purely about reading plugin repos and producing a `PluginManifest`.

Phase 20 creates the `core/src/plugin/` module group with 6 source files, 1 schema file, 1 barrel, and 4 test files.

## Key Design Decisions

### Convention-based discovery over manifest-declared paths

Real-world Claude Code plugins rarely declare component paths in `plugin.json`. The manifest is typically just `{ name, description, author }`. Components are discovered from convention directories: `skills/*/SKILL.md`, `.mcp.json`, `agents/*.md`. The parser uses convention-first discovery with optional manifest path overrides.

### Two `.mcp.json` formats

Real `.mcp.json` files come in two shapes:
- **Flat**: `{ "server-name": { "command": "...", "args": [...] } }`
- **Wrapped**: `{ "mcpServers": { "server-name": { "command": "...", "args": [...] } } }`

The MCP parser handles both by checking for a `mcpServers` key first.

### MCP server types

MCP configs contain two server types:
- **stdio**: `{ command, args?, env? }` — most common
- **HTTP**: `{ type: "http", url }` — less common but valid

Both are normalized into the same `McpServerEntry` schema with a discriminated `type` field.

### Frontmatter passthrough for agents

Agent `.md` files have platform-specific frontmatter (`name`, `description`, `model`, `tools`, `color`, etc.). skilltap reads and preserves the raw frontmatter dict without validating platform-specific fields — only `name` is extracted for identification. The body content is preserved verbatim.

### Reuse existing `parseSkillFrontmatter`

The existing `frontmatter.ts` parser handles the YAML-subset frontmatter used by both SKILL.md and agent .md files. The agent parser reuses it directly.

### Reuse existing `scanner.scan()` for skill discovery within plugins

Rather than reimplementing SKILL.md discovery, the plugin parsers call the existing `scanner.scan()` on the plugin directory to find skills. This keeps skill discovery logic in one place.

---

## Implementation Units

### Unit 1: Plugin Schemas

**File**: `packages/core/src/schemas/plugin.ts`

```typescript
import { z } from "zod/v4";

// --- Component types within a plugin manifest ---

export const PLUGIN_FORMATS = ["claude-code", "codex"] as const;
export const PLUGIN_COMPONENT_TYPES = ["skill", "mcp", "agent"] as const;

export const PluginSkillComponentSchema = z.object({
  type: z.literal("skill"),
  name: z.string(),
  path: z.string(),
  description: z.string().default(""),
});

export const McpStdioServerSchema = z.object({
  type: z.literal("stdio").default("stdio"),
  name: z.string(),
  command: z.string(),
  args: z.array(z.string()).default([]),
  env: z.record(z.string(), z.string()).default({}),
});

export const McpHttpServerSchema = z.object({
  type: z.literal("http"),
  name: z.string(),
  url: z.string(),
});

export const McpServerEntrySchema = z.union([
  McpStdioServerSchema,
  McpHttpServerSchema,
]);

export const PluginMcpComponentSchema = z.object({
  type: z.literal("mcp"),
  server: McpServerEntrySchema,
});

export const PluginAgentComponentSchema = z.object({
  type: z.literal("agent"),
  name: z.string(),
  path: z.string(),
  frontmatter: z.record(z.string(), z.unknown()).default({}),
});

export const PluginComponentSchema = z.discriminatedUnion("type", [
  PluginSkillComponentSchema,
  PluginMcpComponentSchema,
  PluginAgentComponentSchema,
]);

export const PluginManifestSchema = z.object({
  name: z.string(),
  version: z.string().optional(),
  description: z.string().default(""),
  format: z.enum(PLUGIN_FORMATS),
  pluginRoot: z.string(),
  components: z.array(PluginComponentSchema),
});

// --- Raw plugin.json schemas (for parsing the on-disk format) ---

export const ClaudePluginJsonSchema = z.object({
  name: z.string(),
  description: z.string().optional(),
  version: z.string().optional(),
  author: z.object({
    name: z.string(),
    email: z.string().optional(),
    url: z.string().optional(),
  }).optional(),
  homepage: z.string().optional(),
  repository: z.string().optional(),
  license: z.string().optional(),
  keywords: z.array(z.string()).optional(),
  // Component path overrides (convention paths used when absent)
  skills: z.union([z.string(), z.array(z.string())]).optional(),
  commands: z.union([z.string(), z.array(z.string())]).optional(),
  agents: z.union([z.string(), z.array(z.string())]).optional(),
  mcpServers: z.union([z.string(), z.array(z.string()), z.record(z.string(), z.unknown())]).optional(),
  // Ignored fields (platform-specific)
  hooks: z.unknown().optional(),
  lspServers: z.unknown().optional(),
  outputStyles: z.unknown().optional(),
  channels: z.unknown().optional(),
  userConfig: z.unknown().optional(),
}).passthrough();

export const CodexPluginJsonSchema = z.object({
  name: z.string(),
  version: z.string(),
  description: z.string(),
  author: z.object({
    name: z.string(),
    email: z.string().optional(),
    url: z.string().optional(),
  }).optional(),
  homepage: z.string().optional(),
  repository: z.string().optional(),
  license: z.string().optional(),
  keywords: z.array(z.string()).optional(),
  // Component pointers
  skills: z.string().optional(),
  mcpServers: z.string().optional(),
  apps: z.unknown().optional(),
  interface: z.unknown().optional(),
}).passthrough();

// --- Inferred types ---

export type PluginSkillComponent = z.infer<typeof PluginSkillComponentSchema>;
export type McpStdioServer = z.infer<typeof McpStdioServerSchema>;
export type McpHttpServer = z.infer<typeof McpHttpServerSchema>;
export type McpServerEntry = z.infer<typeof McpServerEntrySchema>;
export type PluginMcpComponent = z.infer<typeof PluginMcpComponentSchema>;
export type PluginAgentComponent = z.infer<typeof PluginAgentComponentSchema>;
export type PluginComponent = z.infer<typeof PluginComponentSchema>;
export type PluginManifest = z.infer<typeof PluginManifestSchema>;
export type ClaudePluginJson = z.infer<typeof ClaudePluginJsonSchema>;
export type CodexPluginJson = z.infer<typeof CodexPluginJsonSchema>;
```

**Implementation Notes**:
- `pluginRoot` is the absolute path to the plugin directory on disk (where plugin.json was found). Used by downstream phases for resolving relative component paths.
- `McpServerEntrySchema` is a union, not discriminatedUnion, because the `type` field on stdio has a default. The `type: "stdio"` is defaulted so flat entries (most common) work without specifying it.
- `ClaudePluginJsonSchema` uses `.passthrough()` to tolerate unknown fields (plugins evolve fast).
- The `PluginComponentSchema` uses `type` as discriminator. MCP components wrap a `McpServerEntry` in a `{ type: "mcp", server: ... }` envelope to keep the discriminated union working (MCP entries themselves use `type` for stdio vs http).

**Acceptance Criteria**:
- [ ] `PluginManifestSchema.safeParse()` accepts a valid manifest with all three component types
- [ ] `ClaudePluginJsonSchema.safeParse()` accepts minimal `{ name }` and full manifests
- [ ] `CodexPluginJsonSchema.safeParse()` requires `name`, `version`, `description`
- [ ] `McpServerEntrySchema.safeParse()` accepts both `{ command, args }` (defaults type to "stdio") and `{ type: "http", url }`
- [ ] Unknown fields in plugin.json are tolerated (passthrough)
- [ ] All types are exported and inferrable from schemas

---

### Unit 2: MCP Config Parser

**File**: `packages/core/src/plugin/mcp.ts`

```typescript
import type { Result } from "../types";
import type { McpServerEntry } from "../schemas/plugin";
import { UserError } from "../types";

/**
 * Parse a .mcp.json file and return normalized MCP server entries.
 *
 * Handles two on-disk formats:
 * - Flat: { "name": { command, args } }
 * - Wrapped: { "mcpServers": { "name": { command, args } } }
 *
 * Server entries can be stdio ({ command, args?, env? }) or HTTP ({ type: "http", url }).
 */
export async function parseMcpJson(
  filePath: string,
): Promise<Result<McpServerEntry[], UserError>>;

/**
 * Parse an inline mcpServers object from plugin.json.
 * Same format as the wrapped .mcp.json but passed directly.
 */
export function parseMcpObject(
  servers: Record<string, unknown>,
): Result<McpServerEntry[], UserError>;
```

**Implementation Notes**:
- Read the file with `Bun.file(filePath).text()`, parse with `JSON.parse`.
- Detection: if the parsed object has a `mcpServers` key whose value is a plain object, unwrap it (wrapped format). Otherwise treat the entire object as the flat format.
- For each key-value pair in the servers object:
  - If value has `type: "http"` and `url` → validate as `McpHttpServerSchema` with name injected.
  - Otherwise → validate as `McpStdioServerSchema` with name injected and `type` defaulted to `"stdio"`.
- Use `safeParse` per entry; collect errors as a single `UserError` listing all invalid entries, but don't fail-fast — parse all entries and report all issues.
- Return `ok([])` if the file doesn't exist or is empty (MCP is optional).

**Acceptance Criteria**:
- [ ] Parses flat format: `{ "db": { "command": "npx", "args": ["-y", "db-mcp"] } }` → `[{ type: "stdio", name: "db", command: "npx", args: ["-y", "db-mcp"], env: {} }]`
- [ ] Parses wrapped format: `{ "mcpServers": { "db": { ... } } }` → same result
- [ ] Parses HTTP servers: `{ "api": { "type": "http", "url": "https://..." } }` → `[{ type: "http", name: "api", url: "https://..." }]`
- [ ] Returns `ok([])` for non-existent file
- [ ] Returns `err` with descriptive message for malformed JSON
- [ ] Preserves `env` dict when present on stdio servers
- [ ] Handles mixed stdio + http servers in one file

---

### Unit 3: Agent Definition Parser

**File**: `packages/core/src/plugin/agents.ts`

```typescript
import type { Result } from "../types";
import type { PluginAgentComponent } from "../schemas/plugin";
import { UserError } from "../types";

/**
 * Discover and parse agent definition .md files from a directory.
 * Reads each .md file, extracts frontmatter (using parseSkillFrontmatter),
 * and returns agent components with name, path, and raw frontmatter.
 *
 * @param agentsDir - Absolute path to the agents directory (e.g., plugin/agents/)
 * @returns Array of agent components, or empty array if directory doesn't exist
 */
export async function parseAgentDefinitions(
  agentsDir: string,
): Promise<Result<PluginAgentComponent[], UserError>>;
```

**Implementation Notes**:
- Check if `agentsDir` exists with `Bun.file(join(agentsDir, "..")).exists()` — actually use `readdir` in a try/catch. If the directory doesn't exist, return `ok([])`.
- List all `.md` files via `readdir` + filter on `.md` extension (exclude non-.md files).
- For each `.md` file:
  - Read content with `Bun.file().text()`.
  - Parse frontmatter with `parseSkillFrontmatter(content)`.
  - Extract `name` from frontmatter. If no frontmatter or no `name` field, derive name from filename (strip `.md`).
  - Build `PluginAgentComponent`: `{ type: "agent", name, path: relative_path, frontmatter }`.
- The `path` field stores the path relative to the plugin root (e.g., `agents/code-review.md`).
- Sort results by name for deterministic output.

**Acceptance Criteria**:
- [ ] Discovers all `.md` files in `agentsDir`
- [ ] Extracts `name` from frontmatter when present
- [ ] Falls back to filename (minus `.md`) when frontmatter has no `name`
- [ ] Preserves full frontmatter dict (model, tools, color, etc.)
- [ ] Returns `ok([])` for non-existent directory
- [ ] Returns `ok([])` for empty directory
- [ ] Ignores non-.md files (README.txt, etc.)
- [ ] Path is relative to plugin root, not absolute

---

### Unit 4: Claude Code Plugin Parser

**File**: `packages/core/src/plugin/parse-claude.ts`

```typescript
import type { Result } from "../types";
import type { PluginManifest } from "../schemas/plugin";
import { UserError } from "../types";

/**
 * Parse a Claude Code plugin from a directory containing .claude-plugin/plugin.json.
 *
 * Component discovery order:
 * 1. Skills: manifest `skills` field → convention `skills/` directory → scanner fallback
 * 2. MCP: manifest `mcpServers` field (inline or path) → convention `.mcp.json`
 * 3. Agents: manifest `agents` field → convention `agents/` directory
 *
 * @param pluginDir - Absolute path to the plugin root (parent of .claude-plugin/)
 * @returns Parsed PluginManifest, or err if plugin.json is invalid
 */
export async function parseClaudePlugin(
  pluginDir: string,
): Promise<Result<PluginManifest, UserError>>;
```

**Implementation Notes**:
- Read `.claude-plugin/plugin.json`, parse JSON, validate with `ClaudePluginJsonSchema.safeParse()`.
- **Skills discovery**:
  - If manifest has `skills` field (string or string[]), resolve each path relative to `pluginDir`, then call `scanner.scan()` on each resolved directory.
  - If no `skills` field, call `scanner.scan(pluginDir)` to use the standard skill discovery algorithm (which already checks `skills/*/SKILL.md` and other convention paths).
  - Map each `ScannedSkill` to a `PluginSkillComponent`: `{ type: "skill", name: skill.name, path: relative(pluginDir, skill.path), description: skill.description }`.
- **MCP discovery**:
  - If manifest has `mcpServers` as a string → treat as a file path, call `parseMcpJson(resolve(pluginDir, path))`.
  - If manifest has `mcpServers` as an array of strings → parse each file, concat results.
  - If manifest has `mcpServers` as an object → call `parseMcpObject(mcpServers)` (inline config).
  - If no `mcpServers` field → try `parseMcpJson(join(pluginDir, ".mcp.json"))` (convention path, returns `[]` if missing).
  - Wrap each `McpServerEntry` in `{ type: "mcp", server: entry }`.
- **Agent discovery**:
  - If manifest has `agents` field (string or string[]), resolve each path, call `parseAgentDefinitions()` on each.
  - If no `agents` field → call `parseAgentDefinitions(join(pluginDir, "agents"))` (convention path, returns `[]` if missing).
- Assemble and return `PluginManifest`:
  - `name` from manifest.
  - `format: "claude-code"`.
  - `pluginRoot: pluginDir`.
  - `components`: concat all skill, mcp, and agent components.

**Acceptance Criteria**:
- [ ] Parses minimal plugin.json (`{ "name": "test" }`) and discovers components from conventions
- [ ] Uses manifest `skills` path override when present
- [ ] Uses manifest `mcpServers` path override when present
- [ ] Uses manifest `agents` path override when present
- [ ] Discovers skills from `skills/*/SKILL.md` by default (via scanner)
- [ ] Discovers MCP from `.mcp.json` by default
- [ ] Discovers agents from `agents/*.md` by default
- [ ] Handles inline `mcpServers` object in manifest
- [ ] Returns empty components arrays when no skills/mcps/agents found
- [ ] Returns `err` when `plugin.json` is malformed or missing `name`
- [ ] Component paths are relative to plugin root

---

### Unit 5: Codex Plugin Parser

**File**: `packages/core/src/plugin/parse-codex.ts`

```typescript
import type { Result } from "../types";
import type { PluginManifest } from "../schemas/plugin";
import { UserError } from "../types";

/**
 * Parse a Codex plugin from a directory containing .codex-plugin/plugin.json.
 *
 * Codex plugins have skills and MCP servers, but no agent definitions.
 *
 * @param pluginDir - Absolute path to the plugin root (parent of .codex-plugin/)
 * @returns Parsed PluginManifest, or err if plugin.json is invalid
 */
export async function parseCodexPlugin(
  pluginDir: string,
): Promise<Result<PluginManifest, UserError>>;
```

**Implementation Notes**:
- Read `.codex-plugin/plugin.json`, parse JSON, validate with `CodexPluginJsonSchema.safeParse()`.
- **Skills**: If `skills` field present, resolve path and scan. Otherwise scan `pluginDir` with `scanner.scan()`.
- **MCP**: If `mcpServers` field present, resolve path and parse. Otherwise try `.mcp.json` convention.
- **No agents**: Codex plugins don't have agent definitions.
- Same assembly pattern as Claude Code parser but with `format: "codex"`.

**Acceptance Criteria**:
- [ ] Parses full Codex plugin.json (requires `name`, `version`, `description`)
- [ ] Returns `err` for missing required fields
- [ ] Discovers skills and MCP from conventions when manifest fields absent
- [ ] Never produces agent components (even if an `agents/` directory exists)
- [ ] `format` is `"codex"` in the returned manifest

---

### Unit 6: Plugin Detector

**File**: `packages/core/src/plugin/detect.ts`

```typescript
import type { Result } from "../types";
import type { PluginManifest } from "../schemas/plugin";
import { UserError } from "../types";

/**
 * Detect and parse a plugin from a cloned directory.
 *
 * Priority: Claude Code (.claude-plugin/plugin.json) → Codex (.codex-plugin/plugin.json).
 * Returns null if no plugin manifest found (the repo is a plain skill repo).
 *
 * @param dir - Absolute path to the cloned repo root
 * @returns PluginManifest if a plugin was detected, null if not, or err on parse failure
 */
export async function detectPlugin(
  dir: string,
): Promise<Result<PluginManifest | null, UserError>>;
```

**Implementation Notes**:
- Check if `.claude-plugin/plugin.json` exists (via `Bun.file().exists()`).
  - If yes → call `parseClaudePlugin(dir)` and return the result.
- Check if `.codex-plugin/plugin.json` exists.
  - If yes → call `parseCodexPlugin(dir)` and return the result.
- If neither exists → return `ok(null)`.
- This is the single entry point that the install flow will call. It hides the detection logic from consumers.

**Acceptance Criteria**:
- [ ] Returns `PluginManifest` for a Claude Code plugin repo
- [ ] Returns `PluginManifest` for a Codex plugin repo
- [ ] Returns `null` for a plain skill repo (no plugin.json)
- [ ] Prefers Claude Code format when both exist (unlikely but defined)
- [ ] Propagates parse errors from the individual parsers

---

### Unit 7: Barrel Export

**File**: `packages/core/src/plugin/index.ts`

```typescript
export { detectPlugin } from "./detect";
export { parseClaudePlugin } from "./parse-claude";
export { parseCodexPlugin } from "./parse-codex";
export { parseMcpJson, parseMcpObject } from "./mcp";
export { parseAgentDefinitions } from "./agents";
```

**Add to core barrel** in `packages/core/src/index.ts`:

```typescript
export * from "./plugin";
```

**Add to schemas barrel** in `packages/core/src/schemas/index.ts`:

```typescript
export * from "./plugin";
```

**Acceptance Criteria**:
- [ ] All public functions are importable from `@skilltap/core`
- [ ] All schema types are importable from `@skilltap/core`

---

## Implementation Order

1. **Unit 1: Plugin Schemas** (`schemas/plugin.ts`) — foundation types needed by everything else
2. **Unit 2: MCP Config Parser** (`plugin/mcp.ts`) — standalone, no deps on other plugin modules
3. **Unit 3: Agent Definition Parser** (`plugin/agents.ts`) — standalone, depends only on existing `frontmatter.ts`
4. **Unit 4: Claude Code Plugin Parser** (`plugin/parse-claude.ts`) — depends on Units 1-3 + existing `scanner.ts`
5. **Unit 5: Codex Plugin Parser** (`plugin/parse-codex.ts`) — depends on Units 1-2 + existing `scanner.ts`
6. **Unit 6: Plugin Detector** (`plugin/detect.ts`) — depends on Units 4-5
7. **Unit 7: Barrel Export** (`plugin/index.ts`, schema/index.ts, core/index.ts) — wire up after all modules exist

Units 2 and 3 are independent and can be implemented in parallel.

---

## Testing

### Schema Tests: `packages/core/src/schemas/plugin.test.ts`

Follow existing schema test pattern: `VALID_*` constants, `safeParse`, spread-and-override for variants.

```
describe("PluginManifestSchema")
  - accepts valid manifest with all component types
  - accepts manifest with no components
  - rejects missing name

describe("ClaudePluginJsonSchema")
  - accepts minimal { name }
  - accepts full manifest with all fields
  - tolerates unknown fields (passthrough)
  - accepts skills as string
  - accepts skills as string[]

describe("CodexPluginJsonSchema")
  - accepts valid { name, version, description }
  - rejects missing version
  - rejects missing description

describe("McpServerEntrySchema")
  - accepts stdio server with command only
  - accepts stdio server with command, args, env
  - defaults type to "stdio" when omitted
  - accepts http server with type and url
  - rejects entry with neither command nor url

describe("PluginComponentSchema")
  - discriminates on type field
  - accepts skill component
  - accepts mcp component wrapping stdio server
  - accepts mcp component wrapping http server
  - accepts agent component
```

### MCP Parser Tests: `packages/core/src/plugin/mcp.test.ts`

Test `parseMcpJson` and `parseMcpObject` as pure functions with temp files.

```
describe("parseMcpJson")
  - parses flat format (server entries at top level)
  - parses wrapped format (under mcpServers key)
  - handles mixed stdio and http servers
  - returns ok([]) for non-existent file
  - returns err for invalid JSON
  - preserves env dict
  - returns ok([]) for empty object {}

describe("parseMcpObject")
  - parses server entries from object
  - handles empty object
  - returns err for invalid server entry
```

### Agent Parser Tests: `packages/core/src/plugin/agents.test.ts`

Uses temp directories with `.md` files.

```
describe("parseAgentDefinitions")
  - discovers all .md files in directory
  - extracts name from frontmatter
  - falls back to filename when no frontmatter name
  - preserves full frontmatter dict
  - returns ok([]) for non-existent directory
  - returns ok([]) for empty directory
  - ignores non-.md files
  - path is relative to plugin root
```

### Claude Parser Tests: `packages/core/src/plugin/parse-claude.test.ts`

Uses temp directories with full plugin structures.

```
describe("parseClaudePlugin")
  - parses minimal plugin (name only, no components)
  - discovers skills from skills/ convention
  - discovers mcp from .mcp.json convention
  - discovers agents from agents/ convention
  - uses skills path override from manifest
  - uses mcpServers path override from manifest
  - uses agents path override from manifest
  - handles inline mcpServers object in manifest
  - returns err for missing plugin.json
  - returns err for invalid plugin.json
  - component paths are relative to plugin root
```

### Codex Parser Tests: `packages/core/src/plugin/parse-codex.test.ts`

```
describe("parseCodexPlugin")
  - parses full codex plugin
  - discovers skills and MCP from conventions
  - never produces agent components
  - returns err for missing required fields
  - format is "codex"
```

### Detector Tests: `packages/core/src/plugin/detect.test.ts`

Uses temp directories with minimal plugin structures.

```
describe("detectPlugin")
  - detects Claude Code plugin
  - detects Codex plugin
  - returns null for plain skill repo
  - prefers Claude Code when both exist
  - propagates parse errors
```

### Test Fixtures

Create in `packages/test-utils/fixtures/`:

**`claude-plugin/`** — minimal Claude Code plugin:
```
claude-plugin/
  .claude-plugin/
    plugin.json         # { "name": "test-plugin", "description": "Test plugin" }
  skills/
    helper/
      SKILL.md          # Valid skill with name: helper, description: A helper skill
  agents/
    reviewer.md         # ---\nname: reviewer\ndescription: Reviews code\nmodel: sonnet\n---\nReview instructions
  .mcp.json             # { "test-db": { "command": "npx", "args": ["-y", "test-mcp"] } }
```

**`codex-plugin/`** — minimal Codex plugin:
```
codex-plugin/
  .codex-plugin/
    plugin.json         # { "name": "test-codex", "version": "1.0.0", "description": "Test Codex plugin" }
  skills/
    linter/
      SKILL.md          # Valid skill with name: linter, description: Lints code
  .mcp.json             # { "mcpServers": { "lint-server": { "command": "node", "args": ["server.js"] } } }
```

Add fixture factories to `packages/test-utils/src/fixtures.ts`:

```typescript
export const createClaudePluginRepo = () => createFixtureRepo("claude-plugin");
export const createCodexPluginRepo = () => createFixtureRepo("codex-plugin");
```

---

## Verification Checklist

```bash
# Run all plugin-related tests
bun test packages/core/src/schemas/plugin.test.ts
bun test packages/core/src/plugin/

# Verify exports
bun -e "import { detectPlugin, PluginManifestSchema } from './packages/core/src/index'; console.log(typeof detectPlugin, typeof PluginManifestSchema)"

# Verify no circular imports
bun build packages/core/src/index.ts --target=bun --outdir=/tmp/skilltap-check 2>&1 | grep -i circular
```
