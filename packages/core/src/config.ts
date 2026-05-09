import { join } from "node:path";
import { parse, stringify } from "smol-toml";
import { ensureDirs, getConfigDir } from "./dirs";
import { type Config, ConfigSchema } from "./schemas/config";
import { parseWithResult } from "./schemas/index";
import type { InstalledJson } from "./schemas/installed";
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
const DEFAULT_CONFIG_TEMPLATE = `# skilltap configuration
# Run \`skilltap config edit\` to open this file in $EDITOR.

[defaults]
# also = ["claude-code"]    # extra agents to symlink installs into
# scope = "project"          # default install scope (smart-default if unset)

[security]
scan = "static"              # semantic | static | none
on_warn = "install"          # prompt | fail | install
trust = []                   # glob patterns (tap name or source URL)

[scanner]
# agent_cli = ""             # path/name of semantic-scan CLI (claude, codex, ollama, …)
# ollama_model = ""          # ollama model name when agent_cli = "ollama"
# threshold = 5              # 0-10, semantic risk score threshold
# max_size = 51200           # bytes, skill size limit

[updates]
auto_update = "off"          # off | patch | minor

[telemetry]
enabled = false
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

  // Hard-fail on legacy shapes — no silent fallback.
  const legacyDetection = detectLegacyConfig(raw);
  if (legacyDetection !== null) {
    return err(
      new UserError(
        `Legacy config detected (${legacyDetection}). Run \`skilltap migrate\` to upgrade to the v2.2 config schema.`,
        "skilltap migrate",
      ),
    );
  }

  return parseWithResult(ConfigSchema, raw as Record<string, unknown>, "config.toml");
}

// Returns the name of the first legacy marker found, or null if none.
function detectLegacyConfig(raw: unknown): string | null {
  if (!raw || typeof raw !== "object") return null;
  const r = raw as Record<string, unknown>;
  const sec = r.security as Record<string, unknown> | undefined;

  if (sec && typeof sec === "object" && !Array.isArray(sec)) {
    if ("human" in sec) return "[security.human]";
    if ("agent" in sec) return "[security.agent]";
    if ("overrides" in sec) return "[[security.overrides]]";
    if ("require_scan" in sec) return "security.require_scan";
    if ("agent_cli" in sec) return "security.agent_cli";
    if ("ollama_model" in sec) return "security.ollama_model";
    if ("threshold" in sec) return "security.threshold";
    if ("max_size" in sec) return "security.max_size";
  }
  if ("agent-mode" in r) return "[agent-mode]";
  if ("agent" in r) return "[agent]";

  return null;
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

// state.json is the only canonical store. v0.x installed.json fallback removed.
// Users on v0.x must run `skilltap migrate` to populate state.json.
export async function loadInstalled(
  projectRoot?: string,
): Promise<Result<InstalledJson>> {
  const stateResult = await loadState(projectRoot);
  if (!stateResult.ok) return stateResult;
  return ok({ version: 1 as const, skills: [...stateResult.value.skills] });
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
