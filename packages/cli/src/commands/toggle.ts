import { multiselect } from "@clack/prompts";
import { pickOne } from "../ui/picker";
import {
  disableSkill,
  enableSkill,
  findComponentInPlugin,
  loadState,
  type Output,
  parseComponentRef,
  type PluginRecord,
  type StoredComponent,
  toggleInstalledComponent,
} from "@skilltap/core";
import { defineCommand } from "citty";
import { setupOutput } from "../ui/setup";
import { ansi } from "../ui/format";
import { componentLabel, loadPluginByName } from "../ui/plugin-format";
import { tryFindProjectRoot } from "../ui/resolve";

const VALID_TOGGLE_TYPES = ["skill", "plugin", "mcp"] as const;
type ToggleType = (typeof VALID_TOGGLE_TYPES)[number];

export default defineCommand({
  meta: {
    name: "toggle",
    description:
      "Toggle a skill, plugin, or component active state. Bare opens a picker.",
  },
  args: {
    type: {
      type: "positional",
      required: false,
      description: "skill | plugin | mcp",
    },
    target: {
      type: "positional",
      required: false,
      description: "Name (or plugin name:component for plugins)",
    },
    json: {
      type: "boolean",
      description: "Output as JSON",
      default: false,
    },
  },
  async run({ args }) {
    const out = setupOutput(args);
    const typeArg = args.type as string | undefined;
    const target = args.target as string | undefined;

    // Bare invocation — open picker
    if (!typeArg && !target) {
      if (process.stdout.isTTY !== true) {
        out.error(
          "Toggle requires arguments in non-interactive mode.",
          "Usage: skilltap toggle <type> <target>",
        );
        process.exit(1);
      }
      return runTogglePicker(out);
    }

    // One arg but not both
    if (!typeArg || !target) {
      out.error(
        "Toggle requires both type and target.",
        "Usage: skilltap toggle <type> <target>  |  skilltap toggle (no args opens picker)",
      );
      process.exit(1);
    }

    if (!VALID_TOGGLE_TYPES.includes(typeArg as ToggleType)) {
      out.error(
        `Invalid type: "${typeArg}".`,
        `Valid types: ${VALID_TOGGLE_TYPES.join(", ")}`,
      );
      process.exit(1);
    }

    return runToggle(typeArg as ToggleType, target, out, args.json as boolean);
  },
});

// ─── Direct toggle ────────────────────────────────────────────────────────────

async function runToggle(
  type: ToggleType,
  target: string,
  out: Output,
  json: boolean,
): Promise<void> {
  const projectRoot = await tryFindProjectRoot();

  if (type === "skill") {
    await runToggleSkill(target, out, json, projectRoot);
    return;
  }
  if (type === "plugin") {
    await runTogglePlugin(target, out, json, projectRoot);
    return;
  }
  // type === "mcp"
  await runToggleMcp(target, out, json, projectRoot);
}

async function runToggleSkill(
  name: string,
  out: Output,
  json: boolean,
  projectRoot: string | undefined,
): Promise<void> {
  // Load state to determine current active status
  const stateResult = await loadState(projectRoot);
  if (!stateResult.ok) {
    out.error(stateResult.error.message);
    process.exit(1);
  }

  const skill = stateResult.value.skills.find((s) => s.name === name);
  if (!skill) {
    out.error(
      `Skill '${name}' is not installed.`,
      "Run 'skilltap status' to see installed skills.",
    );
    process.exit(1);
  }

  const isCurrentlyActive = skill.active !== false;
  let result: Awaited<ReturnType<typeof disableSkill>>;

  // Pass the record's scope so disable/enable resolve the correct state file.
  // Without this, scope=undefined causes them to read GLOBAL state and miss
  // project-scoped records (and vice versa).
  const recordScope: "global" | "project" | "linked" = skill.scope;
  if (isCurrentlyActive) {
    result = await disableSkill(name, { scope: recordScope, projectRoot });
  } else {
    result = await enableSkill(name, { scope: recordScope, projectRoot });
  }

  if (!result.ok) {
    out.error(result.error.message, result.error.hint);
    process.exit(1);
  }

  const nowActive = !isCurrentlyActive;
  const action = nowActive ? "Enabled" : "Disabled";

  if (json) {
    out.json({ skill: name, nowActive, action });
    return;
  }
  out.success(`${action} skill ${ansi.bold(name)}`);
}

async function runTogglePlugin(
  target: string,
  out: Output,
  json: boolean,
  projectRoot: string | undefined,
): Promise<void> {
  const ref = parseComponentRef(target);
  const plugin = await loadPluginByName(ref.name, projectRoot);

  if (!plugin) {
    out.error(
      `Plugin '${ref.name}' is not installed.`,
      "Run 'skilltap status' to see installed plugins.",
    );
    process.exit(1);
  }

  if (ref.component) {
    // Toggle a specific component: name:component
    const component = findComponentInPlugin(plugin, ref.component);
    if (!component) {
      const available =
        plugin.components.map((c) => c.name).join(", ") || "(none)";
      out.error(
        `Component '${ref.component}' not found in plugin '${ref.name}'.`,
        `Available: ${available}`,
      );
      process.exit(1);
    }

    const result = await toggleInstalledComponent(
      plugin.name,
      component.type,
      component.name,
      { projectRoot },
    );
    if (!result.ok) {
      out.error(result.error.message);
      process.exit(1);
    }

    const action = result.value.nowActive ? "Enabled" : "Disabled";
    if (json) {
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

  // Bare plugin name — open component multi-picker
  await runPluginComponentPicker(out, plugin, projectRoot, json);
}

async function runPluginComponentPicker(
  out: Output,
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
      results.push({ component: c, nowActive: c.active, error: r.error.message });
    } else {
      results.push({ component: r.value.component, nowActive: r.value.nowActive });
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

async function runToggleMcp(
  name: string,
  out: Output,
  json: boolean,
  projectRoot: string | undefined,
): Promise<void> {
  // MCP standalone toggle is not yet implemented in core (no active/inactive
  // concept for standalone MCP servers in the current schema). For now,
  // inform the user.
  const stateResult = await loadState(projectRoot);
  if (!stateResult.ok) {
    out.error(stateResult.error.message);
    process.exit(1);
  }

  const server = stateResult.value.mcpServers.find((m) => m.name === name);
  if (!server) {
    out.error(
      `MCP server '${name}' is not installed.`,
      "Run 'skilltap status' to see installed MCP servers.",
    );
    process.exit(1);
  }

  out.info(
    `MCP server toggle is not yet implemented. Use 'skilltap remove mcp ${name}' to uninstall.`,
  );
}

// ─── Picker ──────────────────────────────────────────────────────────────────

async function runTogglePicker(
  out: Output,
): Promise<void> {
  const projectRoot = await tryFindProjectRoot();

  const type = await pickOne<ToggleType>({
    message: "What do you want to toggle?",
    options: [
      { value: "skill" as const, label: "Skill" },
      { value: "plugin" as const, label: "Plugin" },
      { value: "mcp" as const, label: "MCP server" },
    ],
    out,
  });
  if (!type) return;

  // Step 2: load items for the chosen type
  const stateResult = await loadState(projectRoot);
  if (!stateResult.ok) {
    out.error(stateResult.error.message);
    process.exit(1);
  }

  if (type === "skill") {
    const skillName = await pickOne<string>({
      message: "Which skill?",
      options: stateResult.value.skills.map((s) => ({
        value: s.name,
        label: s.name,
        hint: s.active !== false ? "enabled" : "disabled",
      })),
      emptyMessage: "No skills installed.",
      out,
    });
    if (!skillName) return;
    return runToggleSkill(skillName, out, false, projectRoot);
  }

  if (type === "plugin") {
    const pluginName = await pickOne<string>({
      message: "Which plugin?",
      options: stateResult.value.plugins.map((p) => ({
        value: p.name,
        label: p.name,
      })),
      emptyMessage: "No plugins installed.",
      out,
    });
    if (!pluginName) return;
    const plugin = await loadPluginByName(pluginName, projectRoot);
    if (!plugin) {
      out.error(`Plugin '${pluginName}' could not be loaded.`);
      process.exit(1);
    }
    return runPluginComponentPicker(out, plugin, projectRoot, false);
  }

  // type === "mcp"
  const mcpName = await pickOne<string>({
    message: "Which MCP server?",
    options: stateResult.value.mcpServers.map((m) => ({
      value: m.name,
      label: m.name,
    })),
    emptyMessage: "No MCP servers installed.",
    out,
  });
  if (!mcpName) return;
  return runToggleMcp(mcpName, out, false, projectRoot);
}
