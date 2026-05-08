import { multiselect } from "@clack/prompts";
import {
  findComponentInPlugin,
  type PluginRecord,
  parseComponentRef,
  type StoredComponent,
  toggleInstalledComponent,
} from "@skilltap/core";
import { defineCommand } from "citty";
import { ansi, errorLine, jsonLine, successLine } from "../ui/format";
import { componentLabel, loadPluginByName } from "../ui/plugin-format";
import { tryFindProjectRoot } from "../ui/resolve";

export default defineCommand({
  meta: {
    name: "toggle",
    description: "Toggle a plugin component (name:component) or open a picker",
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
      const action = result.value.nowActive ? "Enabled" : "Disabled";
      if (args.json) {
        jsonLine({
          plugin: plugin.name,
          component: result.value.component,
          nowActive: result.value.nowActive,
          action,
        });
        return;
      }
      successLine(`${action} ${componentLabel(result.value.component)}`);
      return;
    }

    await runPicker(plugin, projectRoot, args.json as boolean);
  },
});

async function runPicker(
  plugin: PluginRecord,
  projectRoot: string | undefined,
  json: boolean,
): Promise<void> {
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

  if (typeof selected === "symbol") process.exit(0);
  const selectedSet = new Set(selected as string[]);

  const toToggle: StoredComponent[] = [];
  for (const c of plugin.components) {
    const key = `${c.type}:${c.name}`;
    const shouldBeActive = selectedSet.has(key);
    if (shouldBeActive !== c.active) toToggle.push(c);
  }

  if (toToggle.length === 0) {
    process.stdout.write("No changes.\n");
    return;
  }

  const results: {
    component: StoredComponent;
    nowActive: boolean;
    error?: string;
  }[] = [];
  for (const c of toToggle) {
    const r = await toggleInstalledComponent(plugin.name, c.type, c.name, {
      projectRoot,
    });
    if (!r.ok) {
      results.push({
        component: c,
        nowActive: c.active,
        error: r.error.message,
      });
    } else {
      results.push({
        component: r.value.component,
        nowActive: r.value.nowActive,
      });
    }
  }

  if (json) {
    jsonLine(results);
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
}
