import { multiselect } from "@clack/prompts";
import {
  findComponentInPlugin,
  type PluginRecord,
  parseComponentRef,
  type StoredComponent,
  toggleInstalledComponent,
} from "@skilltap/core";
import { defineCommand } from "citty";
import { createOutput } from "../output";
import { ansi } from "../ui/format";
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
    const out = createOutput({ json: args.json, quiet: false });
    const ref = parseComponentRef(args.target as string);
    const projectRoot = await tryFindProjectRoot();

    const plugin = await loadPluginByName(ref.name, projectRoot);
    if (!plugin) {
      out.error(
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
        out.error(
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
        out.error(result.error.message);
        process.exit(1);
      }
      const action = result.value.nowActive ? "Enabled" : "Disabled";
      if (args.json) {
        out.json({
          plugin: plugin.name,
          component: result.value.component,
          nowActive: result.value.nowActive,
          action,
        });
        return;
      }
      out.success(`${action} ${componentLabel(result.value.component)}`);
      return;
    }

    await runPicker(out, plugin, projectRoot, args.json as boolean);
  },
});

async function runPicker(
  out: ReturnType<typeof createOutput>,
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
    out.info("No changes.");
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
    out.json(results);
    return;
  }
  for (const r of results) {
    if (r.error) {
      out.error(`Failed to toggle ${componentLabel(r.component)}: ${r.error}`);
    } else {
      const action = r.nowActive ? "Enabled" : "Disabled";
      out.success(`${action} ${componentLabel(r.component)}`);
    }
  }
}
