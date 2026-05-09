import {
  adoptPlugin,
  adoptSkill,
  disableSkill,
  discoverAllAdoptable,
  enableSkill,
  findComponentInPlugin,
  gatherStatus,
  installMcp,
  installSkill,
  isInGitRepo,
  loadState,
  toggleInstalledComponent,
} from "@skilltap/core";
import type { AdoptCandidate, AppContext, DashboardTab, FindResult, ToggleState } from "./state/types";

export async function createAppContext(): Promise<AppContext> {
  const projectRoot = (await isInGitRepo()) ?? undefined;

  return {
    dispatchInstall: async (type, source) => {
      try {
        if (type === "skill") {
          const result = await installSkill(source, {
            scope: projectRoot ? "project" : "global",
            projectRoot,
            onSelectSkills: async (skills) => skills.map((s) => s.name),
            onWarnings: async () => true,
            onConfirmInstall: async () => true,
            onAlreadyInstalled: async () => "update",
            onPluginDetected: async () => "skills-only",
            onPluginCaptureConfirm: async () => true,
            onPluginCaptureConflict: async () => "abort",
          });
          if (!result.ok) return { ok: false, error: result.error.message };
          return { ok: true };
        }
        if (type === "plugin") {
          const result = await installSkill(source, {
            scope: projectRoot ? "project" : "global",
            projectRoot,
            onSelectSkills: async (skills) => skills.map((s) => s.name),
            onWarnings: async () => true,
            onConfirmInstall: async () => true,
            onAlreadyInstalled: async () => "update",
            onPluginDetected: async () => "plugin",
            onPluginCaptureConfirm: async () => true,
            onPluginCaptureConflict: async () => "abort",
          });
          if (!result.ok) return { ok: false, error: result.error.message };
          return { ok: true };
        }
        if (type === "mcp") {
          const result = await installMcp(source, {
            scope: projectRoot ? "project" : "global",
            projectRoot,
          });
          if (!result.ok) return { ok: false, error: result.error.message };
          return { ok: true };
        }
        return { ok: false, error: `unknown type: ${type}` };
      } catch (e) {
        return { ok: false, error: e instanceof Error ? e.message : String(e) };
      }
    },

    dispatchToggle: async (type, name, component) => {
      try {
        if (type === "skill") {
          const stateResult = await loadState(projectRoot);
          if (!stateResult.ok) return { ok: false, error: stateResult.error.message };
          const skill = stateResult.value.skills.find((s) => s.name === name);
          if (!skill) return { ok: false, error: `Skill '${name}' not found` };
          const isActive = skill.active !== false;
          const result = isActive
            ? await disableSkill(name, { projectRoot })
            : await enableSkill(name, { projectRoot });
          if (!result.ok) return { ok: false, error: result.error.message };
          return { ok: true };
        }
        if (type === "plugin") {
          if (!component) return { ok: false, error: "component name required for plugin toggle" };
          const stateResult = await loadState(projectRoot);
          if (!stateResult.ok) return { ok: false, error: stateResult.error.message };
          const plugin = stateResult.value.plugins.find((p) => p.name === name);
          if (!plugin) return { ok: false, error: `Plugin '${name}' not found` };
          const comp = findComponentInPlugin(plugin, component);
          if (!comp) return { ok: false, error: `Component '${component}' not found in plugin '${name}'` };
          const result = await toggleInstalledComponent(name, comp.type, comp.name, { projectRoot });
          if (!result.ok) return { ok: false, error: result.error.message };
          return { ok: true };
        }
        if (type === "mcp") {
          return { ok: false, error: "MCP server toggle is not yet implemented. Use remove mcp." };
        }
        return { ok: false, error: `unknown type: ${type}` };
      } catch (e) {
        return { ok: false, error: e instanceof Error ? e.message : String(e) };
      }
    },

    dispatchAdopt: async (kind, name, mode) => {
      try {
        const candidates = await discoverAllAdoptable({
          scope: projectRoot ? "project" : "global",
          projectRoot,
        });
        if (!candidates.ok) return { ok: false, error: candidates.error.message };
        if (kind === "skill") {
          const skill = candidates.value.skills.find((s) => s.name === name);
          if (!skill) return { ok: false, error: `Skill '${name}' not found` };
          const result = await adoptSkill(skill, {
            mode,
            scope: projectRoot ? "project" : "global",
            projectRoot,
            skipScan: false,
            onWarnings: async () => true,
          });
          if (!result.ok) return { ok: false, error: result.error.message };
          return { ok: true };
        }
        if (kind === "plugin") {
          const plugin = candidates.value.plugins.find((p) => p.name === name);
          if (!plugin) return { ok: false, error: `Plugin '${name}' not found` };
          const result = await adoptPlugin(plugin, { projectRoot });
          if (!result.ok) return { ok: false, error: result.error.message };
          return { ok: true };
        }
        return { ok: false, error: `unknown kind: ${kind}` };
      } catch (e) {
        return { ok: false, error: e instanceof Error ? e.message : String(e) };
      }
    },

    dispatchSync: async () => {
      return { ok: false, error: "Sync is not yet implemented in TUI. Run `skilltap sync`." };
    },

    loadDashboardData: async (tab: DashboardTab): Promise<unknown> => {
      try {
        const status = await gatherStatus({ projectRoot });
        if (tab === "installed") {
          return {
            skills: status.skills,
            plugins: status.plugins,
          };
        }
        if (tab === "taps") return status.taps;
        if (tab === "updates") return [];
        if (tab === "drift") return status.drift;
        return null;
      } catch {
        return null;
      }
    },

    loadFindResults: async (_query: string): Promise<FindResult[]> => {
      return [];
    },

    loadToggleComponents: async (
      type: "skill" | "plugin" | "mcp",
      name: string,
    ): Promise<ToggleState["components"]> => {
      try {
        if (type !== "plugin") return [];
        const stateResult = await loadState(projectRoot);
        if (!stateResult.ok) return [];
        const plugin = stateResult.value.plugins.find((p) => p.name === name);
        if (!plugin) return [];
        return plugin.components.map((c) => ({ name: c.name, active: c.active ?? true }));
      } catch {
        return [];
      }
    },

    loadAdoptCandidates: async (): Promise<AdoptCandidate[]> => {
      try {
        const result = await discoverAllAdoptable({
          scope: projectRoot ? "project" : "global",
          projectRoot,
        });
        if (!result.ok) return [];
        return [
          ...result.value.skills.map((s) => ({
            kind: "skill" as const,
            name: s.name,
            source: s.locations[0]?.path ?? "(unknown)",
            description: s.description,
          })),
          ...result.value.plugins.map((p) => ({
            kind: "plugin" as const,
            name: p.name,
            source: p.marketplaceName
              ? `${p.scannerName}: ${p.marketplaceName}`
              : p.scannerName,
            description: p.manifest.description,
          })),
        ];
      } catch {
        return [];
      }
    },
  };
}
