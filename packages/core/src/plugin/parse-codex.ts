import { join, resolve } from "node:path";
import { CodexPluginJsonSchema, type PluginManifest } from "../schemas/plugin";
import { err, ok, type Result, UserError } from "../types";
import { parseMcpJson } from "./mcp";
import { discoverSkills } from "./parse-common";

/**
 * Parse a Codex plugin from a directory containing .codex-plugin/plugin.json.
 *
 * @param pluginDir - Absolute path to the plugin root (parent of .codex-plugin/)
 */
export async function parseCodexPlugin(
  pluginDir: string,
): Promise<Result<PluginManifest, UserError>> {
  const manifestPath = join(pluginDir, ".codex-plugin", "plugin.json");
  const file = Bun.file(manifestPath);
  if (!(await file.exists())) {
    return err(new UserError(`No plugin.json found at ${manifestPath}`));
  }

  let raw: unknown;
  try {
    raw = JSON.parse(await file.text());
  } catch {
    return err(new UserError(`Invalid JSON in ${manifestPath}`));
  }

  const parsed = CodexPluginJsonSchema.safeParse(raw);
  if (!parsed.success) {
    return err(new UserError(`Invalid plugin.json: missing required fields (name, version, description)`));
  }
  const manifest = parsed.data;

  // --- Skills ---
  const skillComponents = await discoverSkills(pluginDir, manifest.skills);

  // --- MCP ---
  const mcpComponents: PluginManifest["components"] = [];
  if (manifest.mcpServers !== undefined) {
    const absPath = resolve(pluginDir, manifest.mcpServers);
    const mcpResult = await parseMcpJson(absPath);
    if (!mcpResult.ok) return mcpResult;
    for (const server of mcpResult.value) {
      mcpComponents.push({ type: "mcp", server });
    }
  } else {
    const mcpResult = await parseMcpJson(join(pluginDir, ".mcp.json"));
    if (!mcpResult.ok) return mcpResult;
    for (const server of mcpResult.value) {
      mcpComponents.push({ type: "mcp", server });
    }
  }

  // Codex plugins have no agent definitions
  return ok({
    name: manifest.name,
    version: manifest.version,
    description: manifest.description,
    format: "codex",
    pluginRoot: pluginDir,
    components: [...skillComponents, ...mcpComponents],
  });
}
