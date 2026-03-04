import { intro, isCancel, outro, spinner } from "@clack/prompts";
import type { TapEntry } from "@skilltap/core";
import {
  installSkill,
  loadTaps,
  searchTaps,
  skillInstallDir,
} from "@skilltap/core";
import { defineCommand } from "citty";
import pc from "picocolors";
import { errorLine, successLine, termWidth, truncate } from "../../ui/format";
import { createInstallCallbacks } from "../../ui/install-callbacks";
import { loadPolicyOrExit } from "../../ui/policy";
import { parseAlsoFlag, resolveScope } from "../../ui/resolve";
import { searchPrompt } from "../../ui/search-prompt";

export default defineCommand({
  meta: {
    name: "install",
    description: "Install skills from your configured taps",
  },
  args: {
    tap: {
      description: "Only show skills from this tap",
      valueHint: "name",
    },
    project: {
      type: "boolean",
      description: "Install to .agents/skills/ in current project",
      default: false,
    },
    global: {
      type: "boolean",
      description: "Install to ~/.agents/skills/",
      default: false,
    },
    also: {
      description: "Create symlink in agent-specific directory",
      valueHint: "agent",
    },
    "skip-scan": {
      type: "boolean",
      description: "Skip security scanning",
      default: false,
    },
    yes: {
      type: "boolean",
      alias: "y",
      description: "Auto-select all skills and install",
      default: false,
    },
    strict: {
      type: "boolean",
      description: "Abort on any security warning",
    },
    "no-strict": {
      type: "boolean",
      description: "Override config on_warn=fail for this invocation",
    },
    semantic: {
      type: "boolean",
      description: "Force semantic scan",
      default: false,
    },
  },
  async run({ args }) {
    const { config, policy } = await loadPolicyOrExit({
      strict: args.strict,
      noStrict: args["no-strict"],
      skipScan: args["skip-scan"],
      yes: args.yes,
      semantic: args.semantic,
      project: args.project,
      global: args.global,
    });

    // Load all tap entries
    const tapsResult = await loadTaps();
    if (!tapsResult.ok) {
      errorLine(tapsResult.error.message, tapsResult.error.hint);
      process.exit(1);
    }

    let tapEntries = tapsResult.value;

    if (tapEntries.length === 0) {
      errorLine(
        "No taps configured.",
        "Run 'skilltap tap add <name> <url>' to add one.",
      );
      process.exit(1);
    }

    // Filter by --tap if specified
    if (args.tap) {
      const tapNames = [...new Set(tapEntries.map((e) => e.tapName))];
      if (!tapNames.includes(args.tap)) {
        errorLine(
          `Tap '${args.tap}' is not configured.`,
          `Configured taps: ${tapNames.join(", ")}`,
        );
        process.exit(1);
      }
      tapEntries = tapEntries.filter((e) => e.tapName === args.tap);
    }

    // Select skills
    let selected: TapEntry[];

    if (policy.yes) {
      selected = tapEntries;
    } else {
      const width = termWidth();
      const maxLabelWidth = Math.max(40, width - 10);

      const result = await searchPrompt({
        message: "Select tap skills to install:",
        placeholder: "type to filter…",
        multiselect: true,
        source: (query, _signal) => {
          if (!query.trim()) return tapEntries;
          return searchTaps(tapEntries, query);
        },
        selector: (e) =>
          `${e.skill.name} ${e.skill.description} ${e.skill.tags.join(" ")}`,
        renderItem: (entry, active, _positions, selected) => {
          const checkbox = selected
            ? pc.green("◆")
            : active
              ? pc.cyan("◇")
              : pc.dim("◇");
          const rawName = truncate(entry.skill.name, 30);
          const nameStr = active ? pc.cyan(rawName) : pc.dim(rawName);
          const source = `[${entry.tapName}]`;
          const fixedCols = rawName.length + 2 + source.length + 2;
          const descSpace = Math.max(0, maxLabelWidth - fixedCols);
          const desc = entry.skill.description
            ? truncate(entry.skill.description, descSpace)
            : "";
          const pad = Math.max(
            1,
            maxLabelWidth - rawName.length - desc.length - source.length - 2,
          );
          return `${checkbox} ${nameStr}  ${pc.dim(desc)}${" ".repeat(pad)}${pc.dim(source)}`;
        },
      });

      if (isCancel(result)) process.exit(2);
      selected = result as TapEntry[];
    }

    if (selected.length === 0) {
      process.exit(0);
    }

    intro("skilltap");

    const { scope, projectRoot } = await resolveScope(
      { project: args.project, global: args.global },
      config,
    );
    const also = parseAlsoFlag(args.also, config);

    const errors: { name: string; message: string }[] = [];

    for (const entry of selected) {
      const skillName = entry.skill.name;
      const s = spinner();
      s.start(`Installing ${skillName}…`);

      const callbacks = createInstallCallbacks({
        spinner: s,
        onWarn: policy.onWarn,
        skipScan: policy.skipScan,
        agent: undefined,
        yes: policy.yes,
      });

      const result = await installSkill(skillName, {
        scope,
        projectRoot,
        also,
        skipScan: policy.skipScan,
        ...callbacks,
        onSelectSkills: async (skills) => {
          const match = skills.find((sk) => sk.name === skillName);
          return match ? [match.name] : skills.map((sk) => sk.name);
        },
        onSelectTap: async (matches) => matches[0] ?? null,
      });

      if (!result.ok) {
        s.stop(`Failed.`, 1);
        errors.push({ name: skillName, message: result.error.message });
      } else {
        s.stop(`Done.`);
        for (const record of result.value.records) {
          successLine(
            `${record.name} → ${skillInstallDir(record.name, scope, projectRoot)}`,
          );
        }
      }
    }

    if (errors.length > 0) {
      for (const { name, message } of errors) {
        errorLine(`${name}: ${message}`);
      }
      outro("Finished with errors.");
      process.exit(1);
    }

    outro("Done.");
  },
});
