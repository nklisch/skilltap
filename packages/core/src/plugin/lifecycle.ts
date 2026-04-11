import { mkdir, rename, rm } from "node:fs/promises";
import { join } from "node:path";
import { globalBase } from "../fs";
import { skillDisabledDir, skillInstallDir } from "../paths";
import type { PluginsJson, StoredComponent, StoredMcpComponent } from "../schemas/plugins";
import { createAgentSymlinks, removeAgentSymlinks } from "../symlink";
import { err, ok, type Result, UserError } from "../types";
import { injectMcpServers, removeMcpServers } from "./mcp-inject";
import { findPlugin, loadPlugins, removePlugin, savePlugins, toggleComponent } from "./state";
import type { PluginRecord } from "../schemas/plugins";

export type RemovePluginOptions = {
  scope?: "global" | "project";
  projectRoot?: string;
};

export type ToggleComponentOptions = {
  projectRoot?: string;
};

export type ToggleResult = {
  component: StoredComponent;
  nowActive: boolean;
  mcpAgents: string[];
};

type ScopedState = {
  state: PluginsJson;
  scope: "global" | "project";
  projectRoot?: string;
};

async function findPluginInScopes(
  pluginName: string,
  options: { scope?: "global" | "project"; projectRoot?: string },
): Promise<Result<ScopedState & { record: PluginRecord }, UserError>> {
  const { scope, projectRoot } = options;

  if (scope === "global" || !scope) {
    const globalResult = await loadPlugins();
    if (!globalResult.ok) return globalResult;
    const record = findPlugin(globalResult.value, pluginName);
    if (record) {
      return ok({ state: globalResult.value, scope: "global", projectRoot: undefined, record });
    }
  }

  if (scope === "project" || (!scope && projectRoot)) {
    const projResult = await loadPlugins(projectRoot);
    if (!projResult.ok) return projResult;
    const record = findPlugin(projResult.value, pluginName);
    if (record) {
      return ok({ state: projResult.value, scope: "project", projectRoot, record });
    }
  }

  return err(new UserError(`Plugin "${pluginName}" not found`));
}

/**
 * Remove an installed plugin: delete skill dirs + agent symlinks,
 * remove MCP entries from agent configs, delete agent definition files,
 * and remove the record from plugins.json.
 */
export async function removeInstalledPlugin(
  pluginName: string,
  options?: RemovePluginOptions,
): Promise<Result<PluginRecord, UserError>> {
  const found = await findPluginInScopes(pluginName, options ?? {});
  if (!found.ok) return found;

  const { state, scope, projectRoot, record } = found.value;
  const base = scope === "global" ? globalBase() : (projectRoot ?? process.cwd());

  for (const component of record.components) {
    if (component.type === "skill") {
      const activeDir = skillInstallDir(component.name, scope, projectRoot);
      const disabledDir = skillDisabledDir(component.name, scope, projectRoot);
      await rm(activeDir, { recursive: true, force: true });
      await rm(disabledDir, { recursive: true, force: true });
      await removeAgentSymlinks(component.name, record.also, scope, projectRoot);
    } else if (component.type === "mcp") {
      const removeResult = await removeMcpServers({
        pluginName,
        agents: record.also,
        scope,
        projectRoot,
      });
      if (!removeResult.ok) return removeResult;
    } else if (component.type === "agent") {
      const activePath = join(base, ".claude", "agents", `${component.name}.md`);
      const disabledPath = join(base, ".claude", "agents", ".disabled", `${component.name}.md`);
      await rm(activePath, { force: true });
      await rm(disabledPath, { force: true });
    }
  }

  const newState = removePlugin(state, pluginName);
  const saveResult = await savePlugins(newState, scope === "project" ? projectRoot : undefined);
  if (!saveResult.ok) return saveResult;

  return ok(record);
}

/**
 * Toggle a single component within an installed plugin.
 * Handles filesystem moves for skills and agents, MCP injection/removal for MCPs.
 * Updates plugins.json state.
 */
export async function toggleInstalledComponent(
  pluginName: string,
  componentType: StoredComponent["type"],
  componentName: string,
  options?: ToggleComponentOptions,
): Promise<Result<ToggleResult, UserError>> {
  const { projectRoot } = options ?? {};

  const found = await findPluginInScopes(pluginName, { projectRoot });
  if (!found.ok) return found;

  const { state, scope, record } = found.value;
  const base = scope === "global" ? globalBase() : (projectRoot ?? process.cwd());

  const component = record.components.find(
    (c) => c.type === componentType && c.name === componentName,
  );
  if (!component) {
    return err(
      new UserError(
        `Component "${componentName}" of type "${componentType}" not found in plugin "${pluginName}"`,
      ),
    );
  }

  const wasActive = component.active;
  const nowActive = !wasActive;
  let mcpAgents: string[] = [];

  if (component.type === "skill") {
    const activeDir = skillInstallDir(component.name, scope, projectRoot);
    const disabledDir = skillDisabledDir(component.name, scope, projectRoot);

    if (wasActive) {
      // Deactivate: move to .disabled/, remove symlinks
      await mkdir(join(base, ".agents", "skills", ".disabled"), { recursive: true });
      await rename(activeDir, disabledDir);
      await removeAgentSymlinks(component.name, record.also, scope, projectRoot);
    } else {
      // Activate: move from .disabled/ back, recreate symlinks
      await rename(disabledDir, activeDir);
      const symlinkResult = await createAgentSymlinks(
        component.name,
        activeDir,
        record.also,
        scope,
        projectRoot,
      );
      if (!symlinkResult.ok) return symlinkResult;
    }
  } else if (component.type === "mcp") {
    if (wasActive) {
      // Deactivate: remove MCP entries for this plugin
      const removeResult = await removeMcpServers({
        pluginName,
        agents: record.also,
        scope,
        projectRoot,
      });
      if (!removeResult.ok) return removeResult;
      mcpAgents = removeResult.value;

      // Re-inject all other active MCP components (remove removed all for the plugin)
      const otherActive = record.components.filter(
        (c): c is StoredMcpComponent =>
          c.type === "mcp" && c.active && c.name !== component.name,
      );
      if (otherActive.length > 0) {
        const reinjectResult = await injectMcpServers({
          pluginName,
          servers: otherActive,
          agents: record.also,
          scope,
          projectRoot,
        });
        if (!reinjectResult.ok) return reinjectResult;
      }
    } else {
      // Activate: inject just this one MCP server
      const mcpComponent = component as StoredMcpComponent;
      const injectResult = await injectMcpServers({
        pluginName,
        servers: [mcpComponent],
        agents: record.also,
        scope,
        projectRoot,
      });
      if (!injectResult.ok) return injectResult;
      mcpAgents = injectResult.value;
    }
  } else if (component.type === "agent") {
    const agentDir = join(base, ".claude", "agents");
    const disabledDir = join(agentDir, ".disabled");
    const activePath = join(agentDir, `${component.name}.md`);
    const disabledPath = join(disabledDir, `${component.name}.md`);

    if (wasActive) {
      // Deactivate: move to .disabled/
      await mkdir(disabledDir, { recursive: true });
      await rename(activePath, disabledPath);
    } else {
      // Activate: move from .disabled/ back
      await rename(disabledPath, activePath);
    }
  }

  const toggleResult = toggleComponent(state, pluginName, componentType, componentName);
  if (!toggleResult.ok) return toggleResult;

  const saveResult = await savePlugins(
    toggleResult.value,
    scope === "project" ? projectRoot : undefined,
  );
  if (!saveResult.ok) return saveResult;

  return ok({ component, nowActive, mcpAgents });
}
