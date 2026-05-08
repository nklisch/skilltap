import { join } from "node:path";
import { parse } from "smol-toml";
import { getConfigDir } from "../config";

export interface V1StateMarkers {
  scope: "global" | "project";
  /** Path of installed.json if it exists, else null. */
  installedJson: string | null;
  /** Path of plugins.json if it exists, else null. */
  pluginsJson: string | null;
  /** Path of config.toml if it exists (global only), else null. */
  configToml: string | null;
  /** True iff config.toml contains v1.0-only keys. */
  configHasV1Keys: boolean;
}

const _V1_CONFIG_KEYS_RE =
  /^\s*\[(security\.human|security\.agent|agent-mode|\[security\.overrides\])\]/m;

async function fileExists(path: string): Promise<boolean> {
  return await Bun.file(path).exists();
}

async function configIsV1(path: string): Promise<boolean> {
  const text = await Bun.file(path)
    .text()
    .catch(() => "");
  if (!text) return false;
  // Quick string check first (cheap)
  if (
    text.includes("[security.human]") ||
    text.includes("[security.agent]") ||
    text.includes("[agent-mode]") ||
    text.includes("[[security.overrides]]")
  ) {
    return true;
  }
  // Proper TOML check for nested keys (e.g. agent under security)
  try {
    const parsed = parse(text) as Record<string, unknown>;
    const security = parsed.security;
    if (security && typeof security === "object" && !Array.isArray(security)) {
      const sec = security as Record<string, unknown>;
      // "overrides" is intentionally NOT checked here — the new schema also has
      // security.overrides as a flat array. Only the per-mode subkeys indicate v1.
      if ("human" in sec || "agent" in sec) return true;
    }
    if ("agent-mode" in parsed) return true;
  } catch {
    // ignore
  }
  return false;
}

export async function detectV1StateGlobal(): Promise<V1StateMarkers> {
  const dir = getConfigDir();
  const installedPath = join(dir, "installed.json");
  const pluginsPath = join(dir, "plugins.json");
  const configPath = join(dir, "config.toml");

  const [installedExists, pluginsExists, configExists] = await Promise.all([
    fileExists(installedPath),
    fileExists(pluginsPath),
    fileExists(configPath),
  ]);

  return {
    scope: "global",
    installedJson: installedExists ? installedPath : null,
    pluginsJson: pluginsExists ? pluginsPath : null,
    configToml: configExists ? configPath : null,
    configHasV1Keys: configExists ? await configIsV1(configPath) : false,
  };
}

export async function detectV1StateProject(
  projectRoot: string,
): Promise<V1StateMarkers> {
  const installedPath = join(projectRoot, ".agents", "installed.json");
  const pluginsPath = join(projectRoot, ".agents", "plugins.json");

  const [installedExists, pluginsExists] = await Promise.all([
    fileExists(installedPath),
    fileExists(pluginsPath),
  ]);

  return {
    scope: "project",
    installedJson: installedExists ? installedPath : null,
    pluginsJson: pluginsExists ? pluginsPath : null,
    configToml: null,
    configHasV1Keys: false,
  };
}

export function hasAnyV1Markers(markers: V1StateMarkers): boolean {
  return (
    markers.installedJson !== null ||
    markers.pluginsJson !== null ||
    markers.configHasV1Keys
  );
}
