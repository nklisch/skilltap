import { join } from "node:path";
import { discoverSkilltapPlugins } from "../plugin-v2/discover";
import type { PluginManifest } from "../schemas/plugin";
import { err, ok, type Result, UserError } from "../types";
import { parseClaudePlugin } from "./parse-claude";
import { parseCodexPlugin } from "./parse-codex";

export interface DetectOptions {
  /**
   * If `.skilltap/` contains multiple publishable plugins, pick the one
   * with this name. Required (and surfaced as an error) when multiple
   * exist and no name is given.
   */
  selectName?: string;
}

/**
 * Detect and parse a plugin from a cloned directory.
 *
 * Priority:
 *   1. `.skilltap/<name>.toml` (native v2.0 — multiple plugins supported)
 *   2. `.claude-plugin/plugin.json` (Claude Code, single plugin)
 *   3. `.codex-plugin/plugin.json` (Codex, single plugin)
 *
 * Returns null if no plugin manifest is found (the repo is a plain skill repo).
 *
 * @param dir - Absolute path to the cloned repo root
 * @param options - Optional. `selectName` picks one plugin from a multi-plugin
 *   `.skilltap/` repo by name.
 */
export async function detectPlugin(
  dir: string,
  options: DetectOptions = {},
): Promise<Result<PluginManifest | null, UserError>> {
  const skilltapResult = await discoverSkilltapPlugins(dir);
  if (!skilltapResult.ok) return skilltapResult;
  const skilltapManifests = skilltapResult.value.manifests;

  if (skilltapManifests.length > 0) {
    if (skilltapManifests.length === 1) {
      return ok(skilltapManifests[0]);
    }

    const { selectName } = options;
    if (selectName !== undefined) {
      const match = skilltapManifests.find((m) => m.name === selectName);
      if (!match) {
        const available = skilltapManifests.map((m) => m.name).join(", ");
        return err(
          new UserError(
            `Plugin "${selectName}" not found in this repo. Available: ${available}.`,
          ),
        );
      }
      return ok(match);
    }

    const names = skilltapManifests.map((m) => m.name).join(", ");
    return err(
      new UserError(
        `Multiple publishable plugins in this repo: ${names}. Specify one with user/repo:<name> or user/repo:*.`,
      ),
    );
  }

  if (await Bun.file(join(dir, ".claude-plugin", "plugin.json")).exists()) {
    return parseClaudePlugin(dir);
  }

  if (await Bun.file(join(dir, ".codex-plugin", "plugin.json")).exists()) {
    return parseCodexPlugin(dir);
  }

  return ok(null);
}

/**
 * List all publishable v2.0 plugins in a repo without selecting one.
 * Caller can use this to render a picker before calling `detectPlugin`
 * with `selectName`.
 */
export async function listPluginOptions(
  dir: string,
): Promise<Result<{ name: string; description: string }[], UserError>> {
  const result = await discoverSkilltapPlugins(dir);
  if (!result.ok) return result;
  return ok(
    result.value.manifests.map((m) => ({
      name: m.name,
      description: m.description,
    })),
  );
}
