import { join } from "node:path";
import { type PluginManifest } from "../schemas/plugin";
import { ok, type Result, UserError } from "../types";
import { parseClaudePlugin } from "./parse-claude";
import { parseCodexPlugin } from "./parse-codex";

/**
 * Detect and parse a plugin from a cloned directory.
 *
 * Priority: Claude Code (.claude-plugin/plugin.json) → Codex (.codex-plugin/plugin.json).
 * Returns null if no plugin manifest found (the repo is a plain skill repo).
 *
 * @param dir - Absolute path to the cloned repo root
 */
export async function detectPlugin(
  dir: string,
): Promise<Result<PluginManifest | null, UserError>> {
  if (await Bun.file(join(dir, ".claude-plugin", "plugin.json")).exists()) {
    return parseClaudePlugin(dir);
  }

  if (await Bun.file(join(dir, ".codex-plugin", "plugin.json")).exists()) {
    return parseCodexPlugin(dir);
  }

  return ok(null);
}
