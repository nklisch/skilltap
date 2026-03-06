import { mkdir } from "node:fs/promises";
import { homedir } from "node:os";
import { join } from "node:path";
import { parse, stringify } from "smol-toml";
import { z } from "zod/v4";
import { type Config, ConfigSchema } from "./schemas/config";
import { type InstalledJson, InstalledJsonSchema } from "./schemas/installed";
import { err, ok, type Result, UserError } from "./types";

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

# Security scanning settings
[security]
# Scan mode: "static" (Layer 1 only), "semantic" (Layer 1 + Layer 2), "off"
scan = "static"

# What to do when security warnings are found:
#   "prompt" = show warnings and ask user (default)
#   "fail"   = abort immediately, no prompt (same as --strict)
on_warn = "prompt"

# Prevent --skip-scan from being used. When true, security scanning
# cannot be bypassed via CLI flags. Useful for org/machine-level policy.
require_scan = false

# Agent CLI to use for semantic scanning.
# Values: see KNOWN_AGENT_NAMES in core/src/agents/detect.ts (claude, gemini, codex, opencode, ollama)
# or an absolute path to a custom binary (e.g. "/usr/local/bin/my-llm").
# Empty string = prompt on first use, then save selection.
agent = ""

# Risk threshold for semantic scan (0-10, chunks scoring >= this are flagged)
threshold = 5

# Max total skill directory size in bytes before warning (default 50KB)
max_size = 51200

# Ollama model for semantic scanning (if using ollama adapter)
ollama_model = ""

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

  const result = ConfigSchema.safeParse(raw);
  if (!result.success) {
    return err(
      new UserError(`Invalid config.toml: ${z.prettifyError(result.error)}`),
    );
  }

  return ok(result.data);
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
  const file = getInstalledPath(projectRoot);

  const f = Bun.file(file);
  const exists = await f.exists();

  if (!exists) {
    return ok({ version: 1 as const, skills: [] });
  }

  let raw: unknown;
  try {
    raw = await f.json();
  } catch (e) {
    return err(new UserError(`Invalid JSON in installed.json: ${e}`));
  }

  const result = InstalledJsonSchema.safeParse(raw);
  if (!result.success) {
    return err(
      new UserError(`Invalid installed.json: ${z.prettifyError(result.error)}`),
    );
  }

  return ok(result.data);
}

export async function saveInstalled(
  installed: InstalledJson,
  projectRoot?: string,
): Promise<Result<void>> {
  const file = getInstalledPath(projectRoot);

  if (projectRoot) {
    try {
      await mkdir(join(projectRoot, ".agents"), { recursive: true });
    } catch (e) {
      return err(new UserError(`Failed to create .agents directory: ${e}`));
    }
  } else {
    const dirsResult = await ensureDirs();
    if (!dirsResult.ok) return dirsResult;
  }

  try {
    await Bun.write(file, JSON.stringify(installed, null, 2));
    return ok(undefined);
  } catch (e) {
    return err(new UserError(`Failed to save installed.json: ${e}`));
  }
}
