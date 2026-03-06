import { intro, isCancel, outro, spinner } from "@clack/prompts";
import type { InstalledSkill, TapEntry } from "@skilltap/core";
import {
  ensureBuiltinTap,
  findProjectRoot,
  installSkill,
  isBuiltinTapCloned,
  loadConfig,
  loadInstalled,
  loadTaps,
  removeSkill,
  saveConfig,
  searchTaps,
  skillInstallDir,
} from "@skilltap/core";
import { defineCommand } from "citty";
import pc from "picocolors";
import { errorLine, successLine, termWidth, truncate } from "../../ui/format";
import { createInstallCallbacks } from "../../ui/install-callbacks";
import { createStepLogger } from "../../ui/install-steps";
import { loadPolicyOrExit } from "../../ui/policy";
import { confirmSaveDefault, selectAgents } from "../../ui/prompts";
import { parseAlsoFlag, resolveScope } from "../../ui/resolve";
import { searchPrompt } from "../../ui/search-prompt";

export default defineCommand({
  meta: {
    name: "skilltap tap install",
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

    // Ensure the built-in tap is cloned (only show spinner on first clone)
    const configResult = await loadConfig();
    if (configResult.ok && configResult.value.builtin_tap !== false) {
      const alreadyCloned = await isBuiltinTapCloned();
      if (!alreadyCloned) {
        const s = spinner();
        s.start("Fetching built-in skills tap…");
        const ensureResult = await ensureBuiltinTap();
        if (!ensureResult.ok) {
          s.stop("Could not reach built-in tap — continuing without it.");
        } else {
          s.stop("Built-in tap ready.");
        }
      }
    }

    // Load all tap entries
    const tapsResult = await loadTaps();
    if (!tapsResult.ok) {
      errorLine(tapsResult.error.message, tapsResult.error.hint);
      process.exit(1);
    }

    let tapEntries = tapsResult.value;

    if (tapEntries.length === 0) {
      errorLine(
        "No skills available.",
        "Check your connection, or add a tap with 'skilltap tap add <name> <url>'.",
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

    // Load installed skills for pre-selection
    const installedNames = new Set<string>();
    const installedSkills: InstalledSkill[] = [];
    const detectedProjectRoot = await findProjectRoot().catch(() => undefined);
    const globalInstalledResult = await loadInstalled();
    if (globalInstalledResult.ok) {
      for (const s of globalInstalledResult.value.skills) {
        installedNames.add(s.name);
        installedSkills.push(s);
      }
    }
    const projectInstalledResult = detectedProjectRoot
      ? await loadInstalled(detectedProjectRoot)
      : null;
    if (projectInstalledResult?.ok) {
      for (const s of projectInstalledResult.value.skills) {
        if (!installedNames.has(s.name)) {
          installedNames.add(s.name);
          installedSkills.push(s);
        }
      }
    }

    // Select skills
    let selected: TapEntry[];

    if (policy.yes) {
      selected = tapEntries;
    } else {
      const width = termWidth();
      const maxLabelWidth = Math.max(40, width - 10);

      const result = await searchPrompt<TapEntry>({
        message: "Select tap skills to install:",
        placeholder: "type to filter…",
        multiselect: true,
        allowEmpty: true,
        initialSelected: (e) => installedNames.has(e.skill.name),
        source: (query, _signal) => {
          if (!query.trim()) return tapEntries;
          return searchTaps(tapEntries, query);
        },
        selector: (e) =>
          `${e.skill.name} ${e.skill.description} ${e.skill.tags.join(" ")}`,
        renderItem: (entry, active, _positions, selected) => {
          const isInstalled = installedNames.has(entry.skill.name);
          const willRemove = isInstalled && !selected;
          const checkbox = willRemove
            ? active
              ? pc.red("◆")
              : pc.dim(pc.red("◆"))
            : selected
              ? pc.green("◆")
              : active
                ? pc.cyan("◇")
                : pc.dim("◇");
          const rawName = truncate(entry.skill.name, 30);
          const nameStr = willRemove
            ? pc.red(rawName)
            : active
              ? pc.cyan(rawName)
              : pc.dim(rawName);
          const source = `[${entry.tapName}]`;
          const installedTag = isInstalled ? pc.dim(" installed") : "";
          const fixedCols = rawName.length + 2 + source.length + 2 + (isInstalled ? 10 : 0);
          const descSpace = Math.max(0, maxLabelWidth - fixedCols);
          const desc = entry.skill.description
            ? truncate(entry.skill.description, descSpace)
            : "";
          const pad = Math.max(
            1,
            maxLabelWidth - rawName.length - desc.length - source.length - 2 - (isInstalled ? 10 : 0),
          );
          return `${checkbox} ${nameStr}${installedTag}  ${pc.dim(desc)}${" ".repeat(pad)}${pc.dim(source)}`;
        },
      });

      if (isCancel(result)) process.exit(2);
      selected = result as TapEntry[];
    }

    // Compute what to install vs remove
    const selectedNames = new Set(selected.map((e) => e.skill.name));
    const toInstall = selected.filter((e) => !installedNames.has(e.skill.name));
    const toRemove = installedSkills.filter(
      (s) =>
        tapEntries.some((e) => e.skill.name === s.name) &&
        !selectedNames.has(s.name),
    );

    if (toInstall.length === 0 && toRemove.length === 0) {
      process.exit(0);
    }

    intro("skilltap");

    const errors: { name: string; message: string }[] = [];

    // Resolve scope/also only when there are new skills to install
    let scope: "global" | "project" = "global";
    let projectRoot: string | undefined;
    let also: string[] = [];

    if (toInstall.length > 0) {
      ({ scope, projectRoot } = await resolveScope(
        { project: args.project, global: args.global },
        config,
      ));
      also = parseAlsoFlag(args.also, config);

      if (!args.also && !policy.yes && !config.defaults.also.length) {
        const agentSelected = await selectAgents(also);
        if (isCancel(agentSelected)) process.exit(2);
        also = agentSelected as string[];

        if (also.length) {
          const save = await confirmSaveDefault("Save agent selection as default?");
          if (!isCancel(save) && save) {
            config.defaults.also = also;
            await saveConfig(config);
          }
        }
      }
    }

    // Remove deselected skills
    for (const skill of toRemove) {
      const s = spinner();
      s.start(`Removing ${skill.name}…`);
      const result = await removeSkill(skill.name, {
        scope: skill.scope,
        projectRoot: skill.scope === "project" ? detectedProjectRoot : undefined,
      });
      s.stop();
      if (!result.ok) {
        errors.push({ name: skill.name, message: result.error.message });
      } else {
        successLine(`Removed ${skill.name}`);
      }
    }

    for (const entry of toInstall) {
      const skillName = entry.skill.name;
      const s = spinner();
      s.start(`Fetching ${skillName}…`);

      const steps = createStepLogger(config.verbose);
      const { callbacks, logScanResults } = createInstallCallbacks({
        spinner: s,
        onWarn: policy.onWarn,
        skipScan: policy.skipScan,
        agent: undefined,
        yes: policy.yes,
        source: skillName,
        steps,
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
        s.stop();
        errors.push({ name: skillName, message: result.error.message });
      } else {
        s.stop();
        logScanResults();
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
