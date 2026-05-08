# Design: Phase 43 — Claude Code Plugin Adoption

## Overview

Phase 43 makes skilltap aware of plugins that Claude Code's native `/plugin install`
already manages. The user runs `skilltap adopt`, picks Claude-Code-managed plugins from a
unified list, and skilltap adds them to its state — pointing at Claude Code's cache rather
than copying. After adoption: `skilltap status`, `skilltap toggle`, `skilltap doctor`, and
`skilltap sync` all see the plugin.

This phase also extends `adopt` with an external-path argument (`adopt <path>`) — the
replacement for Phase 42's deleted `link` workflow. Internal: skilltap symlinks the
external dir into the canonical agent dir.

The phase is built around a generic `AgentPluginScanner` framework so future agents
(Codex if/when it ships a marketplace, Gemini, Cursor extensions) can plug in without
touching the adoption code path.

## Discovered formats (read from this machine, 2026-05-08)

### `~/.claude/plugins/installed_plugins.json`

```json
{
  "version": 2,
  "plugins": {
    "<plugin-name>@<marketplace-name>": [
      {
        "scope": "user" | "local",
        "projectPath": "<absolute-path>",     // present when scope === "local"
        "installPath": "<absolute-path>",      // points to cache/marketplace/plugin/version/
        "version": "1.0.0",
        "installedAt": "2026-..." (ISO date),
        "lastUpdated": "2026-..." (ISO date),
        "gitCommitSha": "<40-char>"
      }
    ]
  }
}
```

A single plugin can have multiple installation entries (different scopes). Plugin keys
are `name@marketplace` — both parts needed for unique identification.

### `~/.claude/plugins/known_marketplaces.json`

```json
{
  "<marketplace-name>": {
    "source": {
      "source": "github",
      "repo": "owner/repo"
    },
    "installLocation": "<absolute-path>",
    "lastUpdated": "2026-..." (ISO),
    "autoUpdate"?: boolean
  }
}
```

The `source` field describes where the marketplace was cloned from. Phase 43 only handles
`source: "github"`; `local` and `url` source types fail tolerantly (warning, skip).

### Plugin cache layout

`<installPath>` (e.g., `~/.claude/plugins/cache/nklisch-skills/workflow/1.4.0/`) is a
standard skilltap plugin directory:

```
<installPath>/
├── .claude-plugin/
│   └── plugin.json     # standard manifest (name, description, version, author, repository, license)
├── skills/<name>/SKILL.md
├── agents/             # agent definitions (Claude Code only)
└── .mcp.json           # if applicable
```

**This is a structure skilltap's `parseClaudePlugin()` already reads.** Adoption reuses
that parser — no new manifest reader needed.

## Acceptance Criteria

- `skilltap adopt` (TTY, no args) opens a clack picker showing both unmanaged skills (existing) and Claude Code plugins (new).
- `skilltap adopt --source claude-code` filters the picker to Claude-Code-managed plugins only.
- `skilltap adopt <path>` (with a path) symlinks the external skill into the canonical agent dir and tracks it. Default mode: `track-in-place` (symlink).
- `skilltap adopt <path> --move` physically moves the dir into the canonical location.
- After adoption, `skilltap status` shows the plugin with a "managed by claude-code" indicator.
- `skilltap doctor` warns when a Claude Code plugin in `installed_plugins.json` has a name that overlaps with a skilltap standalone skill or plugin (cross-source canary).
- `bun test` passes.

## Out of scope (deferred follow-ups)

- Auto-symlinking adopted plugin skills into other agent dirs (cursor, codex, etc.). User opts in later via `--also`.
- `--also-uninstall` flag for `skilltap remove plugin <claude-adopted>` (would also remove from Claude Code via shell-out).
- Adopting Codex plugins (Codex has no marketplace; the stub returns no results).
- Adopting Cursor extensions, Gemini plugins, etc. (no concrete schemas yet).
- TUI dashboard integration (Phase 44).

## Architectural Options

### Option A — DiscoveredItem union
A common base type with `kind: "skill" | "plugin"`. `discoverAll()` returns a flat list. Pro: one code path. Con: forces awkward unification on different domain models — DiscoveredSkill has `locations[]` and `gitRemote`; DiscoveredAgentPlugin has `installPath` and `marketplaceSource`.

### Option B — Parallel channels (chosen)
`discoverSkills()` (existing, unchanged) returns `DiscoveredSkill[]`. New `scanAgentPlugins()` returns `DiscoveredAgentPlugin[]`. The CLI picker shows a unified list with type-tagged entries. Adoption dispatches to `adoptSkill()` (existing) or new `adoptAgentPlugin()` based on the picked item's kind.

### Option C — Plugin as skill-collection
Convert each Claude Code plugin into N DiscoveredSkill records (one per bundled skill). Reuse existing adopt path. Pro: zero new code. Con: loses plugin-as-unit; user can't adopt "the plugin" as a record; the state.plugins[] table is bypassed.

**Choice: Option B.** Plugin and skill are already first-class concepts in skilltap state. Adopting a plugin produces a `state.plugins[]` entry with `components[]`. The capture work in Phase 39 already deals with the skill-vs-plugin-component distinction. Option B keeps consistency.

## Trickiest Unit — Designed First

### Unit 1: `claude-code.ts` scanner

Reads two undocumented JSON files, walks per-plugin install paths via existing parser, returns adoption candidates. Risk: format drift breaks parsing; we use `.passthrough()` and graceful-degradation per file.

**File**: `packages/core/src/agent-plugins/claude-code.ts`

```typescript
import { join } from "node:path";
import { homedir } from "node:os";
import { z } from "zod/v4";
import { detectPlugin } from "../plugin/detect";
import type { PluginManifest } from "../schemas/plugin";
import { ok, err, type Result, UserError } from "../types";
import type {
  AgentPluginScanner,
  DiscoveredAgentPlugin,
} from "./types";

// --- Schemas (tolerant) ---

const InstalledPluginEntrySchema = z
  .object({
    scope: z.enum(["user", "local"]),
    projectPath: z.string().optional(),
    installPath: z.string(),
    version: z.string(),
    installedAt: z.string(),
    lastUpdated: z.string(),
    gitCommitSha: z.string().optional(),
  })
  .passthrough();

const InstalledPluginsFileSchema = z
  .object({
    version: z.literal(2),
    plugins: z.record(z.string(), z.array(InstalledPluginEntrySchema)),
  })
  .passthrough();

const MarketplaceSourceSchema = z
  .object({
    source: z.string(), // "github" | "local" | "url" — tolerant
    repo: z.string().optional(),
    url: z.string().optional(),
    path: z.string().optional(),
  })
  .passthrough();

const KnownMarketplaceEntrySchema = z
  .object({
    source: MarketplaceSourceSchema,
    installLocation: z.string(),
    lastUpdated: z.string(),
    autoUpdate: z.boolean().optional(),
  })
  .passthrough();

const KnownMarketplacesFileSchema = z.record(z.string(), KnownMarketplaceEntrySchema);

// --- Path helpers ---

function claudePluginsDir(): string {
  return join(homedir(), ".claude", "plugins");
}

function installedPluginsPath(): string {
  return join(claudePluginsDir(), "installed_plugins.json");
}

function knownMarketplacesPath(): string {
  return join(claudePluginsDir(), "known_marketplaces.json");
}

// --- Scanner ---

export function createClaudeCodeScanner(): AgentPluginScanner {
  return {
    name: "claude-code",
    async detect(): Promise<boolean> {
      // Cheap existence check — file present iff Claude Code's plugin system has run.
      return await Bun.file(installedPluginsPath()).exists();
    },
    async scan(): Promise<Result<DiscoveredAgentPlugin[], UserError>> {
      const installedRaw = await readJsonTolerant(
        installedPluginsPath(),
        InstalledPluginsFileSchema,
      );
      if (!installedRaw.ok) return installedRaw;
      const installed = installedRaw.value;

      const marketplacesRaw = await readJsonTolerant(
        knownMarketplacesPath(),
        KnownMarketplacesFileSchema,
      );
      // Marketplaces file is OPTIONAL — adoption still works without it,
      // just with less source-canonical metadata.
      const marketplaces = marketplacesRaw.ok ? marketplacesRaw.value : {};

      const results: DiscoveredAgentPlugin[] = [];
      for (const [key, entries] of Object.entries(installed.plugins)) {
        // key is "<name>@<marketplace>". Split.
        const at = key.lastIndexOf("@");
        if (at < 0) continue; // malformed; skip
        const name = key.slice(0, at);
        const marketplaceName = key.slice(at + 1);

        const marketplace = marketplaces[marketplaceName];
        const sourceUrl = marketplaceToSourceUrl(marketplace);

        for (const entry of entries) {
          // Walk the cached plugin's manifest.
          const manifestResult = await detectPlugin(entry.installPath);
          if (!manifestResult.ok || manifestResult.value === null) {
            // Cache is stale / format doesn't match; skip silently.
            continue;
          }
          const manifest = manifestResult.value;

          results.push({
            scannerName: "claude-code",
            name,
            marketplaceName,
            sourceUrl,
            installPath: entry.installPath,
            version: entry.version,
            sha: entry.gitCommitSha ?? null,
            // Map Claude Code's scope vocabulary to skilltap's:
            //   user  → global
            //   local → project (uses entry.projectPath)
            scope: entry.scope === "user" ? "global" : "project",
            projectRoot: entry.scope === "local" ? entry.projectPath : undefined,
            installedAt: entry.installedAt,
            updatedAt: entry.lastUpdated,
            manifest,
          });
        }
      }
      return ok(results);
    },
  };
}

function marketplaceToSourceUrl(
  m: z.infer<typeof KnownMarketplaceEntrySchema> | undefined,
): string | null {
  if (!m) return null;
  const s = m.source;
  if (s.source === "github" && s.repo) {
    return `github:${s.repo}`;
  }
  if (s.url) return s.url;
  if (s.path) return s.path;
  return null;
}

async function readJsonTolerant<T extends z.ZodTypeAny>(
  path: string,
  schema: T,
): Promise<Result<z.infer<T>, UserError>> {
  const file = Bun.file(path);
  if (!(await file.exists())) {
    return err(new UserError(`File not found: ${path}`));
  }
  let raw: unknown;
  try {
    raw = await file.json();
  } catch (e) {
    return err(new UserError(`Failed to parse JSON at ${path}: ${e}`));
  }
  const parsed = schema.safeParse(raw);
  if (!parsed.success) {
    return err(
      new UserError(
        `Schema mismatch at ${path}: ${parsed.error.issues
          .slice(0, 3)
          .map((i) => i.message)
          .join("; ")}`,
        "Claude Code's plugin file format may have changed. Run `skilltap doctor` for details.",
      ),
    );
  }
  return ok(parsed.data);
}
```

**Implementation Notes**:
- `detectPlugin()` already supports `.claude-plugin/plugin.json` parsing (via `parseClaudePlugin()`). We reuse it — no new manifest reader.
- Marketplaces file is treated as optional metadata — if it's missing or malformed, adoption still proceeds with `sourceUrl: null`. `gitCommitSha` is also optional (some entries lack it).
- `passthrough()` is used on every Zod schema so unknown fields don't fail parsing — matters for forward compat as Claude Code adds fields.
- Mapping `scope: "user"` → `scope: "global"` and `scope: "local"` → `scope: "project"` (with `projectRoot: entry.projectPath`).
- A plugin with multiple entries (e.g., installed both as `user` and `local`) produces multiple `DiscoveredAgentPlugin` records — one per entry. The picker may show duplicates with scope distinction.

**Acceptance Criteria**:
- [ ] `detect()` returns `true` when `installed_plugins.json` exists, `false` otherwise.
- [ ] `scan()` returns an empty array when `installed_plugins.json` is missing or empty.
- [ ] `scan()` parses real-world `installed_plugins.json` content (test uses fixture from this machine's actual file).
- [ ] `scan()` skips entries whose `installPath` doesn't contain a `.claude-plugin/plugin.json` (cache stale).
- [ ] `scan()` is tolerant of unknown fields in either file (adds via `.passthrough()`).
- [ ] When `known_marketplaces.json` is missing, scan still returns plugins with `sourceUrl: null`.
- [ ] When a marketplace entry has `source: "url"` or `source: "local"`, the URL field is preserved as `sourceUrl`.
- [ ] `scope: "local"` entries produce `DiscoveredAgentPlugin.scope === "project"` and carry `projectRoot`.

---

## Implementation Units

### Unit 2: `AgentPluginScanner` interface

**File**: `packages/core/src/agent-plugins/types.ts`

```typescript
import type { PluginManifest } from "../schemas/plugin";
import type { Result, UserError } from "../types";

export interface DiscoveredAgentPlugin {
  /** Scanner that produced this record (e.g., "claude-code"). */
  scannerName: string;
  /** Plugin name from the manifest. */
  name: string;
  /** Marketplace name (claude-code: from "<name>@<marketplace>" key). Optional for non-marketplace scanners. */
  marketplaceName?: string;
  /** Canonical source URL (e.g., "github:owner/repo"). May be null when unknown. */
  sourceUrl: string | null;
  /** Absolute path to the plugin's content (skilltap reads from here, doesn't copy). */
  installPath: string;
  /** Plugin version. */
  version: string;
  /** Git SHA at install time. May be null. */
  sha: string | null;
  /** Skilltap scope this should be adopted into. */
  scope: "global" | "project";
  /** Project root if scope === "project". */
  projectRoot?: string;
  installedAt: string;
  updatedAt: string;
  /** Parsed plugin manifest from installPath. */
  manifest: PluginManifest;
}

export interface AgentPluginScanner {
  /** Identifier for this scanner (e.g., "claude-code", "codex"). */
  name: string;
  /** Returns true if this agent's plugin system is detectable on the host. */
  detect(): Promise<boolean>;
  /** Returns the agent's installed plugins, or an empty array if none. */
  scan(): Promise<Result<DiscoveredAgentPlugin[], UserError>>;
}
```

**File**: `packages/core/src/agent-plugins/index.ts`

```typescript
export type { AgentPluginScanner, DiscoveredAgentPlugin } from "./types";
export { createClaudeCodeScanner } from "./claude-code";
export { createCodexScanner } from "./codex";
export { defaultScanners, scanAllAgentPlugins } from "./registry";
```

**File**: `packages/core/src/agent-plugins/registry.ts`

```typescript
import { createClaudeCodeScanner } from "./claude-code";
import { createCodexScanner } from "./codex";
import type { AgentPluginScanner, DiscoveredAgentPlugin } from "./types";
import { ok, type Result, type UserError } from "../types";

export function defaultScanners(): AgentPluginScanner[] {
  return [createClaudeCodeScanner(), createCodexScanner()];
}

export interface ScanAllResult {
  plugins: DiscoveredAgentPlugin[];
  /** Per-scanner errors — non-fatal; scan continues. */
  errors: { scanner: string; error: UserError }[];
}

export async function scanAllAgentPlugins(
  scanners: AgentPluginScanner[] = defaultScanners(),
): Promise<Result<ScanAllResult, UserError>> {
  const all: DiscoveredAgentPlugin[] = [];
  const errors: ScanAllResult["errors"] = [];
  for (const scanner of scanners) {
    if (!(await scanner.detect())) continue;
    const result = await scanner.scan();
    if (!result.ok) {
      errors.push({ scanner: scanner.name, error: result.error });
      continue;
    }
    all.push(...result.value);
  }
  return ok({ plugins: all, errors });
}
```

**Implementation Notes**:
- `defaultScanners()` returns an array consumers can extend or replace. Tests pass mocks here.
- `scanAllAgentPlugins()` is fail-soft: a broken scanner produces a warning, not a fatal error. The CLI surfaces the per-scanner errors via `out.warn()`.

**Acceptance Criteria**:
- [ ] `AgentPluginScanner` interface exported with two methods (`detect`, `scan`).
- [ ] `scanAllAgentPlugins([])` returns empty result (no scanners → no plugins).
- [ ] `scanAllAgentPlugins([failingScanner])` returns `ok` with empty plugins and one entry in `errors`.
- [ ] `scanAllAgentPlugins([scannerThatDetectsFalse])` skips that scanner without invoking `scan()`.

---

### Unit 3: Codex scanner stub

**File**: `packages/core/src/agent-plugins/codex.ts`

```typescript
import { ok, type Result, type UserError } from "../types";
import type { AgentPluginScanner, DiscoveredAgentPlugin } from "./types";

/**
 * Codex stub. OpenAI's Codex CLI does not ship a plugin marketplace today;
 * detect() returns false. When (if) Codex ships one, this file gets a real
 * implementation that mirrors claude-code.ts.
 */
export function createCodexScanner(): AgentPluginScanner {
  return {
    name: "codex",
    async detect(): Promise<boolean> {
      return false;
    },
    async scan(): Promise<Result<DiscoveredAgentPlugin[], UserError>> {
      return ok([]);
    },
  };
}
```

**Acceptance Criteria**:
- [ ] `detect()` always returns false.
- [ ] `scan()` always returns `ok([])`.

---

### Unit 4: Extend `adopt()` core function

The current `adoptSkill(skill, options)` takes a `DiscoveredSkill` and produces a state.skills entry. Phase 43 adds:

1. `adoptSkillFromPath(path, options)` — for external skill paths (replaces `link`).
2. `adoptAgentPlugin(plugin, options)` — for `DiscoveredAgentPlugin`. Produces a state.plugins entry pointing at `installPath`.
3. `discoverAllAdoptable(options)` — unified scanner that returns both unmanaged skills and agent plugins.

**File**: `packages/core/src/adopt.ts` (extend)

```typescript
import type { DiscoveredAgentPlugin } from "./agent-plugins/types";
import { scanAllAgentPlugins } from "./agent-plugins/registry";
import type { PluginRecord } from "./schemas/plugins";
import { manifestToRecord } from "./plugin/state";
// ... existing imports

// New types

export interface AdoptFromPathOptions {
  scope?: "global" | "project";
  projectRoot?: string;
  also?: string[];
  /** "track-in-place" (default; symlink) or "move" (relocate dir + symlink back). */
  mode?: AdoptMode;
  skipScan?: boolean;
  onWarnings?: (warnings: StaticWarning[], skillName: string) => Promise<boolean>;
}

export interface DiscoverAdoptableResult {
  skills: DiscoveredSkill[];   // unmanaged only
  plugins: DiscoveredAgentPlugin[];
  scannerErrors: { scanner: string; error: UserError }[];
}

// New API

/**
 * Adopt a skill from an arbitrary on-disk path. Replaces the deleted `link`
 * command. Default mode: track-in-place — symlinks the path into the
 * canonical agent dir. With mode: "move", relocates the dir.
 *
 * Validates that the path contains a SKILL.md before doing anything.
 */
export async function adoptSkillFromPath(
  path: string,
  options: AdoptFromPathOptions,
): Promise<Result<AdoptResult, UserError>>;

/**
 * Adopt a Claude-Code-managed plugin into skilltap state. Doesn't copy or
 * move files; just adds a state.plugins[] entry that points at the
 * installPath. The plugin remains owned by Claude Code; removing from
 * skilltap doesn't uninstall from Claude Code (out of scope).
 */
export async function adoptAgentPlugin(
  plugin: DiscoveredAgentPlugin,
  options: { also?: string[] },
): Promise<Result<{ record: PluginRecord }, UserError>>;

/**
 * Unified discovery: scan unmanaged skills (existing discoverSkills) +
 * scan all registered AgentPluginScanners. Returns a combined result the
 * CLI picker can show.
 */
export async function discoverAllAdoptable(
  options: DiscoverOptions,
): Promise<Result<DiscoverAdoptableResult, UserError>>;
```

**Implementation Notes**:
- `adoptSkillFromPath()` first validates: path exists, contains SKILL.md, frontmatter parses. Errors with a clear hint otherwise.
- For `mode: "track-in-place"`: create a symlink at `<agentDir>/<name>` → `<path>`. Add state record with `scope: "linked"` and `path: <original>`.
- For `mode: "move"`: move the dir to `<agentDir>/<name>`, then create a symlink at the original location pointing back. Add state record with `scope: scope` and `path: null`.
- `adoptAgentPlugin()` uses `manifestToRecord(manifest, { repo, ref, sha, scope, also, tap })` (existing helper) to build the PluginRecord. The `repo` field is set to the plugin's `sourceUrl` (or constructed as `"claude-code:<marketplaceName>:<name>"` when sourceUrl is null — this is the marker that distinguishes Claude-Code-adopted plugins from skilltap-installed ones).
- `adoptAgentPlugin()` does NOT inject MCP servers or copy skill content. Phase 43 keeps adoption read-only on Claude Code's cache. Future: extend with `also` list to inject into agent configs.
- The `repo` field's `claude-code:` prefix is the convention that lets `skilltap doctor`, `skilltap remove plugin`, and `skilltap status` recognize adopted plugins. Keep this prefix internal — never user-typed.

**Acceptance Criteria**:
- [ ] `adoptSkillFromPath("/path/with-no-skill-md", ...)` errors with hint to add SKILL.md.
- [ ] `adoptSkillFromPath(validPath, { mode: "track-in-place" })` creates a symlink, leaves the original dir untouched.
- [ ] `adoptSkillFromPath(validPath, { mode: "move" })` moves the dir, creates a back-symlink at original location.
- [ ] `adoptAgentPlugin(plugin, {})` adds a state.plugins[] entry with `repo` carrying a recognizable marker, `path: <installPath>`, components from the manifest.
- [ ] After `adoptAgentPlugin()`, the plugin appears in `loadState().plugins`.
- [ ] `discoverAllAdoptable({})` returns combined skills + plugins from all enabled scanners.

---

### Unit 5: CLI `adopt` command rewrite

The current `commands/adopt.ts` (Phase 42's lift from skills/adopt.ts) takes a single skill name and adopts a previously-unmanaged skill from the existing discoverSkills() scan. Phase 43 widens it to:

- `adopt` (no args, TTY) — opens picker showing unmanaged skills + Claude Code plugins.
- `adopt <path>` — adopt from arbitrary external path (track-in-place by default).
- `adopt <name>` — adopt a specific unmanaged skill by name (existing behavior preserved).
- `adopt --source claude-code` — picker filtered to Claude Code plugins only.

The "is `<arg>` a path or a name?" disambiguation: if `<arg>` starts with `./`, `/`, `~/`, or contains `/`, treat as path. Otherwise, treat as name (look up in unmanaged skills).

**File**: `packages/cli/src/commands/adopt.ts` (rewrite)

```typescript
import { defineCommand } from "citty";
import { isAbsolute } from "node:path";
import {
  adoptAgentPlugin,
  adoptSkill,
  adoptSkillFromPath,
  defaultScanners,
  discoverAllAdoptable,
} from "@skilltap/core";
import { isCancel, multiselect, select } from "@clack/prompts";
import { createOutput } from "../output";

export const adoptCommand = defineCommand({
  meta: {
    name: "adopt",
    description: "Bring an external skill or agent-managed plugin into skilltap",
  },
  args: {
    target: {
      type: "positional",
      required: false,
      description: "External path, or name of an unmanaged skill",
    },
    source: {
      type: "string",
      description: "Filter picker to one source (e.g., claude-code)",
    },
    project: { type: "boolean", default: false },
    global: { type: "boolean", default: false },
    also: { type: "string", description: "Comma-separated agent dirs to symlink into" },
    move: {
      type: "boolean",
      default: false,
      description: "When adopting a path: physically move dir (default: symlink in place)",
    },
    "skip-scan": { type: "boolean", default: false },
    yes: { type: "boolean", default: false, alias: "y" },
    json: { type: "boolean", default: false },
  },
  async run({ args }) {
    const out = createOutput({ json: args.json });

    if (!args.target) {
      // Picker mode (Phase 44 will replace with Ink TUI).
      if (process.stdout.isTTY !== true) {
        out.error(
          "adopt requires a target in non-interactive mode.",
          "Usage: skilltap adopt <path-or-name> | adopt --source claude-code",
        );
        process.exit(1);
      }
      return runAdoptPicker(out, args);
    }

    // Path or name?
    if (looksLikePath(args.target)) {
      return runAdoptPath(args.target, out, args);
    }
    return runAdoptName(args.target, out, args);
  },
});

function looksLikePath(s: string): boolean {
  return s.startsWith("./") || s.startsWith("/") || s.startsWith("~/") || isAbsolute(s) || s.includes("/");
}
```

`runAdoptPicker(out, args)`:
1. Call `discoverAllAdoptable({ ...scope })`.
2. If `args.source`: filter `result.plugins` to that scanner; skip skills section.
3. Build a clack select list with type-tagged labels:
   - `skill: <name> (<location>)`
   - `plugin: <name>@<marketplace> (managed by claude-code)`
4. On select, dispatch to `adoptSkill()` or `adoptAgentPlugin()`.
5. Emit `out.success(...)` with adopted name + scope + components count.

`runAdoptPath(path, out, args)`:
1. Validate path resolves and contains a SKILL.md.
2. Call `adoptSkillFromPath(path, { mode: args.move ? "move" : "track-in-place", scope, also, skipScan })`.
3. `out.success(\`Adopted skill from ${path} (${mode})\`)`.

`runAdoptName(name, out, args)`:
1. `discoverAllAdoptable({ ...scope })`.
2. Find name in skills (unmanaged) or plugins.
3. If skill: `adoptSkill(skill, { mode: args.move ? "move" : "track-in-place", ...})`.
4. If plugin: `adoptAgentPlugin(plugin, { also })`.
5. If neither: error.

**Acceptance Criteria**:
- [ ] `adopt` (no args, TTY) opens a picker.
- [ ] `adopt` (no args, non-TTY) errors with usage hint.
- [ ] `adopt /path/to/dir` adopts from external path (track-in-place default).
- [ ] `adopt /path/to/dir --move` moves the dir.
- [ ] `adopt my-skill` adopts an unmanaged skill by name (existing behavior).
- [ ] `adopt --source claude-code` opens picker filtered to Claude Code plugins.
- [ ] Adopting a Claude Code plugin adds a state.plugins[] entry with the canonical marker.
- [ ] Plugin selection from the picker (or by name) does NOT copy/move files; just records state.

---

### Unit 6: Doctor canary check for cross-source overlaps

When a Claude Code plugin's name matches a skilltap-managed standalone skill or plugin (without being adopted), warn. This is the early-warning system before silent substitution.

**File**: `packages/core/src/doctor/checks/claude-code-overlap.ts`

```typescript
import { defaultScanners, scanAllAgentPlugins } from "../../agent-plugins/registry";
import type { State } from "../../state/schema";
import type { DoctorCheck } from "../types";

/**
 * Defensive: a Claude Code plugin's name matches a skilltap-managed standalone
 * skill or plugin (where the skilltap record is NOT itself an adopted Claude
 * Code plugin). This signals a potential collision a user might want to
 * resolve via `skilltap adopt --source claude-code`.
 */
export async function checkClaudeCodeOverlap(state: State | null): Promise<DoctorCheck> {
  if (!state) return { name: "Claude Code overlaps", status: "pass" };

  const scanResult = await scanAllAgentPlugins();
  if (!scanResult.ok) {
    return {
      name: "Claude Code overlaps",
      status: "warn",
      issues: [
        {
          message: `Could not scan Claude Code plugins: ${scanResult.error.message}`,
          fixable: false,
        },
      ],
    };
  }
  const claudePlugins = scanResult.value.plugins.filter(
    (p) => p.scannerName === "claude-code",
  );
  if (claudePlugins.length === 0) {
    return { name: "Claude Code overlaps", status: "pass" };
  }

  const adoptedSourceMarker = "claude-code:";
  const issues: DoctorCheck["issues"] = [];

  for (const plugin of claudePlugins) {
    // Skill collision
    const skillCollision = state.skills.find((s) => s.name === plugin.name);
    if (skillCollision) {
      issues.push({
        message: `Claude Code plugin "${plugin.name}" overlaps with skilltap standalone skill "${skillCollision.name}".`,
        fixable: false,
        fixDescription: `Run \`skilltap adopt ${plugin.name}\` to bring the Claude Code plugin under skilltap, or \`skilltap remove skill ${skillCollision.name}\` if Claude Code's version should win.`,
      });
    }

    // Plugin collision (only flag if the existing record is NOT itself adopted)
    const pluginCollision = state.plugins.find(
      (p) => p.name === plugin.name && !p.repo?.startsWith(adoptedSourceMarker),
    );
    if (pluginCollision) {
      issues.push({
        message: `Claude Code plugin "${plugin.name}" overlaps with skilltap-installed plugin (different source).`,
        fixable: false,
        fixDescription: `Run \`skilltap remove plugin ${plugin.name}\` then \`skilltap adopt ${plugin.name}\` if you want Claude Code's version.`,
      });
    }
  }

  return {
    name: "Claude Code overlaps",
    status: issues.length > 0 ? "warn" : "pass",
    issues,
  };
}
```

Wire into `packages/core/src/doctor/index.ts` after the existing capture-collisions check:

```typescript
// 17. Claude Code plugin overlaps (Phase 43 canary).
await emit(await checkClaudeCodeOverlap(state));
```

**Acceptance Criteria**:
- [ ] Returns `pass` when no Claude Code plugins are present.
- [ ] Returns `pass` when Claude Code plugins are present but none collide with state.
- [ ] Returns `warn` with one issue per collision (skill name or non-adopted plugin name).
- [ ] Skips collisions where the existing skilltap record was itself adopted from Claude Code (recognized by the `claude-code:` marker in `repo`).

---

### Unit 7: Tests

#### Unit Tests

`packages/core/src/agent-plugins/claude-code.test.ts`:
- Real-world fixture parse (use a copy of this machine's `installed_plugins.json` as a fixture).
- Empty `installed_plugins.json` → empty result.
- Missing `installed_plugins.json` → `detect()` returns false; `scan()` errors gracefully.
- Missing `known_marketplaces.json` → scan returns plugins with `sourceUrl: null`.
- Malformed entry — passthrough preserves unknown fields without crashing.
- `scope: "user"` → `scope: "global"`; `scope: "local"` → `scope: "project"` with projectRoot.

`packages/core/src/agent-plugins/registry.test.ts`:
- Empty scanners → empty result.
- Failing scanner → recorded in `errors`, not fatal.
- `detect() === false` skips `scan()`.

`packages/core/src/agent-plugins/codex.test.ts`:
- Stub returns false / empty.

`packages/core/src/adopt.test.ts` (extend):
- `adoptSkillFromPath` track-in-place: symlink created, original dir intact, state record added.
- `adoptSkillFromPath` move: dir moved, back-symlink created, state record added.
- `adoptSkillFromPath` invalid path errors.
- `adoptAgentPlugin`: state record added with marker prefix; no files copied/moved.
- `discoverAllAdoptable`: combined results from multiple scanners.

`packages/core/src/doctor/checks/claude-code-overlap.test.ts`:
- No state → pass.
- No collisions → pass.
- Skill collision → warn with hint.
- Plugin collision (non-adopted) → warn.
- Plugin collision (adopted) → no warning.

#### CLI Tests

`packages/cli/src/commands/adopt.test.ts` (extend):
- `adopt /tmp/skill-fixture` adopts via path (subprocess).
- `adopt /tmp/skill-fixture --move` moves and creates back-symlink.
- `adopt nonexistent-name` errors.
- `adopt --source claude-code` (in TTY-mocked env) shows picker output.
- `adopt` (no args, non-TTY) errors with usage hint.

---

## Implementation Order

1. **Unit 2** — types + index + registry (no runtime, just scaffolding).
2. **Unit 3** — codex stub (trivial).
3. **Unit 1** — claude-code scanner (the trickiest unit).
4. **Unit 4** — extend `adopt()` core. Depends on Units 1+2.
5. **Unit 5** — CLI command rewrite. Depends on Unit 4.
6. **Unit 6** — doctor check. Depends on Units 1+2.
7. **Unit 7** — tests, woven through each unit's commit (don't pile up).

Parallelizable agents:
- Agent A: Units 2+3+1 (the agent-plugins module).
- Agent B: Unit 4 (adopt extension) → Unit 6 (doctor check) — sequential, both depend on A.
- Agent C: Unit 5 (CLI rewrite) — depends on A and B.

Sequential gate: A → B → C. With B and C overlapping where possible.

For Phase 43, two agents probably suffice:
- **Agent A**: Units 1+2+3+4+6 — full core surface.
- **Agent B**: Unit 5 + 7 — CLI + test rewrites.

## Pre-Mortem

**Riskiest assumption**: That Claude Code's `installed_plugins.json` and `known_marketplaces.json` formats stay stable enough for our tolerant parser. If Claude Code rewrites the schema (removes the `version: 2` literal, restructures the plugins map), our parser fails parsing and the user sees an error.

**Mitigation**:
- `.passthrough()` on every schema makes us tolerant of additions.
- The doctor check surfaces "Could not scan Claude Code plugins: <reason>" rather than failing the whole skilltap run.
- The `version: 2` literal is one of the few places we're not tolerant — if Claude Code bumps to v3 we'll need to update. We accept that follow-up.

**What would have to be true to fail in production**:
- Claude Code restructures `plugins` map (e.g., changes from `Record<string, Entry[]>` to `Record<string, Entry>`). Our parser fails; user sees a doctor warning.
- A plugin's `installPath` is removed from disk but the file entry remains. We skip silently (existing `detectPlugin()` returns null).
- Two plugins from different marketplaces share a name. The `<name>@<marketplace>` keying handles this; we surface them as distinct adoption candidates.

**Fallback if the riskiest unit doesn't work**:
- If parsing `installed_plugins.json` fails entirely, the doctor warns and `adopt --source claude-code` returns "no Claude Code plugins detected." Other adoption paths (external path, unmanaged skill) still work.

**Where I'm least sure**:
- Whether `adoptAgentPlugin()` should also create symlinks for the plugin's skills into other agent dirs (cursor, codex). Decision: **no for Phase 43**. Adoption is a record-only operation. User opts into multi-agent symlinking later via `--also` or a follow-up command. This keeps Phase 43 minimal and lets us validate the adoption shape before adding side effects.

## Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| Claude Code format drift | Medium | Tolerant Zod parsing + doctor surfacing |
| User has 100+ Claude Code plugins; picker is unwieldy | Low | Phase 44's TUI dashboard solves this; Phase 43's clack picker is a placeholder |
| Marketplace file missing → adoption proceeds with `sourceUrl: null` → doctor canary triggers a false "cross-source" alarm later | Low | The doctor check uses the `claude-code:` repo marker prefix, not sourceUrl, to identify adopted plugins; sourceUrl is informational only |
| `adopt <path>` defaults to track-in-place (symlink), but user expects move | Medium | Hint in the success message: "Adopted via symlink (use --move to relocate)." |
| Adopting + then removing a Claude Code plugin from skilltap leaves dangling symlinks | Low | `removeInstalledPlugin()` already cleans up component symlinks; Claude Code's own cache stays untouched |

## Verification Checklist

```bash
# 1. Build
bun run build

# 2. Source-side
test -d packages/core/src/agent-plugins/ || echo FAIL
test -f packages/core/src/agent-plugins/types.ts || echo FAIL
test -f packages/core/src/agent-plugins/claude-code.ts || echo FAIL
test -f packages/core/src/agent-plugins/codex.ts || echo FAIL
test -f packages/core/src/agent-plugins/registry.ts || echo FAIL
test -f packages/core/src/doctor/checks/claude-code-overlap.ts || echo FAIL

# 3. Tests
bun test packages/core/src/agent-plugins/
bun test packages/core/src/adopt.test.ts
bun test packages/core/src/doctor/checks/claude-code-overlap.test.ts
bun test packages/cli/src/commands/adopt.test.ts

# 4. Full suite
bun test

# 5. Smoke (manual, requires Claude Code installed):
bun run dev adopt --source claude-code
bun run dev adopt /tmp/some-skill-fixture --move
bun run dev doctor   # observe the new "Claude Code overlaps" check
```
