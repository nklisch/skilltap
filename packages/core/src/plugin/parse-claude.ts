import { join, relative, resolve } from "node:path";
import { scan } from "../scanner";
import { ClaudePluginJsonSchema, type PluginManifest } from "../schemas/plugin";
import { err, ok, type Result, UserError } from "../types";
import { parseAgentDefinitions } from "./agents";
import { parseMcpJson, parseMcpObject } from "./mcp";

/**
 * Parse a Claude Code plugin from a directory containing .claude-plugin/plugin.json.
 *
 * @param pluginDir - Absolute path to the plugin root (parent of .claude-plugin/)
 */
export async function parseClaudePlugin(
  pluginDir: string,
): Promise<Result<PluginManifest, UserError>> {
  const manifestPath = join(pluginDir, ".claude-plugin", "plugin.json");
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

  const parsed = ClaudePluginJsonSchema.safeParse(raw);
  if (!parsed.success) {
    return err(new UserError(`Invalid plugin.json: missing required field "name"`));
  }
  const manifest = parsed.data;

  // --- Skills ---
  const skillComponents: PluginManifest["components"] = [];
  if (manifest.skills !== undefined) {
    const skillPaths = Array.isArray(manifest.skills) ? manifest.skills : [manifest.skills];
    for (const skillPath of skillPaths) {
      const absDir = resolve(pluginDir, skillPath);
      let skills: Awaited<ReturnType<typeof scan>> = [];
      try {
        skills = await scan(absDir);
      } catch {
        // Path override points to non-existent directory — treat as no skills
      }
      for (const skill of skills) {
        skillComponents.push({
          type: "skill",
          name: skill.name,
          path: relative(pluginDir, skill.path),
          description: skill.description,
        });
      }
    }
  } else {
    const skills = await scan(pluginDir);
    for (const skill of skills) {
      skillComponents.push({
        type: "skill",
        name: skill.name,
        path: relative(pluginDir, skill.path),
        description: skill.description,
      });
    }
  }

  // --- MCP ---
  const mcpComponents: PluginManifest["components"] = [];
  if (manifest.mcpServers !== undefined) {
    if (typeof manifest.mcpServers === "string") {
      const absPath = resolve(pluginDir, manifest.mcpServers);
      const mcpResult = await parseMcpJson(absPath);
      if (!mcpResult.ok) return mcpResult;
      for (const server of mcpResult.value) {
        mcpComponents.push({ type: "mcp", server });
      }
    } else if (Array.isArray(manifest.mcpServers)) {
      for (const mcpPath of manifest.mcpServers) {
        if (typeof mcpPath !== "string") continue;
        const absPath = resolve(pluginDir, mcpPath);
        const mcpResult = await parseMcpJson(absPath);
        if (!mcpResult.ok) return mcpResult;
        for (const server of mcpResult.value) {
          mcpComponents.push({ type: "mcp", server });
        }
      }
    } else if (typeof manifest.mcpServers === "object") {
      const mcpResult = parseMcpObject(manifest.mcpServers as Record<string, unknown>);
      if (!mcpResult.ok) return mcpResult;
      for (const server of mcpResult.value) {
        mcpComponents.push({ type: "mcp", server });
      }
    }
  } else {
    const mcpResult = await parseMcpJson(join(pluginDir, ".mcp.json"));
    if (!mcpResult.ok) return mcpResult;
    for (const server of mcpResult.value) {
      mcpComponents.push({ type: "mcp", server });
    }
  }

  // --- Agents ---
  const agentComponents: PluginManifest["components"] = [];
  if (manifest.agents !== undefined) {
    const agentPaths = Array.isArray(manifest.agents) ? manifest.agents : [manifest.agents];
    for (const agentPath of agentPaths) {
      const absDir = resolve(pluginDir, agentPath);
      const agentResult = await parseAgentDefinitions(absDir, pluginDir);
      if (!agentResult.ok) return agentResult;
      agentComponents.push(...agentResult.value);
    }
  } else {
    const agentResult = await parseAgentDefinitions(join(pluginDir, "agents"), pluginDir);
    if (!agentResult.ok) return agentResult;
    agentComponents.push(...agentResult.value);
  }

  return ok({
    name: manifest.name,
    version: manifest.version,
    description: manifest.description ?? "",
    format: "claude-code",
    pluginRoot: pluginDir,
    components: [...skillComponents, ...mcpComponents, ...agentComponents],
  });
}
