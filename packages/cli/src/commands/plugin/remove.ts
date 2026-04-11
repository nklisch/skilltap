import { confirm } from "@clack/prompts";
import { defineCommand } from "citty";
import { loadPlugins, removeInstalledPlugin } from "@skilltap/core";
import type { PluginRecord } from "@skilltap/core";
import { agentError, exitWithError, outputJson } from "../../ui/agent-out";
import { ansi, errorLine, successLine } from "../../ui/format";
import { isAgentMode } from "../../ui/policy";
import { tryFindProjectRoot } from "../../ui/resolve";

function componentSummary(record: PluginRecord): string {
  const counts = { skill: 0, mcp: 0, agent: 0 };
  for (const c of record.components) {
    counts[c.type]++;
  }
  const parts: string[] = [];
  if (counts.skill > 0) parts.push(`${counts.skill} ${counts.skill === 1 ? "skill" : "skills"}`);
  if (counts.mcp > 0) parts.push(`${counts.mcp} ${counts.mcp === 1 ? "MCP server" : "MCP servers"}`);
  if (counts.agent > 0) parts.push(`${counts.agent} ${counts.agent === 1 ? "agent" : "agents"}`);
  return parts.join(", ") || "no components";
}

export default defineCommand({
  meta: {
    name: "remove",
    description: "Remove a plugin and all its components",
  },
  args: {
    name: {
      type: "positional",
      description: "Plugin name",
      required: true,
    },
    yes: {
      type: "boolean",
      alias: "y",
      description: "Skip confirmation prompt",
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

    const summary = componentSummary(plugin);

    if (agentMode) {
      const result = await removeInstalledPlugin(plugin.name, { projectRoot });
      if (!result.ok) {
        agentError(result.error.message);
        process.exit(1);
      }
      if (args.json) {
        outputJson({ removed: result.value.name, components: summary });
      } else {
        process.stdout.write(`OK: Removed plugin ${plugin.name} (${summary})\n`);
      }
      return;
    }

    if (!args.yes) {
      process.stdout.write(
        `Remove plugin ${ansi.bold(plugin.name)}? This will remove ${summary}.\n`,
      );
      const confirmed = await confirm({ message: "Continue?" });
      if (!confirmed || typeof confirmed === "symbol") {
        process.stdout.write("Cancelled.\n");
        process.exit(0);
      }
    }

    const result = await removeInstalledPlugin(plugin.name, { projectRoot });
    if (!result.ok) {
      errorLine(result.error.message, result.error.hint);
      process.exit(1);
    }

    if (args.json) {
      outputJson({ removed: result.value.name, components: summary });
    } else {
      successLine(`Removed plugin ${plugin.name} (${summary})`);
    }
  },
});
