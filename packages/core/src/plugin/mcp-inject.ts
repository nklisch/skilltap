import { mkdir } from "node:fs/promises";
import { dirname, join } from "node:path";
import { debug } from "../debug";
import { scopeBase } from "../paths";
import type { StoredMcpComponent } from "../schemas/plugins";
import { err, ok, type Result, UserError } from "../types";

// --- Agent MCP config registry ---

/**
 * Maps agent IDs to their MCP config file path (relative to base).
 * Base is globalBase() for global scope, projectRoot for project scope.
 */
export const MCP_AGENT_CONFIGS: Record<string, string> = {
  "claude-code": ".claude/settings.json",
  cursor: ".cursor/mcp.json",
  codex: ".codex/mcp.json",
  gemini: ".gemini/settings.json",
  windsurf: ".windsurf/mcp.json",
};

// --- Namespacing ---

const SKILLTAP_MCP_PREFIX = "skilltap:";

export function namespaceMcpServer(pluginName: string, serverName: string): string {
  return `${SKILLTAP_MCP_PREFIX}${pluginName}:${serverName}`;
}

export function isNamespacedKey(key: string): boolean {
  return key.startsWith(SKILLTAP_MCP_PREFIX);
}

export function parseNamespacedKey(
  key: string,
): { pluginName: string; serverName: string } | null {
  if (!key.startsWith(SKILLTAP_MCP_PREFIX)) return null;
  const parts = key.split(":");
  // parts[0] = "skilltap", parts[1] = pluginName, parts[2..] = serverName segments
  if (parts.length < 3) return null;
  return {
    pluginName: parts[1],
    serverName: parts.slice(2).join(":"),
  };
}

// --- Variable substitution ---

export type McpVarContext = {
  pluginRoot: string;
  pluginData: string;
};

function substituteVars(s: string, ctx: McpVarContext): string {
  return s
    .replaceAll("${CLAUDE_PLUGIN_ROOT}", ctx.pluginRoot)
    .replaceAll("${CLAUDE_PLUGIN_DATA}", ctx.pluginData);
}

export function substituteMcpVars(
  component: StoredMcpComponent,
  ctx: McpVarContext,
): StoredMcpComponent {
  if (component.serverType === "http") {
    return {
      ...component,
      url: substituteVars(component.url, ctx),
      headers: Object.fromEntries(
        Object.entries(component.headers).map(([k, v]) => [k, substituteVars(v, ctx)]),
      ),
    };
  }
  return {
    ...component,
    command: substituteVars(component.command, ctx),
    args: component.args.map((a) => substituteVars(a, ctx)),
    env: Object.fromEntries(
      Object.entries(component.env).map(([k, v]) => [k, substituteVars(v, ctx)]),
    ),
  };
}

// --- Config file I/O ---

export function mcpConfigPath(
  agent: string,
  scope: "global" | "project",
  projectRoot?: string,
): string | null {
  const relPath = MCP_AGENT_CONFIGS[agent];
  if (!relPath) return null;
  return join(scopeBase(scope, projectRoot), relPath);
}

async function readConfigJson(
  path: string,
): Promise<Result<Record<string, unknown>, UserError>> {
  const f = Bun.file(path);
  const exists = await f.exists();
  if (!exists) return ok({});

  let text: string;
  try {
    text = await f.text();
  } catch (e) {
    return err(new UserError(`Failed to read ${path}: ${e}`));
  }

  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch (e) {
    return err(new UserError(`Invalid JSON in ${path}: ${e}`));
  }

  if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
    return err(new UserError(`Expected a JSON object in ${path}`));
  }

  return ok(parsed as Record<string, unknown>);
}

async function writeConfigJson(
  path: string,
  data: Record<string, unknown>,
): Promise<Result<void, UserError>> {
  try {
    await mkdir(dirname(path), { recursive: true });
    await Bun.write(path, JSON.stringify(data, null, 2));
    return ok(undefined);
  } catch (e) {
    return err(new UserError(`Failed to write ${path}: ${e}`));
  }
}

async function backupIfNeeded(path: string): Promise<void> {
  const f = Bun.file(path);
  if (!(await f.exists())) return;

  const backupPath = path + ".skilltap.bak";
  if (await Bun.file(backupPath).exists()) return;

  try {
    await Bun.write(backupPath, f);
  } catch {
    // Backup is best-effort — don't fail the whole operation
  }
}

// --- Public API ---

export type InjectOptions = {
  pluginName: string;
  servers: StoredMcpComponent[];
  agents: string[];
  scope: "global" | "project";
  projectRoot?: string;
  vars?: McpVarContext;
};

/**
 * Inject MCP server entries into target agent config files.
 * Creates the config file if it doesn't exist.
 * Backs up before first modification.
 * Idempotent — re-injection replaces existing entries with same key.
 *
 * Returns the list of agents that were successfully injected into.
 */
export async function injectMcpServers(
  options: InjectOptions,
): Promise<Result<string[], UserError>> {
  const { pluginName, agents, scope, projectRoot, vars } = options;

  const servers = vars
    ? options.servers.map((s) => substituteMcpVars(s, vars))
    : options.servers;

  const injected: string[] = [];

  for (const agent of agents) {
    const configPath = mcpConfigPath(agent, scope, projectRoot);
    if (!configPath) {
      debug("mcp-inject: skipping unknown agent", { agent });
      continue;
    }

    const readResult = await readConfigJson(configPath);
    if (!readResult.ok) return readResult;

    const config = readResult.value;

    await backupIfNeeded(configPath);

    if (
      typeof config.mcpServers !== "object" ||
      config.mcpServers === null ||
      Array.isArray(config.mcpServers)
    ) {
      config.mcpServers = {};
    }

    const mcpServers = config.mcpServers as Record<string, unknown>;

    for (const server of servers) {
      const key = namespaceMcpServer(pluginName, server.name);
      let entry: Record<string, unknown>;

      if (server.serverType === "http") {
        entry = { url: server.url };
        if (Object.keys(server.headers).length > 0) {
          entry.headers = server.headers;
        }
      } else {
        entry = { command: server.command, args: server.args };
        if (Object.keys(server.env).length > 0) {
          entry.env = server.env;
        }
      }

      mcpServers[key] = entry;
    }

    const writeResult = await writeConfigJson(configPath, config);
    if (!writeResult.ok) return writeResult;

    injected.push(agent);
  }

  return ok(injected);
}

export type RemoveOptions = {
  pluginName: string;
  agents: string[];
  scope: "global" | "project";
  projectRoot?: string;
};

/**
 * Remove all MCP server entries for a plugin from target agent config files.
 * Only removes entries with the skilltap: namespace prefix.
 * If no skilltap entries remain and the config was MCP-only, leaves an empty mcpServers object.
 */
export async function removeMcpServers(
  options: RemoveOptions,
): Promise<Result<string[], UserError>> {
  const { pluginName, agents, scope, projectRoot } = options;
  const removed: string[] = [];

  for (const agent of agents) {
    const configPath = mcpConfigPath(agent, scope, projectRoot);
    if (!configPath) {
      debug("mcp-inject: skipping unknown agent", { agent });
      continue;
    }

    const readResult = await readConfigJson(configPath);
    if (!readResult.ok) return readResult;

    const config = readResult.value;
    if (Object.keys(config).length === 0) continue; // file didn't exist

    if (
      typeof config.mcpServers !== "object" ||
      config.mcpServers === null ||
      Array.isArray(config.mcpServers)
    ) {
      continue;
    }

    const mcpServers = config.mcpServers as Record<string, unknown>;
    const prefix = `${SKILLTAP_MCP_PREFIX}${pluginName}:`;

    for (const key of Object.keys(mcpServers)) {
      if (key.startsWith(prefix)) {
        delete mcpServers[key];
      }
    }

    const writeResult = await writeConfigJson(configPath, config);
    if (!writeResult.ok) return writeResult;

    removed.push(agent);
  }

  return ok(removed);
}

/**
 * List all skilltap-managed MCP server keys in an agent's config file.
 * Returns empty array if config file doesn't exist.
 */
export async function listMcpServers(
  agent: string,
  scope: "global" | "project",
  projectRoot?: string,
): Promise<Result<string[], UserError>> {
  const configPath = mcpConfigPath(agent, scope, projectRoot);
  if (!configPath) return ok([]);

  const readResult = await readConfigJson(configPath);
  if (!readResult.ok) return readResult;

  const config = readResult.value;
  if (Object.keys(config).length === 0) return ok([]);

  if (
    typeof config.mcpServers !== "object" ||
    config.mcpServers === null ||
    Array.isArray(config.mcpServers)
  ) {
    return ok([]);
  }

  const keys = Object.keys(config.mcpServers as Record<string, unknown>).filter(isNamespacedKey);
  return ok(keys);
}
