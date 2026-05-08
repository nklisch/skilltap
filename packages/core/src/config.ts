import { join } from "node:path";
import { parse, stringify } from "smol-toml";
import { ensureDirs, getConfigDir } from "./dirs";
import { loadJsonState } from "./json-state";
import { type Config, ConfigSchema } from "./schemas/config";
import { parseWithResult } from "./schemas/index";
import { type InstalledJson, InstalledJsonSchema } from "./schemas/installed";
import { loadState } from "./state/load";
import { saveState } from "./state/save";
import { err, ok, type Result, UserError } from "./types";

// Re-export from leaf module so existing importers keep working.
export { ensureDirs, getConfigDir };

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
# Scan mode for static analysis
# Values: "static", "semantic", "off"
scan = "static"

# What to do when security warnings are found
# Values: "prompt" (ask), "fail" (abort), "allow" (ignore)
on_warn = "prompt"

# Require security scan — blocks --skip-scan when true
require_scan = false

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

  return parseWithResult(ConfigSchema, raw as Record<string, unknown>, "config.toml");
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

// state.json is the canonical store. Reads still fall back to installed.json
// for unmigrated v0.x users (one-time; the next saveInstalled writes state.json
// and the fallback stops firing). Writes go ONLY to state.json.
export async function loadInstalled(
  projectRoot?: string,
): Promise<Result<InstalledJson>> {
  const stateResult = await loadState(projectRoot);
  if (stateResult.ok && stateResult.value.skills.length > 0) {
    return ok({ version: 1 as const, skills: stateResult.value.skills });
  }
  if (!stateResult.ok) return stateResult;
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
  const stateResult = await loadState(projectRoot);
  if (!stateResult.ok) return stateResult;
  const newState = {
    version: 2 as const,
    skills: installed.skills,
    plugins: stateResult.value.plugins,
    mcpServers: stateResult.value.mcpServers,
  };
  return saveState(newState, projectRoot);
}
