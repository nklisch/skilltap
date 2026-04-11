import { mkdir } from "node:fs/promises";
import { homedir } from "node:os";
import { join } from "node:path";
import { parse, stringify } from "smol-toml";
import { type Config, ConfigSchema } from "./schemas/config";
import { type InstalledJson, InstalledJsonSchema } from "./schemas/installed";
import { parseWithResult } from "./schemas/index";
import { loadJsonState, saveJsonState } from "./json-state";
import { err, ok, type Result, UserError } from "./types";

/**
 * Migrate v1 flat security config to v2 per-mode structure.
 * Called during loadConfig before Zod validation.
 *
 * v1 shape: security.scan, security.on_warn, security.require_scan (flat)
 * v2 shape: security.human.*, security.agent.* (per-mode)
 *
 * Idempotent — v2 config passes through unchanged.
 */
export function migrateSecurityConfig(raw: Record<string, unknown>): Record<string, unknown> {
  const security = raw.security;
  if (!security || typeof security !== "object" || Array.isArray(security)) {
    return raw;
  }

  const sec = security as Record<string, unknown>;

  // Detect v1 by presence of flat `scan` string at top level of security
  if (typeof sec.scan !== "string") {
    // Already v2 or no security config — pass through
    return raw;
  }

  // Extract v1 values
  const v1Scan = sec.scan as string;
  const v1OnWarn = typeof sec.on_warn === "string" ? sec.on_warn : "prompt";
  const v1RequireScan = typeof sec.require_scan === "boolean" ? sec.require_scan : false;
  const v1AgentCli = typeof sec.agent === "string" ? sec.agent : "";

  // Build v2 security object
  const v2Security: Record<string, unknown> = {
    // Keep shared fields
    agent_cli: v1AgentCli,
    threshold: sec.threshold,
    max_size: sec.max_size,
    ollama_model: sec.ollama_model,
    overrides: sec.overrides ?? [],
    // Per-mode settings
    human: {
      scan: v1Scan,
      on_warn: v1OnWarn,
      require_scan: v1RequireScan,
    },
    agent: {
      // Preserve current strict agent-mode behavior for existing users
      scan: v1Scan === "off" ? "static" : v1Scan,
      on_warn: "fail",
      require_scan: true,
    },
  };

  // Remove undefined shared fields (let Zod apply defaults)
  if (v2Security.threshold === undefined) delete v2Security.threshold;
  if (v2Security.max_size === undefined) delete v2Security.max_size;
  if (v2Security.ollama_model === undefined) delete v2Security.ollama_model;

  return { ...raw, security: v2Security };
}

export function getConfigDir(): string {
  const xdg = process.env.XDG_CONFIG_HOME;
  return xdg ? join(xdg, "skilltap") : join(homedir(), ".config", "skilltap");
}

export async function ensureDirs(): Promise<Result<void>> {
  const dir = getConfigDir();
  try {
    await mkdir(join(dir, "taps"), { recursive: true });
    await mkdir(join(dir, "cache"), { recursive: true });
    return ok(undefined);
  } catch (e) {
    return err(new UserError(`Failed to create config directories: ${e}`));
  }
}

// Static template preserves comments for user reference.
// smol-toml.stringify() strips comments, so saveConfig() will lose them — acceptable.
// NOTE: valid-values comments below are documentation only. Keep in sync with:
//   - AGENT_PATHS / AGENT_LABELS in core/src/symlink.ts (for agent IDs)
//   - SCAN_MODES / ON_WARN_MODES / etc. in core/src/schemas/config.ts (for enum values)
const DEFAULT_CONFIG_TEMPLATE = `# Default settings for install commands
[defaults]
# Agent-specific directories to also symlink to on every install
# Valid values: "claude-code", "cursor", "codex", "gemini", "windsurf"
also = []

# Auto-accept prompts (same as --yes). Auto-selects all skills and
# auto-accepts clean installs. Security warnings still require confirmation.
# Scope still prompts unless a default scope is also set.
yes = false

# Default install scope. If set, skips the scope prompt.
# Values: "global", "project", or "" (prompt)
scope = ""

# Security scanning settings (use 'skilltap config security' to configure)
[security]
# Agent CLI to use for semantic scanning.
# Values: see KNOWN_AGENT_NAMES in core/src/agents/detect.ts (claude, gemini, codex, opencode, ollama)
# or an absolute path to a custom binary (e.g. "/usr/local/bin/my-llm").
# Empty string = prompt on first use, then save selection.
agent_cli = ""

# Risk threshold for semantic scan (0-10, chunks scoring >= this are flagged)
threshold = 5

# Max total skill directory size in bytes before warning (default 50KB)
max_size = 51200

# Ollama model for semantic scanning (if using ollama adapter)
ollama_model = ""

# Security settings when you run skilltap interactively
[security.human]
scan = "static"
on_warn = "prompt"
require_scan = false

# Security settings when an AI agent runs skilltap
[security.agent]
scan = "static"
on_warn = "fail"
require_scan = true

# Agent mode — for when skilltap is invoked by an AI agent, not a human.
["agent-mode"]
# Enable agent mode. When true, all prompts auto-accept or hard-fail.
enabled = false

# Default scope for agent installs. Required when agent mode is enabled.
# Values: "global", "project"
scope = "project"

# Registry search settings
[registry]
# Which skill registries to search when running 'skilltap find <query>'.
# Built-in registry: "skills.sh" (https://skills.sh). Set to [] to disable all.
enabled = ["skills.sh"]

# Custom registries implementing the skills.sh search API:
#   GET {url}/api/search?q={query}&limit={n}
#   Response: { "skills": [{ "id", "name", "description", "source", "installs" }] }
# Add to enabled[] above to activate.
# [[registry.sources]]
# name = "my-org"
# url = "https://skills.example.com"

# Built-in tap: the official skilltap-skills collection.
# Set to false to opt out of the built-in tap entirely.
builtin_tap = true

# Show step details during install (fetched, scan clean). Set false to silence.
# verbose = true

# Additional tap definitions (repeatable section)
# [[taps]]
# name = "home"
# url = "https://gitea.example.com/nathan/my-skills-tap"
`;

export async function loadConfig(): Promise<Result<Config>> {
  const dir = getConfigDir();
  const file = join(dir, "config.toml");

  const dirsResult = await ensureDirs();
  if (!dirsResult.ok) return dirsResult;

  const f = Bun.file(file);
  const exists = await f.exists();

  if (!exists) {
    try {
      await Bun.write(file, DEFAULT_CONFIG_TEMPLATE);
    } catch (e) {
      return err(new UserError(`Failed to write default config: ${e}`));
    }
    return ok(ConfigSchema.parse({}));
  }

  let text: string;
  try {
    text = await f.text();
  } catch (e) {
    return err(new UserError(`Failed to read config.toml: ${e}`));
  }

  let raw: unknown;
  try {
    raw = parse(text);
  } catch (e) {
    return err(new UserError(`Invalid TOML in config.toml: ${e}`));
  }

  // Migrate v1 flat security config to v2 per-mode structure before validation
  const migrated = migrateSecurityConfig(raw as Record<string, unknown>);

  return parseWithResult(ConfigSchema, migrated, "config.toml");
}

export async function saveConfig(config: Config): Promise<Result<void>> {
  const dir = getConfigDir();
  const file = join(dir, "config.toml");

  const dirsResult = await ensureDirs();
  if (!dirsResult.ok) return dirsResult;

  try {
    // biome-ignore lint/suspicious/noExplicitAny: smol-toml stringify types don't accept Config directly
    const text = stringify(config as any);
    await Bun.write(file, text);
    return ok(undefined);
  } catch (e) {
    return err(new UserError(`Failed to save config: ${e}`));
  }
}

const _DEFAULT_INSTALLED: InstalledJson = { version: 1, skills: [] };

function getInstalledPath(projectRoot?: string): string {
  return projectRoot
    ? join(projectRoot, ".agents", "installed.json")
    : join(getConfigDir(), "installed.json");
}

export async function loadInstalled(projectRoot?: string): Promise<Result<InstalledJson>> {
  return loadJsonState(
    getInstalledPath(projectRoot),
    InstalledJsonSchema,
    "installed.json",
    { version: 1 as const, skills: [] },
  );
}

export async function saveInstalled(
  installed: InstalledJson,
  projectRoot?: string,
): Promise<Result<void>> {
  return saveJsonState(getInstalledPath(projectRoot), installed, "installed.json", projectRoot, ensureDirs);
}
