import { intro, isCancel, outro, spinner } from "@clack/prompts";
import type { ScannedSkill, StaticWarning, TapEntry } from "@skilltap/core";
import { installSkill, loadConfig } from "@skilltap/core";
import { defineCommand } from "citty";
import { errorLine, successLine } from "../ui/format";
import { confirmInstall, selectSkills, selectTap } from "../ui/prompts";
import { parseAlsoFlag, resolveScope } from "../ui/resolve";
import { printWarnings } from "../ui/scan";

export default defineCommand({
  meta: {
    name: "install",
    description: "Install a skill from a URL, tap name, or local path",
  },
  args: {
    source: {
      type: "positional",
      description: "Git URL, github:owner/repo, tap skill name, or local path",
      required: true,
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
    ref: {
      description: "Branch or tag to install",
      valueHint: "ref",
    },
    "skip-scan": {
      type: "boolean",
      description: "Skip security scanning",
      default: false,
    },
    yes: {
      type: "boolean",
      alias: "y",
      description: "Auto-accept prompts",
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
    // 1. Load config
    const configResult = await loadConfig();
    if (!configResult.ok) {
      errorLine(configResult.error.message, configResult.error.hint);
      process.exit(1);
    }
    const config = configResult.value;

    // 2. Security policy composition
    if (args["skip-scan"] && config.security.require_scan) {
      errorLine("--skip-scan is blocked by security.require_scan = true");
      process.exit(1);
    }

    let onWarn: "fail" | "prompt";
    if (args.strict) {
      onWarn = "fail";
    } else if (args["no-strict"]) {
      onWarn = "prompt";
    } else {
      onWarn = config.security.on_warn;
    }

    const skipScan = args["skip-scan"];

    const also = parseAlsoFlag(args.also, config);

    intro("skilltap");

    const { scope, projectRoot } = await resolveScope(args, config);

    // 6. Build spinner
    const s = spinner();
    s.start(`Cloning ${args.source}...`);

    const autoSelectAll = args.yes || config.defaults.yes;

    // 7a. onWarnings callback — security intercept
    const warningsCallback = async (
      warnings: StaticWarning[],
      skillName: string,
    ): Promise<boolean> => {
      s.stop();
      printWarnings(warnings, skillName);
      if (onWarn === "fail") {
        errorLine(
          `Security warnings found in ${skillName} — aborting (--strict / on_warn=fail)`,
        );
        process.exit(1);
      }
      const proceed = await confirmInstall(skillName);
      if (isCancel(proceed) || proceed === false) process.exit(2);
      s.start("Installing...");
      return true;
    };

    // 7b. onSelectSkills callback — skill selection intercept
    const selectSkillsCallback = async (
      skills: ScannedSkill[],
    ): Promise<string[]> => {
      if (autoSelectAll || skills.length === 1) {
        if (autoSelectAll && skills.length > 1) {
          s.message(`Auto-selecting all ${skills.length} skills (--yes)`);
        }
        return skills.map((sk) => sk.name);
      }
      s.stop();
      const selected = await selectSkills(skills);
      if (isCancel(selected)) process.exit(2);
      s.start("Installing...");
      return selected as string[];
    };

    // 7c. onSelectTap callback — when multiple taps have the same skill
    const selectTapCallback = async (
      matches: TapEntry[],
    ): Promise<TapEntry | null> => {
      s.stop();
      const chosen = await selectTap(matches);
      if (isCancel(chosen)) process.exit(2);
      s.start("Installing...");
      return chosen as TapEntry;
    };

    // 8. Run install
    const result = await installSkill(args.source, {
      scope,
      projectRoot,
      also,
      ref: args.ref,
      skipScan,
      onWarnings: skipScan ? undefined : warningsCallback,
      onSelectSkills: selectSkillsCallback,
      onSelectTap: selectTapCallback,
    });

    if (!result.ok) {
      s.stop("Failed.", 1);
      errorLine(result.error.message, result.error.hint);
      process.exit(1);
    }

    s.stop("Done.");

    for (const record of result.value.records) {
      successLine(`Installed ${record.name}`);
    }

    outro("Complete!");
  },
});
