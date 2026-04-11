import { defineCommand } from "citty";
import { loadPlugins } from "@skilltap/core";
import type { PluginRecord } from "@skilltap/core";
import { outputJson } from "../../ui/agent-out";
import { ansi, table, termWidth, truncate } from "../../ui/format";
import { isAgentMode } from "../../ui/policy";
import { tryFindProjectRoot } from "../../ui/resolve";

function componentSummary(record: PluginRecord): string {
  const counts = { skill: 0, mcp: 0, agent: 0 };
  for (const c of record.components) {
    counts[c.type]++;
  }
  const parts: string[] = [];
  if (counts.skill > 0) parts.push(`${counts.skill} ${counts.skill === 1 ? "skill" : "skills"}`);
  if (counts.mcp > 0) parts.push(`${counts.mcp} ${counts.mcp === 1 ? "MCP" : "MCPs"}`);
  if (counts.agent > 0) parts.push(`${counts.agent} ${counts.agent === 1 ? "agent" : "agents"}`);
  return parts.length > 0 ? parts.join(", ") : "no components";
}

export default defineCommand({
  meta: {
    name: "plugin",
    description: "Manage installed plugins",
  },
  args: {
    global: { type: "boolean", description: "Show only global plugins", default: false },
    project: { type: "boolean", description: "Show only project plugins", default: false },
    json: { type: "boolean", description: "Output as JSON", default: false },
  },
  subCommands: {
    info: () => import("./info").then((m) => m.default),
    toggle: () => import("./toggle").then((m) => m.default),
    remove: () => import("./remove").then((m) => m.default),
  },
  async run({ args }) {
    if ((args._ as string[])?.length > 0) return;

    const agentMode = await isAgentMode();
    const projectRoot = await tryFindProjectRoot();

    const globalResult = await loadPlugins();
    if (!globalResult.ok) {
      process.stderr.write(`error: ${globalResult.error.message}\n`);
      process.exit(1);
    }

    const projectResult = projectRoot ? await loadPlugins(projectRoot) : null;

    type ScopedPlugin = PluginRecord & { _scope: "global" | "project" };

    let allPlugins: ScopedPlugin[] = [
      ...globalResult.value.plugins.map((p) => ({ ...p, _scope: "global" as const })),
      ...(projectResult?.ok
        ? projectResult.value.plugins.map((p) => ({ ...p, _scope: "project" as const }))
        : []),
    ];

    if (args.global) {
      allPlugins = allPlugins.filter((p) => p._scope === "global");
    } else if (args.project) {
      allPlugins = allPlugins.filter((p) => p._scope === "project");
    }

    if (args.json) {
      outputJson(allPlugins);
      return;
    }

    if (allPlugins.length === 0) {
      process.stdout.write("No plugins installed.\n");
      process.stdout.write("Run 'skilltap install <source>' to install a plugin.\n");
      return;
    }

    if (agentMode) {
      for (const plugin of allPlugins) {
        const source = plugin.repo ?? "local";
        process.stdout.write(
          `${plugin._scope.toUpperCase()} ${plugin.name} ${componentSummary(plugin)} source=${source}\n`,
        );
      }
      return;
    }

    const width = termWidth();

    const globalPlugins = allPlugins.filter((p) => p._scope === "global");
    const projectPlugins = allPlugins.filter((p) => p._scope === "project");

    function printSection(label: string, plugins: ScopedPlugin[]) {
      if (plugins.length === 0) return;
      const count = plugins.length;
      process.stdout.write(
        `\n${ansi.bold(label)} — ${count} ${count === 1 ? "plugin" : "plugins"}\n`,
      );

      const NAME_W = width < 60 ? 16 : 22;
      const COMP_W = width < 60 ? 20 : 28;
      const SRC_W = width < 60 ? 16 : 24;

      const rows = plugins.map((p) => [
        truncate(p.name, NAME_W),
        truncate(componentSummary(p), COMP_W),
        truncate(p.repo ?? "local", SRC_W),
      ]);

      process.stdout.write(
        `${table(rows, { header: ["Name", "Components", "Source"] })}\n`,
      );
    }

    printSection("Global plugins", globalPlugins);
    printSection("Project plugins", projectPlugins);

    process.stdout.write("\n");
  },
});
