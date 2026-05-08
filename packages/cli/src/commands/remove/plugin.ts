import { confirm } from "@clack/prompts";
import { loadPlugins, removeInstalledPlugin } from "@skilltap/core";
import { defineCommand } from "citty";
import { setupOutput } from "../../ui/setup";
import { ansi } from "../../ui/format";
import { componentSummary } from "../../ui/plugin-format";
import { tryFindProjectRoot } from "../../ui/resolve";

export const pluginRemoveCommand = defineCommand({
  meta: {
    name: "plugin",
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
    const out = setupOutput(args);
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
        "Run 'skilltap status' to see installed plugins.",
      );
      process.exit(1);
    }

    const summary = componentSummary(plugin);

    if (!args.yes) {
      out.info(
        `Remove plugin ${ansi.bold(plugin.name)}? This will remove ${summary}.`,
      );
      const confirmed = await confirm({ message: "Continue?" });
      if (!confirmed || typeof confirmed === "symbol") {
        out.info("Cancelled.");
        process.exit(0);
      }
    }

    const result = await removeInstalledPlugin(plugin.name, { projectRoot });
    if (!result.ok) {
      out.error(result.error.message, result.error.hint);
      process.exit(1);
    }

    out.json({ removed: result.value.name, components: summary });
    out.success(`Removed plugin ${plugin.name} (${summary})`);
  },
});
