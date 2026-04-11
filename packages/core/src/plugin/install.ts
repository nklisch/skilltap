import { mkdir } from "node:fs/promises";
import { dirname, join } from "node:path";
import { $ } from "bun";
import { getConfigDir } from "../config";
import { globalBase } from "../fs";
import { skillInstallDir } from "../paths";
import type { PluginAgentComponent, PluginManifest, PluginMcpComponent } from "../schemas/plugin";
import type { PluginRecord, StoredMcpComponent } from "../schemas/plugins";
import { scanStatic } from "../security/static";
import { wrapShell } from "../shell";
import { createAgentSymlinks } from "../symlink";
import { err, ok, type Result, type ScanError, UserError } from "../types";
import { injectMcpServers } from "./mcp-inject";
import { addPlugin, loadPlugins, manifestToRecord, savePlugins } from "./state";
import type { StaticWarning } from "../security/static";

export type PluginInstallOptions = {
  scope: "global" | "project";
  projectRoot?: string;
  also?: string[];
  skipScan?: boolean;
  /** Called when static security warnings found. Return true to proceed. */
  onWarnings?: (warnings: StaticWarning[], pluginName: string) => Promise<boolean>;
  /** Called before placement for confirmation. Return false to cancel. */
  onConfirm?: (manifest: PluginManifest) => Promise<boolean>;
  /** Repo URL for recording */
  repo: string | null;
  /** Git ref */
  ref: string | null;
  /** Git SHA */
  sha: string | null;
  /** Tap name if installed from a tap */
  tap: string | null;
};

export type PluginInstallResult = {
  record: PluginRecord;
  warnings: StaticWarning[];
  /** List of agents where MCP was injected */
  mcpAgents: string[];
  /** Number of agent definitions placed */
  agentDefsPlaced: number;
};

/**
 * Install a plugin from a pre-cloned directory.
 *
 * 1. Security scan all plugin content
 * 2. Place skills in .agents/skills/ with agent symlinks
 * 3. Inject MCP server configs into target agent config files
 * 4. Place agent definitions in .claude/agents/
 * 5. Record plugin in plugins.json
 */
export async function installPlugin(
  contentDir: string,
  manifest: PluginManifest,
  options: PluginInstallOptions,
): Promise<Result<PluginInstallResult, UserError | ScanError>> {
  const { scope, projectRoot, also = [], skipScan } = options;

  // 1. Security scan
  let warnings: StaticWarning[] = [];
  if (!skipScan) {
    const scanResult = await scanStatic(contentDir);
    if (!scanResult.ok) return scanResult;
    warnings = scanResult.value;

    if (warnings.length > 0) {
      if (!options.onWarnings) {
        return err(
          new UserError(
            `Security warnings found in plugin "${manifest.name}". Aborting.`,
            "Use skipScan to bypass (not recommended).",
          ),
        );
      }
      const proceed = await options.onWarnings(warnings, manifest.name);
      if (!proceed) {
        return err(new UserError(`Install of plugin "${manifest.name}" cancelled due to security warnings.`));
      }
    }
  }

  // 2. Place skills
  const skillComponents = manifest.components.filter((c) => c.type === "skill");
  for (const component of skillComponents) {
    const src = join(contentDir, component.path);
    const dest = skillInstallDir(component.name, scope, projectRoot);

    const mkdirResult = await wrapShell(
      () => mkdir(dirname(dest), { recursive: true }).then(() => undefined),
      `Failed to create skill directory for "${component.name}"`,
    );
    if (!mkdirResult.ok) return mkdirResult;

    const cpResult = await wrapShell(
      () => $`cp -a ${src} ${dest}`.quiet().then(() => undefined),
      `Failed to copy skill "${component.name}"`,
      "Check that the skill path exists in the plugin.",
    );
    if (!cpResult.ok) return cpResult;

    if (also.length > 0) {
      const symlinkResult = await createAgentSymlinks(component.name, dest, also, scope, projectRoot);
      if (!symlinkResult.ok) return symlinkResult;
    }
  }

  // 3. Inject MCP servers
  const mcpComponents = manifest.components.filter(
    (c): c is PluginMcpComponent => c.type === "mcp",
  );
  const storedMcpComponents: StoredMcpComponent[] = [];
  for (const component of mcpComponents) {
    const server = component.server;
    if (server.type === "http") continue;
    storedMcpComponents.push({
      type: "mcp",
      name: server.name,
      active: true,
      command: server.command,
      args: server.args ?? [],
      env: server.env ?? {},
    });
  }

  let mcpAgents: string[] = [];
  if (storedMcpComponents.length > 0 && also.length > 0) {
    const vars = {
      pluginRoot: contentDir,
      pluginData: join(getConfigDir(), "plugin-data", manifest.name),
    };
    const injectResult = await injectMcpServers({
      pluginName: manifest.name,
      servers: storedMcpComponents,
      agents: also,
      scope,
      projectRoot,
      vars,
    });
    if (!injectResult.ok) return injectResult;
    mcpAgents = injectResult.value;
  }

  // 4. Place agent definitions
  const agentComponents = manifest.components.filter(
    (c): c is PluginAgentComponent => c.type === "agent",
  );
  const base = scope === "global" ? globalBase() : (projectRoot ?? process.cwd());
  let agentDefsPlaced = 0;
  for (const component of agentComponents) {
    const src = join(contentDir, component.path);
    const dest = join(base, ".claude", "agents", component.name + ".md");

    try {
      await mkdir(dirname(dest), { recursive: true });
      await Bun.write(dest, Bun.file(src));
      agentDefsPlaced++;
    } catch (e) {
      return err(new UserError(`Failed to place agent definition "${component.name}": ${e}`));
    }
  }

  // 5. Record in plugins.json
  const record = manifestToRecord(manifest, {
    repo: options.repo,
    ref: options.ref,
    sha: options.sha,
    scope,
    also,
    tap: options.tap,
  });

  const loadResult = await loadPlugins(projectRoot);
  if (!loadResult.ok) return loadResult;
  const newState = addPlugin(loadResult.value, record);
  const saveResult = await savePlugins(newState, projectRoot);
  if (!saveResult.ok) return saveResult;

  return ok({ record, warnings, mcpAgents, agentDefsPlaced });
}
