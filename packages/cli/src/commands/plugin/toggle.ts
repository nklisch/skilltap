import { multiselect } from "@clack/prompts";
import { defineCommand } from "citty";
import { loadPlugins, toggleInstalledComponent } from "@skilltap/core";
import type { StoredComponent } from "@skilltap/core";
import { agentError, exitWithError } from "../../ui/agent-out";
import { ansi, errorLine, successLine } from "../../ui/format";
import { isAgentMode } from "../../ui/policy";
import { tryFindProjectRoot } from "../../ui/resolve";

function componentLabel(c: StoredComponent): string {
  if (c.type === "skill") return `skill: ${c.name}`;
  if (c.type === "mcp") return `mcp: ${c.name}`;
  return `agent: ${c.name}`;
}

export default defineCommand({
  meta: {
    name: "toggle",
    description: "Enable/disable plugin components",
  },
  args: {
    name: {
      type: "positional",
      description: "Plugin name",
      required: true,
    },
    skills: {
      type: "boolean",
      description: "Toggle all skills",
      default: false,
    },
    mcps: {
      type: "boolean",
      description: "Toggle all MCP servers",
      default: false,
    },
    agents: {
      type: "boolean",
      description: "Toggle all agent definitions",
      default: false,
    },
    json: {
      type: "boolean",
      description: "Output as JSON",
      default: false,
    },
  },
  async run({ args }) {
    const agentMode = await isAgentMode();
    const projectRoot = await tryFindProjectRoot();

    const globalResult = await loadPlugins();
    if (!globalResult.ok) {
      exitWithError(agentMode, globalResult.error.message);
    }

    const projectResult = projectRoot ? await loadPlugins(projectRoot) : null;

    const allPlugins = [
      ...globalResult.value.plugins,
      ...(projectResult?.ok ? projectResult.value.plugins : []),
    ];

    const plugin = allPlugins.find((p) => p.name === args.name);
    if (!plugin) {
      exitWithError(
        agentMode,
        `Plugin '${args.name}' is not installed`,
        "Run 'skilltap plugin' to see installed plugins.",
      );
    }

    let toToggle: StoredComponent[] = [];

    const hasFilter = args.skills || args.mcps || args.agents;

    if (hasFilter) {
      if (args.skills) toToggle.push(...plugin.components.filter((c) => c.type === "skill"));
      if (args.mcps) toToggle.push(...plugin.components.filter((c) => c.type === "mcp"));
      if (args.agents) toToggle.push(...plugin.components.filter((c) => c.type === "agent"));
    } else if (agentMode) {
      agentError("Provide --skills, --mcps, or --agents to specify what to toggle.");
      process.exit(1);
    } else {
      // Interactive multiselect
      const options = plugin.components.map((c) => ({
        value: `${c.type}:${c.name}`,
        label: componentLabel(c),
        hint: c.active ? "currently enabled" : "currently disabled",
      }));

      const initialValues = plugin.components
        .filter((c) => c.active)
        .map((c) => `${c.type}:${c.name}`);

      const selected = await multiselect({
        message: `Select components to enable for ${ansi.bold(plugin.name)}:`,
        options,
        initialValues,
        required: false,
      });

      if (typeof selected === "symbol") {
        // User cancelled
        process.exit(0);
      }

      const selectedSet = new Set(selected as string[]);

      // Figure out what changed
      for (const c of plugin.components) {
        const key = `${c.type}:${c.name}`;
        const shouldBeActive = selectedSet.has(key);
        if (shouldBeActive !== c.active) {
          toToggle.push(c);
        }
      }
    }

    if (toToggle.length === 0) {
      process.stdout.write("No changes.\n");
      return;
    }

    const results: { component: StoredComponent; nowActive: boolean; error?: string }[] = [];

    for (const c of toToggle) {
      const result = await toggleInstalledComponent(plugin.name, c.type, c.name, { projectRoot });
      if (!result.ok) {
        results.push({ component: c, nowActive: c.active, error: result.error.message });
      } else {
        results.push({ component: result.value.component, nowActive: result.value.nowActive });
      }
    }

    if (args.json) {
      process.stdout.write(`${JSON.stringify(results, null, 2)}\n`);
      return;
    }

    for (const r of results) {
      if (r.error) {
        errorLine(`Failed to toggle ${componentLabel(r.component)}: ${r.error}`);
      } else {
        const action = r.nowActive ? "Enabled" : "Disabled";
        successLine(`${action} ${componentLabel(r.component)}`);
      }
    }
  },
});
