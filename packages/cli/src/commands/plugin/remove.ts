import { confirm } from "@clack/prompts";
import { loadPlugins, removeInstalledPlugin } from "@skilltap/core";
import { defineCommand } from "citty";
import { ansi, errorLine, jsonLine, successLine } from "../../ui/format";
import { componentSummary } from "../../ui/plugin-format";
import { tryFindProjectRoot } from "../../ui/resolve";

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
    const projectRoot = await tryFindProjectRoot();

    const globalResult = await loadPlugins();
    if (!globalResult.ok) {
      errorLine(globalResult.error.message);
      process.exit(1);
    }

    const projectResult = projectRoot ? await loadPlugins(projectRoot) : null;

    const allPlugins = [
      ...globalResult.value.plugins,
      ...(projectResult?.ok ? projectResult.value.plugins : []),
    ];

    const plugin = allPlugins.find((p) => p.name === args.name);
    if (!plugin) {
      errorLine(
        `Plugin '${args.name}' is not installed`,
        "Run 'skilltap plugin' to see installed plugins.",
      );
      process.exit(1);
    }

    const summary = componentSummary(plugin);

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
      jsonLine({ removed: result.value.name, components: summary });
    } else {
      successLine(`Removed plugin ${plugin.name} (${summary})`);
    }
  },
});
