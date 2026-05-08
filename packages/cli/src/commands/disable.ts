import {
  findComponentInPlugin,
  parseComponentRef,
  type StoredComponent,
  toggleInstalledComponent,
} from "@skilltap/core";
import { defineCommand } from "citty";
import { errorLine, jsonLine, successLine } from "../ui/format";
import { componentLabel, loadPluginByName } from "../ui/plugin-format";
import { tryFindProjectRoot } from "../ui/resolve";

export default defineCommand({
  meta: {
    name: "disable",
    description:
      "Disable a plugin component (name:component) or all active components (bare name)",
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
    const ref = parseComponentRef(args.target as string);
    const projectRoot = await tryFindProjectRoot();

    const plugin = await loadPluginByName(ref.name, projectRoot);
    if (!plugin) {
      errorLine(
        `Plugin '${ref.name}' is not installed`,
        "Run 'skilltap plugin' to see installed plugins.",
      );
      process.exit(1);
    }

    if (ref.component) {
      const component = findComponentInPlugin(plugin, ref.component);
      if (!component) {
        const available =
          plugin.components.map((c) => c.name).join(", ") || "(none)";
        errorLine(
          `Component '${ref.component}' not found in plugin '${ref.name}'`,
          `Available: ${available}`,
        );
        process.exit(1);
      }
      if (!component.active) {
        if (args.json) {
          jsonLine({
            plugin: plugin.name,
            component,
            action: "noop",
            nowActive: false,
          });
        } else {
          successLine(`${componentLabel(component)} is already disabled`);
        }
        return;
      }
      const result = await toggleInstalledComponent(
        plugin.name,
        component.type,
        component.name,
        {
          projectRoot,
        },
      );
      if (!result.ok) {
        errorLine(result.error.message);
        process.exit(1);
      }
      if (args.json) {
        jsonLine({
          plugin: plugin.name,
          component: result.value.component,
          action: "disabled",
          nowActive: result.value.nowActive,
        });
        return;
      }
      successLine(`Disabled ${componentLabel(result.value.component)}`);
      return;
    }

    const active = plugin.components.filter((c) => c.active);
    if (active.length === 0) {
      if (args.json)
        jsonLine({ plugin: plugin.name, action: "noop", active: 0 });
      else
        process.stdout.write(
          `No active components in plugin '${plugin.name}'.\n`,
        );
      return;
    }

    const results: {
      component: StoredComponent;
      nowActive: boolean;
      action: "disabled" | "failed";
      error?: string;
    }[] = [];
    for (const c of active) {
      const r = await toggleInstalledComponent(plugin.name, c.type, c.name, {
        projectRoot,
      });
      if (!r.ok) {
        results.push({
          component: c,
          nowActive: true,
          action: "failed",
          error: r.error.message,
        });
      } else {
        results.push({
          component: r.value.component,
          nowActive: r.value.nowActive,
          action: "disabled",
        });
      }
    }

    if (args.json) {
      jsonLine({ plugin: plugin.name, results });
      return;
    }
    for (const r of results) {
      if (r.action === "failed") {
        errorLine(
          `Failed to disable ${componentLabel(r.component)}: ${r.error}`,
        );
      } else {
        successLine(`Disabled ${componentLabel(r.component)}`);
      }
    }
  },
});
