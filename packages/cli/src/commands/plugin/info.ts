import type { StoredComponent } from "@skilltap/core";
import { loadPlugins } from "@skilltap/core";
import { defineCommand } from "citty";
import { ansi } from "../../ui/format";
import { tryFindProjectRoot } from "../../ui/resolve";
import { createOutput } from "../../output";

function componentStatusIcon(c: StoredComponent): string {
  return c.active ? ansi.green("✓") : ansi.dim("✗");
}

function componentKind(c: StoredComponent): string {
  return c.type;
}

export default defineCommand({
  meta: {
    name: "info",
    description: "Show plugin details",
  },
  args: {
    name: {
      type: "positional",
      description: "Plugin name",
      required: true,
    },
    json: {
      type: "boolean",
      description: "Output as JSON",
      default: false,
    },
  },
  async run({ args }) {
    const out = createOutput({ json: args.json, quiet: false });
    const projectRoot = await tryFindProjectRoot();

    const globalResult = await loadPlugins();
    if (!globalResult.ok) {
      out.error(globalResult.error.message);
      process.exit(1);
    }

    const projectResult = projectRoot ? await loadPlugins(projectRoot) : null;

    const allPlugins = [
      ...globalResult.value.plugins,
      ...(projectResult?.ok ? projectResult.value.plugins : []),
    ];

    const plugin = allPlugins.find((p) => p.name === args.name);

    if (!plugin) {
      out.error(
        `Plugin '${args.name}' is not installed`,
        "Run 'skilltap plugin' to see installed plugins.",
      );
      process.exit(1);
    }

    if (args.json) {
      out.json(plugin);
      return;
    }

    const rows: [string, string][] = [
      ["name:", ansi.bold(plugin.name)],
      ["description:", plugin.description || "—"],
      ["scope:", plugin.scope],
      ["source:", plugin.repo ?? "local"],
      ["format:", plugin.format],
      ["ref:", plugin.ref ?? "—"],
      ["sha:", plugin.sha ? plugin.sha.slice(0, 7) : "—"],
      ["agents:", plugin.also.length > 0 ? plugin.also.join(", ") : "none"],
      ["installed:", plugin.installedAt],
      ["updated:", plugin.updatedAt],
    ];

    for (const [key, val] of rows) {
      process.stdout.write(`${ansi.dim(key.padEnd(13))} ${val}\n`);
    }

    if (plugin.components.length > 0) {
      process.stdout.write("\n");

      const skills = plugin.components.filter((c) => c.type === "skill");
      const mcps = plugin.components.filter((c) => c.type === "mcp");
      const agents = plugin.components.filter((c) => c.type === "agent");

      if (skills.length > 0) {
        process.stdout.write(`${ansi.bold("Skills:")}\n`);
        for (const c of skills) {
          process.stdout.write(`  ${componentStatusIcon(c)} ${c.name}\n`);
        }
      }

      if (mcps.length > 0) {
        process.stdout.write(`${ansi.bold("MCP Servers:")}\n`);
        for (const c of mcps) {
          process.stdout.write(`  ${componentStatusIcon(c)} ${c.name}\n`);
        }
      }

      if (agents.length > 0) {
        process.stdout.write(`${ansi.bold("Agent Definitions:")}\n`);
        for (const c of agents) {
          process.stdout.write(
            `  ${componentStatusIcon(c)} ${c.name} ${ansi.dim(`(${componentKind(c)})`)}\n`,
          );
        }
      }
    }
  },
});
