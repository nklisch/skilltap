import type { McpServerEntry, PluginManifest } from "../schemas/plugin";
import type {
  PluginRecord,
  StoredComponent,
  StoredMcpComponent,
} from "../schemas/plugins";
import { loadState } from "../state/load";
import { saveState } from "../state/save";
import { DEFAULT_AGENT_ID } from "../symlink";
import { err, ok, type Result, UserError } from "../types";

// state.json is the only canonical store. Plugin-slice accessor: read/write just plugins[].
export async function loadPlugins(
  projectRoot?: string,
): Promise<Result<PluginRecord[], UserError>> {
  const stateResult = await loadState(projectRoot);
  if (!stateResult.ok) return stateResult;
  return ok([...stateResult.value.plugins]);
}

export async function savePlugins(
  plugins: PluginRecord[],
  projectRoot?: string,
): Promise<Result<void, UserError>> {
  const stateResult = await loadState(projectRoot);
  if (!stateResult.ok) return stateResult;
  const newState = {
    version: 2 as const,
    skills: stateResult.value.skills,
    plugins,
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
  plugins: PluginRecord[],
  record: PluginRecord,
): PluginRecord[] {
  const filtered = plugins.filter((p) => p.name !== record.name);
  return [...filtered, record];
}

export function removePlugin(
  plugins: PluginRecord[],
  pluginName: string,
): PluginRecord[] {
  return plugins.filter((p) => p.name !== pluginName);
}

export function toggleComponent(
  plugins: PluginRecord[],
  pluginName: string,
  componentType: StoredComponent["type"],
  componentName: string,
): Result<PluginRecord[], UserError> {
  const plugin = plugins.find((p) => p.name === pluginName);
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
  return ok(plugins.map((p) => (p.name === pluginName ? updatedPlugin : p)));
}

export function findPlugin(
  plugins: PluginRecord[],
  pluginName: string,
): PluginRecord | undefined {
  return plugins.find((p) => p.name === pluginName);
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
        platform: DEFAULT_AGENT_ID,
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
