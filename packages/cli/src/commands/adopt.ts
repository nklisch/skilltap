import { isCancel, select } from "@clack/prompts";
import {
  adoptAgentPlugin,
  adoptSkill,
  adoptSkillFromPath,
  discoverAllAdoptable,
  type DiscoveredAgentPlugin,
  type Output,
} from "@skilltap/core";
import { defineCommand } from "citty";
import { isAbsolute } from "node:path";
import { setupOutput } from "../ui/setup";
import { parseAlsoFlag, resolveScope } from "../ui/resolve";

export const adoptCommand = defineCommand({
  meta: {
    name: "adopt",
    description: "Bring an external skill or agent-managed plugin into skilltap",
  },
  args: {
    target: {
      type: "positional",
      required: false,
      description: "External path, or name of an unmanaged skill or agent-managed plugin",
    },
    source: {
      type: "string",
      description: "Filter picker to one source (e.g., claude-code)",
    },
    project: {
      type: "boolean",
      default: false,
      description: "Adopt into project scope",
    },
    global: {
      type: "boolean",
      default: false,
      description: "Adopt into global scope",
    },
    also: {
      type: "string",
      description: "Comma-separated agent dirs to symlink into",
    },
    move: {
      type: "boolean",
      default: false,
      description: "When adopting a path: physically move dir (default: track-in-place symlink)",
    },
    "skip-scan": {
      type: "boolean",
      default: false,
      description: "Skip security scan",
    },
    yes: {
      type: "boolean",
      default: false,
      alias: "y",
      description: "Auto-accept all prompts",
    },
    json: {
      type: "boolean",
      default: false,
      description: "Output as JSON",
    },
  },
  async run({ args }) {
    const out = setupOutput(args);

    if (!args.target) {
      // Picker mode (Phase 44 will replace with Ink TUI).
      if (process.stdout.isTTY !== true) {
        out.error(
          "adopt requires a target in non-interactive mode.",
          "Usage: skilltap adopt <path-or-name> | adopt --source claude-code",
        );
        process.exit(1);
      }
      return runAdoptPicker(out, args);
    }

    if (looksLikePath(args.target)) {
      return runAdoptPath(args.target, out, args);
    }
    return runAdoptName(args.target, out, args);
  },
});

function looksLikePath(s: string): boolean {
  return (
    s.startsWith("./") ||
    s.startsWith("/") ||
    s.startsWith("~/") ||
    isAbsolute(s) ||
    s.includes("/")
  );
}

type AdoptArgs = {
  project: boolean;
  global: boolean;
  also: string | undefined;
  move: boolean;
  "skip-scan": boolean;
  yes: boolean;
  json: boolean;
  source: string | undefined;
  target: string | undefined;
};

// ─── Picker mode ──────────────────────────────────────────────────────────────

async function runAdoptPicker(
  out: Output,
  args: AdoptArgs,
): Promise<void> {
  const progress = out.progress("Scanning for adoptable skills and plugins");

  const { scope, projectRoot } = await resolveScope(args);

  const result = await discoverAllAdoptable({
    ...(scope === "project" ? { project: true as const } : { global: true as const }),
    unmanagedOnly: true,
    projectRoot,
  });

  if (!result.ok) {
    progress.fail("Scan failed");
    out.error(result.error.message, result.error.hint);
    process.exit(1);
  }

  for (const { scanner, error } of result.value.scannerErrors) {
    out.warn(`Scanner "${scanner}" failed: ${error.message}`);
  }

  let skills = result.value.skills;
  let plugins = result.value.plugins;

  // Filter by --source
  if (args.source) {
    plugins = plugins.filter((p) => p.scannerName === args.source);
    // Skills have no scannerName; when --source is set, skip them
    skills = [];
  }

  type PickerItem =
    | { kind: "skill"; skill: (typeof skills)[number] }
    | { kind: "plugin"; plugin: DiscoveredAgentPlugin };

  const options: Array<{ value: PickerItem; label: string; hint?: string }> = [
    ...skills.map((s) => ({
      value: { kind: "skill" as const, skill: s },
      label: `skill: ${s.name}`,
      hint: s.locations[0]?.path,
    })),
    ...plugins.map((p) => ({
      value: { kind: "plugin" as const, plugin: p },
      label: `plugin: ${p.name}${p.marketplaceName ? `@${p.marketplaceName}` : ""}`,
      hint: `managed by ${p.scannerName}`,
    })),
  ];

  progress.succeed("Scan complete");

  if (options.length === 0) {
    out.info("Nothing to adopt.");
    return;
  }

  const chosen = await select({
    message: "Select a skill or plugin to adopt:",
    options,
  });

  if (isCancel(chosen)) {
    out.info("Cancelled.");
    return;
  }

  const item = chosen as PickerItem;
  const also = parseAlsoFlag(args.also, undefined);
  const mode = args.move ? "move" : "track-in-place";

  if (item.kind === "skill") {
    const adoptResult = await adoptSkill(item.skill, {
      mode,
      scope,
      projectRoot,
      also,
      skipScan: args["skip-scan"],
      onWarnings: args.yes
        ? undefined
        : async (warnings, skillName) => {
            out.warn(`Security warnings for '${skillName}':`);
            for (const w of warnings) {
              out.warn(`  ${w.file}: ${w.category}`);
            }
            return true;
          },
    });
    if (!adoptResult.ok) {
      out.error(adoptResult.error.message, adoptResult.error.hint);
      process.exit(1);
    }
    out.success(`Adopted skill ${item.skill.name} (${mode})`);
    if (args.json) {
      out.json({ kind: "adopt:done", type: "skill", name: item.skill.name, mode, scope });
    }
    return;
  }

  // plugin
  const adoptResult = await adoptAgentPlugin(item.plugin, { also, projectRoot });
  if (!adoptResult.ok) {
    out.error(adoptResult.error.message, adoptResult.error.hint);
    process.exit(1);
  }
  out.success(
    `Adopted plugin ${item.plugin.name}${item.plugin.marketplaceName ? `@${item.plugin.marketplaceName}` : ""}`,
  );
  if (args.json) {
    out.json({
      kind: "adopt:done",
      type: "plugin",
      name: item.plugin.name,
      scanner: item.plugin.scannerName,
      path: item.plugin.installPath,
    });
  }
}

// ─── Path mode ────────────────────────────────────────────────────────────────

async function runAdoptPath(
  path: string,
  out: Output,
  args: AdoptArgs,
): Promise<void> {
  const { scope, projectRoot } = await resolveScope(args);
  const also = parseAlsoFlag(args.also, undefined);
  const mode = args.move ? "move" : "track-in-place";

  const result = await adoptSkillFromPath(path, {
    mode,
    scope,
    projectRoot,
    also,
    skipScan: args["skip-scan"],
    onWarnings: args.yes
      ? undefined
      : async (warnings, skillName) => {
          out.warn(`Security warnings for '${skillName}':`);
          for (const w of warnings) {
            out.warn(`  ${w.file}: ${w.category}`);
          }
          return true;
        },
  });

  if (!result.ok) {
    out.error(result.error.message, result.error.hint);
    process.exit(1);
  }

  const { record } = result.value;
  out.success(
    `Adopted skill from ${path} (${mode})${mode === "track-in-place" ? " — use --move to relocate" : ""}`,
  );
  if (args.json) {
    out.json({ kind: "adopt:done", type: "skill", name: record.name, mode, scope, path });
  }
}

// ─── Name mode ────────────────────────────────────────────────────────────────

async function runAdoptName(
  name: string,
  out: Output,
  args: AdoptArgs,
): Promise<void> {
  const { scope, projectRoot } = await resolveScope(args);
  const also = parseAlsoFlag(args.also, undefined);
  const mode = args.move ? "move" : "track-in-place";

  const progress = out.progress(`Looking up "${name}"`);

  const result = await discoverAllAdoptable({
    ...(scope === "project" ? { project: true as const } : { global: true as const }),
    unmanagedOnly: true,
    projectRoot,
  });

  if (!result.ok) {
    progress.fail("Discovery failed");
    out.error(result.error.message, result.error.hint);
    process.exit(1);
  }

  progress.succeed("Discovery complete");

  const { skills, plugins, scannerErrors } = result.value;
  for (const { scanner, error } of scannerErrors) {
    out.warn(`Scanner "${scanner}" failed: ${error.message}`);
  }

  // Check skills first
  const skill = skills.find((s) => s.name === name);
  if (skill) {
    const adoptResult = await adoptSkill(skill, {
      mode,
      scope,
      projectRoot,
      also,
      skipScan: args["skip-scan"],
      onWarnings: args.yes
        ? undefined
        : async (warnings, skillName) => {
            out.warn(`Security warnings for '${skillName}':`);
            for (const w of warnings) {
              out.warn(`  ${w.file}: ${w.category}`);
            }
            return true;
          },
    });
    if (!adoptResult.ok) {
      out.error(adoptResult.error.message, adoptResult.error.hint);
      process.exit(1);
    }
    out.success(`Adopted skill ${name} (${mode})`);
    if (args.json) {
      out.json({ kind: "adopt:done", type: "skill", name, mode, scope });
    }
    return;
  }

  // Check plugins by name
  const plugin = plugins.find((p) => p.name === name);
  if (plugin) {
    const adoptResult = await adoptAgentPlugin(plugin, { also, projectRoot });
    if (!adoptResult.ok) {
      out.error(adoptResult.error.message, adoptResult.error.hint);
      process.exit(1);
    }
    out.success(
      `Adopted plugin ${name}${plugin.marketplaceName ? `@${plugin.marketplaceName}` : ""}`,
    );
    if (args.json) {
      out.json({
        kind: "adopt:done",
        type: "plugin",
        name,
        scanner: plugin.scannerName,
        path: plugin.installPath,
      });
    }
    return;
  }

  out.error(
    `No unmanaged skill or agent-managed plugin named "${name}".`,
    "Run 'skilltap status --unmanaged' to see unmanaged skills.",
  );
  process.exit(1);
}

export default adoptCommand;
