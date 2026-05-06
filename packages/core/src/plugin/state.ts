import { join } from "node:path";
import { ensureDirs, getConfigDir } from "../dirs";
import { loadJsonState, saveJsonState } from "../json-state";
import type { McpServerEntry, PluginManifest } from "../schemas/plugin";
import {
  type PluginRecord,
  type PluginsJson,
  PluginsJsonSchema,
  type StoredComponent,
  type StoredMcpComponent,
} from "../schemas/plugins";
import { loadState } from "../state/load";
import { saveState } from "../state/save";
import { err, ok, type Result, UserError } from "../types";

function getPluginsPath(projectRoot?: string): string {
  return projectRoot
    ? join(projectRoot, ".agents", "plugins.json")
    : join(getConfigDir(), "plugins.json");
}

// state.json is the canonical store. Same read-fallback + state-only-write
// pattern as loadInstalled/saveInstalled in config.ts.
export async function loadPlugins(
  projectRoot?: string,
): Promise<Result<PluginsJson, UserError>> {
  const stateResult = await loadState(projectRoot);
  if (stateResult.ok && stateResult.value.plugins.length > 0) {
    return ok({ version: 1 as const, plugins: stateResult.value.plugins });
  }
  if (!stateResult.ok) return stateResult;
  return loadJsonState(
    getPluginsPath(projectRoot),
    PluginsJsonSchema,
    "plugins.json",
    { version: 1 as const, plugins: [] },
  );
}

export async function savePlugins(
  plugins: PluginsJson,
  projectRoot?: string,
): Promise<Result<void, UserError>> {
  const stateResult = await loadState(projectRoot);
  if (!stateResult.ok) return stateResult;
  const newState = {
    version: 2 as const,
    skills: stateResult.value.skills,
    plugins: plugins.plugins,
    mcpServers: stateResult.value.mcpServers,
  };
  return saveState(newState, projectRoot);
}

export function mcpServerToStored(server: McpServerEntry): StoredMcpComponent {
  if (server.type === "http") {
    return {
      type: "mcp",
      serverType: "http",
      name: server.name,
      active: true,
      url: server.url,
      headers: server.headers ?? {},
    };
  }
  return {
    type: "mcp",
    serverType: "stdio",
    name: server.name,
    active: true,
    command: server.command,
    args: server.args ?? [],
    env: server.env ?? {},
  };
}

export function addPlugin(
  state: PluginsJson,
  record: PluginRecord,
): PluginsJson {
  const filtered = state.plugins.filter((p) => p.name !== record.name);
  return { ...state, plugins: [...filtered, record] };
}

export function removePlugin(
  state: PluginsJson,
  pluginName: string,
): PluginsJson {
  return {
    ...state,
    plugins: state.plugins.filter((p) => p.name !== pluginName),
  };
}

export function toggleComponent(
  state: PluginsJson,
  pluginName: string,
  componentType: StoredComponent["type"],
  componentName: string,
): Result<PluginsJson, UserError> {
  const plugin = state.plugins.find((p) => p.name === pluginName);
  if (!plugin) {
    return err(new UserError(`Plugin "${pluginName}" not found`));
  }

  const componentIndex = plugin.components.findIndex(
    (c) => c.type === componentType && c.name === componentName,
  );
  if (componentIndex === -1) {
    return err(
      new UserError(
        `Component "${componentName}" of type "${componentType}" not found in plugin "${pluginName}"`,
      ),
    );
  }

  const updatedAt = new Date().toISOString();
  const updatedComponents = plugin.components.map((c, i) =>
    i === componentIndex ? { ...c, active: !c.active } : c,
  );
  const updatedPlugin = { ...plugin, components: updatedComponents, updatedAt };
  const updatedPlugins = state.plugins.map((p) =>
    p.name === pluginName ? updatedPlugin : p,
  );

  return ok({ ...state, plugins: updatedPlugins });
}

export function findPlugin(
  state: PluginsJson,
  pluginName: string,
): PluginRecord | undefined {
  return state.plugins.find((p) => p.name === pluginName);
}

export type PluginInstallMeta = {
  repo: string | null;
  ref: string | null;
  sha: string | null;
  scope: "global" | "project";
  also: string[];
  tap: string | null;
};

export function manifestToRecord(
  manifest: PluginManifest,
  meta: PluginInstallMeta,
): PluginRecord {
  const now = new Date().toISOString();
  const components: StoredComponent[] = [];

  for (const component of manifest.components) {
    if (component.type === "skill") {
      components.push({ type: "skill", name: component.name, active: true });
    } else if (component.type === "mcp") {
      components.push(mcpServerToStored(component.server));
    } else if (component.type === "agent") {
      components.push({
        type: "agent",
        name: component.name,
        active: true,
        platform: "claude-code",
      });
    }
  }

  return {
    name: manifest.name,
    description: manifest.description ?? "",
    format: manifest.format,
    repo: meta.repo,
    ref: meta.ref,
    sha: meta.sha,
    scope: meta.scope,
    also: meta.also,
    tap: meta.tap,
    components,
    installedAt: now,
    updatedAt: now,
    active: true,
  };
}
