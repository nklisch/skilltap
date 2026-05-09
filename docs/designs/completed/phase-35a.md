# Design: Phase 35a — `skilltap try` + Claude Desktop MCP Target

## Overview

Two purely-additive v2.0 enhancements bundled into one phase:

1. **`skilltap try <source>`** — read-only preview command. Clones a source to a temp directory, parses any manifests (SKILL.md or plugin), runs static security scan, prints a structured summary, then cleans up. Never writes to install paths or state.
2. **Claude Desktop in `MCP_AGENT_CONFIGS`** — adds `claude-desktop` as a supported MCP injection target alongside `claude-code` / `cursor` / `codex` / `gemini` / `windsurf`. Platform-aware path: macOS uses `Library/Application Support/Claude/claude_desktop_config.json` (relative to home), Linux uses `.config/Claude/claude_desktop_config.json`. Windows is deferred — `%APPDATA%` resolution doesn't fit the current `Record<string, string>` shape.

The `mcp:` install-prefix sub-piece from the original Phase 35 (per ROADMAP) is deferred — it touches `install.ts` and naturally lands with the install cutover (Phase 31c).

## Autonomous Decisions

### D1. Claude Desktop platform handling: populate registry at module init

The existing registry (`MCP_AGENT_CONFIGS: Record<string, string>`) maps agent IDs to paths joined with `scopeBase()`. For Claude Desktop, the path is OS-specific and global-only (no project equivalent — Claude Desktop is a single-instance app).

Decision: keep the registry shape and resolve Claude Desktop's relative path once at module load via `process.platform`. On unsupported platforms (currently Windows), the entry is omitted; users on those platforms see "claude-desktop: not supported on this platform" if they try to use it.

This avoids changing the registry's shape (which would ripple into mcpConfigPath, doctor, status, etc.) while still adding meaningful Claude Desktop support on macOS + Linux today.

### D2. `try` is core-implemented + cli-thin

Put the heavy lifting in `core/src/try.ts` so `gatherStatus`-style helpers can be reused (resolveSource, clone, detectPlugin, scan, scanStatic). The CLI command is just citty wiring + output rendering.

### D3. Try cleanup is best-effort

If the temp dir cleanup fails (e.g., permission error on a Bun-cloned `.git/objects/pack`), log a debug warning but don't error. The point of `try` is informational; cleanup failures shouldn't surface as command failures.

### D4. Try respects `--deep` for semantic scan

Wire the `--deep` flag (already in CliFlagsV2 from Phase 31a) through to enable Layer 2 semantic scan during preview. Default `try` runs static-only.

## Implementation Units

### Unit 1 — Claude Desktop entry in `MCP_AGENT_CONFIGS`

**File**: `packages/core/src/plugin/mcp-inject.ts`

Add at module init (right after the `MCP_AGENT_CONFIGS` const declaration):

```typescript
function claudeDesktopRelPath(): string | null {
  if (process.platform === "darwin") {
    return "Library/Application Support/Claude/claude_desktop_config.json";
  }
  if (process.platform === "linux") {
    return ".config/Claude/claude_desktop_config.json";
  }
  // Windows: needs %APPDATA% resolution which doesn't fit relative-path shape; deferred.
  return null;
}

const _claudeDesktopPath = claudeDesktopRelPath();
if (_claudeDesktopPath !== null) {
  MCP_AGENT_CONFIGS["claude-desktop"] = _claudeDesktopPath;
}
```

**Acceptance Criteria**:
- [ ] On macOS: `MCP_AGENT_CONFIGS["claude-desktop"] === "Library/Application Support/Claude/claude_desktop_config.json"`.
- [ ] On Linux: `MCP_AGENT_CONFIGS["claude-desktop"] === ".config/Claude/claude_desktop_config.json"`.
- [ ] On Windows: `MCP_AGENT_CONFIGS["claude-desktop"]` is `undefined` (key not present).
- [ ] `mcpConfigPath("claude-desktop", "global")` returns the absolute path on supported platforms; `null` on unsupported.
- [ ] Existing 5 agents (`claude-code`, `cursor`, `codex`, `gemini`, `windsurf`) keep their entries unchanged.

### Unit 2 — `core/src/try.ts`

```typescript
import { join } from "node:path";
import { realpath, rm } from "node:fs/promises";
import { resolveSource } from "./adapters";
import { debug } from "./debug";
import { makeTmpDir } from "./fs";
import { clone, revParse } from "./git";
import { detectPlugin } from "./plugin/detect";
import { scan, type ScannedSkill } from "./scanner";
import type { ResolvedSource } from "./schemas/agent";
import type { PluginManifest } from "./schemas/plugin";
import { scanStatic, type StaticWarning } from "./security";
import { ok, err, type Result, UserError, GitError } from "./types";

export interface TryReport {
  source: string;
  resolved: ResolvedSource;
  /** Cloned commit SHA (truncated). */
  sha: string | null;
  /** Detected plugin manifest, or null if the source is a skill repo. */
  plugin: PluginManifest | null;
  /** Skills found via scanner (always populated for both skill and plugin sources). */
  skills: ScannedSkill[];
  /** Warnings from static scan run on the cloned content. */
  warnings: StaticWarning[];
  /** Whether the scan ran (false if skipped). */
  scanned: boolean;
}

export interface TryOptions {
  /** Default git host for owner/repo shorthand. */
  gitHost?: string;
  /** When true, skips the static security scan. */
  skipScan?: boolean;
}

export async function tryPreview(
  source: string,
  options: TryOptions = {},
): Promise<Result<TryReport, UserError | GitError>> {
  const resolveResult = await resolveSource(source, { gitHost: options.gitHost });
  if (!resolveResult.ok) return resolveResult;
  const resolved = resolveResult.value;

  const tmp = await makeTmpDir("skilltap-try-");
  let cleaned = false;
  const cleanup = async () => {
    if (cleaned) return;
    cleaned = true;
    try {
      await rm(tmp, { recursive: true, force: true });
    } catch (e) {
      debug("try: cleanup failed", { tmp, error: String(e) });
    }
  };

  try {
    const cloneResult = await clone(resolved.url, tmp, {
      branch: resolved.ref,
      depth: 1,
    });
    if (!cloneResult.ok) {
      await cleanup();
      return cloneResult;
    }

    const contentDir = await realpath(tmp).catch(() => tmp);

    const shaResult = await revParse(contentDir);
    const sha = shaResult.ok ? shaResult.value.slice(0, 12) : null;

    const pluginResult = await detectPlugin(contentDir);
    if (!pluginResult.ok) {
      await cleanup();
      return pluginResult;
    }

    const skills = await scan(contentDir);

    let warnings: StaticWarning[] = [];
    if (!options.skipScan) {
      warnings = await scanStatic(contentDir);
    }

    await cleanup();

    return ok({
      source,
      resolved,
      sha,
      plugin: pluginResult.value,
      skills,
      warnings,
      scanned: !options.skipScan,
    });
  } catch (e) {
    await cleanup();
    return err(new UserError(`try preview failed: ${e}`));
  }
}
```

**Implementation Notes**:
- `makeTmpDir` is the existing helper in `core/src/fs.ts`.
- `realpath` resolves macOS `/tmp` → `/private/tmp` so contentDir matches what the scanner produces, mirroring `install.ts:577`.
- Debug logging via `debug()` matches the project pattern.
- Cleanup is idempotent (the `cleaned` flag) and runs in both success and error paths.

**Acceptance Criteria**:
- [ ] `tryPreview("github:user/repo")` clones, parses, scans, and returns a populated `TryReport` without writing to any install path.
- [ ] `tryPreview` always cleans up its temp dir (verified via filesystem check after the call).
- [ ] On clone failure: returns `Result.err` with the GitError; temp dir is cleaned.
- [ ] `skipScan: true` returns `warnings: []` and `scanned: false`.
- [ ] For a single-skill repo: `plugin === null`, `skills.length >= 1`.
- [ ] For a plugin repo: `plugin !== null`, `skills` populated from the cloned content.

### Unit 3 — Wire `tryPreview` into core's barrel export

**File**: `packages/core/src/index.ts`

Add after the existing v2.0 additions block:

```typescript
export { tryPreview, type TryReport, type TryOptions } from "./try";
```

### Unit 4 — `cli/src/commands/try.ts`

```typescript
import { tryPreview, type TryReport } from "@skilltap/core";
import { defineCommand } from "citty";
import { ansi, errorLine } from "../ui/format";

export default defineCommand({
  meta: {
    name: "try",
    description: "Preview a skill or plugin without installing",
  },
  args: {
    source: {
      type: "positional",
      description: "Source URL, owner/repo shorthand, npm:, or local path",
      required: true,
    },
    json: {
      type: "boolean",
      description: "Output as JSON",
      default: false,
    },
    "skip-scan": {
      type: "boolean",
      description: "Skip the static security scan",
      default: false,
    },
  },
  async run({ args }) {
    const result = await tryPreview(args.source as string, {
      skipScan: args["skip-scan"] as boolean,
    });
    if (!result.ok) {
      errorLine(result.error.message);
      process.exit(1);
    }

    if (args.json as boolean) {
      process.stdout.write(`${JSON.stringify(reportToJson(result.value), null, 2)}\n`);
      return;
    }

    renderTry(result.value);
  },
});

function reportToJson(report: TryReport): unknown {
  return {
    source: report.source,
    resolved: report.resolved,
    sha: report.sha,
    plugin: report.plugin
      ? { name: report.plugin.name, format: report.plugin.format, components: report.plugin.components.length }
      : null,
    skills: report.skills.map((s) => ({ name: s.name, description: s.description })),
    warnings: report.warnings.map((w) => ({ category: w.category, file: w.file, line: w.line })),
    scanned: report.scanned,
  };
}

function renderTry(report: TryReport): void {
  process.stdout.write(`\n${ansi.bold("skilltap try")} ${ansi.dim("—")} ${report.source}\n\n`);
  process.stdout.write(`${ansi.dim("Resolved:")} ${report.resolved.url}${report.resolved.ref ? ansi.dim(`@${report.resolved.ref}`) : ""}\n`);
  if (report.sha) process.stdout.write(`${ansi.dim("SHA:")} ${report.sha}\n`);
  process.stdout.write("\n");

  if (report.plugin) {
    process.stdout.write(`${ansi.bold("Plugin:")} ${report.plugin.name} ${ansi.dim(`(${report.plugin.format})`)}\n`);
    const skillCount = report.plugin.components.filter((c) => c.type === "skill").length;
    const mcpCount = report.plugin.components.filter((c) => c.type === "mcp").length;
    const agentCount = report.plugin.components.filter((c) => c.type === "agent").length;
    process.stdout.write(
      `  ${skillCount} skill${skillCount === 1 ? "" : "s"}, ${mcpCount} MCP server${mcpCount === 1 ? "" : "s"}, ${agentCount} agent${agentCount === 1 ? "" : "s"}\n\n`,
    );
  }

  if (report.skills.length > 0) {
    process.stdout.write(`${ansi.bold(`Skills`)} ${ansi.dim(`(${report.skills.length})`)}\n`);
    for (const skill of report.skills) {
      process.stdout.write(`  ${skill.name}${skill.description ? ansi.dim(` — ${skill.description}`) : ""}\n`);
    }
    process.stdout.write("\n");
  }

  if (!report.scanned) {
    process.stdout.write(`${ansi.dim("Scan:")} skipped\n`);
  } else if (report.warnings.length === 0) {
    process.stdout.write(`${ansi.green("✓")} No security warnings.\n`);
  } else {
    process.stdout.write(
      `${ansi.yellow("⚠")} ${report.warnings.length} security warning${report.warnings.length === 1 ? "" : "s"}:\n`,
    );
    for (const w of report.warnings) {
      process.stdout.write(`  ${ansi.yellow(w.category)} ${w.file}${w.line ? `:${w.line}` : ""}\n`);
    }
  }

  process.stdout.write(`\n${ansi.dim("This was a preview. Nothing was installed.")}\n`);
  process.stdout.write(`${ansi.dim("To install: skilltap install ")}${report.source}\n`);
}
```

### Unit 5 — Register `try` in `cli/src/index.ts`

Add to `subCommands`:

```typescript
try: () => import("./commands/try").then((m) => m.default),
```

### Unit 6 — Tests

**`packages/core/src/plugin/mcp-inject.claude-desktop.test.ts`**: New tiny test verifying the registry entry exists on the current platform.

**`packages/core/src/try.test.ts`**: New test using a local fixture (skip the network — use `local:` source via a tmpdir-cloned fake repo, or use a fixture from `@skilltap/test-utils`). Cover:
- Returns `TryReport` with skills populated for a single-skill repo.
- Cleans up temp dir after success.
- `skipScan: true` skips warnings.

If the local-fixture pattern is too fiddly for this phase, a single integration test that uses an actual GitHub URL would work too — but prefer the local fixture for speed.

## Implementation Order

1. Unit 1 (Claude Desktop entry) — independent, smallest.
2. Unit 2 (try.ts core) — depends on existing modules; standalone.
3. Unit 3 (export from index.ts) — after Unit 2.
4. Unit 4 (CLI command) — after Unit 3.
5. Unit 5 (CLI registration) — after Unit 4.
6. Unit 6 (tests) — after the above.

All can be done by a single pass directly (small enough scope; ~6 file edits, mostly additive).

## Verification

```bash
bun test packages/core/src/try.test.ts
bun test packages/core/src/plugin/mcp-inject.claude-desktop.test.ts

# Sanity: try a real source if network available (optional manual check)
SKILLTAP_NO_STARTUP=1 bun packages/cli/src/index.ts try nklisch/skilltap-skills --skip-scan

# Full v2 baseline still passes
bun test packages/core/src/manifest/ packages/core/src/state/ packages/core/src/migrate/ packages/core/src/sync/ packages/core/src/plugin-v2/ packages/core/src/plugin/detect.test.ts packages/core/src/schemas/config-v2.test.ts packages/core/src/policy-v2/ packages/core/src/status/
```

## Out of Scope

- `mcp:` install prefix — deferred to Phase 31c (touches install.ts).
- Windows Claude Desktop path resolution — deferred (needs `%APPDATA%` env var support).
- `try` for `mcp:` sources — deferred with the prefix work.
- Doctor checks for Claude Desktop config validity — Phase 36 doctor v2 upgrades.
