import {
  findComponentInPlugin,
  loadPlugins,
  parseComponentRef,
  type PluginRecord,
  type StoredComponent,
  toggleInstalledComponent,
} from "@skilltap/core";
import { defineCommand } from "citty";
import { exitWithError, outputJson } from "../ui/agent-out";
import { errorLine, successLine } from "../ui/format";
import { isAgentMode } from "../ui/policy";
import { tryFindProjectRoot } from "../ui/resolve";

function componentLabel(c: StoredComponent): string {
  if (c.type === "skill") return `skill: ${c.name}`;
  if (c.type === "mcp") return `mcp: ${c.name}`;
  return `agent: ${c.name}`;
}

async function loadPluginByName(
  name: string,
  projectRoot: string | undefined,
): Promise<PluginRecord | null> {
  const globalResult = await loadPlugins();
  if (!globalResult.ok) return null;
  const projectResult = projectRoot ? await loadPlugins(projectRoot) : null;
  const all = [
    ...globalResult.value.plugins,
    ...(projectResult?.ok ? projectResult.value.plugins : []),
  ];
  return all.find((p) => p.name === name) ?? null;
}

export default defineCommand({
  meta: {
    name: "enable",
    description: "Enable a plugin component (name:component) or all inactive components (bare name)",
  },
  args: {
    target: {
      type: "positional",
      description: "Plugin name, optionally with :component suffix",
      required: true,
    },
    json: {
      type: "boolean",
      description: "Output as JSON",
      default: false,
    },
  },
  async run({ args }) {
    const agentMode = await isAgentMode();
    const ref = parseComponentRef(args.target as string);
    const projectRoot = await tryFindProjectRoot();

    const plugin = await loadPluginByName(ref.name, projectRoot);
    if (!plugin) {
      exitWithError(
        agentMode,
        `Plugin '${ref.name}' is not installed`,
        "Run 'skilltap plugin' to see installed plugins.",
      );
    }

    if (ref.component) {
      const component = findComponentInPlugin(plugin, ref.component);
      if (!component) {
        const available = plugin.components.map((c) => c.name).join(", ") || "(none)";
        exitWithError(
          agentMode,
          `Component '${ref.component}' not found in plugin '${ref.name}'`,
          `Available: ${available}`,
        );
      }
      if (component.active) {
        if (args.json) {
          outputJson({ plugin: plugin.name, component, action: "noop", nowActive: true });
        } else {
          successLine(`${componentLabel(component)} is already enabled`);
        }
        return;
      }
      const result = await toggleInstalledComponent(plugin.name, component.type, component.name, {
        projectRoot,
      });
      if (!result.ok) {
        errorLine(result.error.message);
        process.exit(1);
      }
      if (args.json) {
        outputJson({
          plugin: plugin.name,
          component: result.value.component,
          action: "enabled",
          nowActive: result.value.nowActive,
        });
        return;
      }
      successLine(`Enabled ${componentLabel(result.value.component)}`);
      return;
    }

    // Bare name — enable all currently inactive components
    const inactive = plugin.components.filter((c) => !c.active);
    if (inactive.length === 0) {
      if (args.json) outputJson({ plugin: plugin.name, action: "noop", inactive: 0 });
      else process.stdout.write(`No inactive components in plugin '${plugin.name}'.\n`);
      return;
    }

    const results: {
      component: StoredComponent;
      nowActive: boolean;
      action: "enabled" | "failed";
      error?: string;
    }[] = [];
    for (const c of inactive) {
      const r = await toggleInstalledComponent(plugin.name, c.type, c.name, { projectRoot });
      if (!r.ok) {
        results.push({ component: c, nowActive: false, action: "failed", error: r.error.message });
      } else {
        results.push({ component: r.value.component, nowActive: r.value.nowActive, action: "enabled" });
      }
    }

    if (args.json) {
      outputJson({ plugin: plugin.name, results });
      return;
    }
    for (const r of results) {
      if (r.action === "failed") {
        errorLine(`Failed to enable ${componentLabel(r.component)}: ${r.error}`);
      } else {
        successLine(`Enabled ${componentLabel(r.component)}`);
      }
    }
  },
});
