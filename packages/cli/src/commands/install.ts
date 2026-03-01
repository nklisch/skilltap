import { defineCommand } from "citty";
import { intro, isCancel, outro, spinner } from "@clack/prompts";
import type { ScannedSkill, StaticWarning } from "@skilltap/core";
import {
  findProjectRoot,
  installSkill,
  loadConfig,
  VALID_AGENT_IDS,
} from "@skilltap/core";
import { errorLine, successLine } from "../ui/format";
import { confirmInstall, selectScope, selectSkills } from "../ui/prompts";
import { printWarnings } from "../ui/scan";

export default defineCommand({
  meta: {
    name: "install",
    description: "Install a skill from a URL, tap name, or local path",
  },
  args: {
    source: {
      type: "positional",
      description:
        "Git URL, github:owner/repo, tap skill name, or local path",
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
      errorLine(
        "--skip-scan is blocked by security.require_scan = true",
      );
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

    // 3. Parse --also (CLI flag takes precedence over config default)
    const also: string[] = [];
    if (args.also) {
      const agents = args.also
        .split(",")
        .map((a: string) => a.trim())
        .filter(Boolean);
      for (const agent of agents) {
        if (!VALID_AGENT_IDS.includes(agent)) {
          errorLine(
            `Unknown agent: "${agent}"`,
            `Valid agents: ${VALID_AGENT_IDS.join(", ")}`,
          );
          process.exit(1);
        }
        also.push(agent);
      }
    } else {
      also.push(...config.defaults.also);
    }

    // 4. intro
    intro("skilltap");

    // 5. Scope resolution
    let scope: "global" | "project";
    let projectRoot: string | undefined;

    if (args.project) {
      scope = "project";
      projectRoot = await findProjectRoot();
    } else if (args.global) {
      scope = "global";
    } else if (config.defaults.scope) {
      scope = config.defaults.scope as "global" | "project";
      if (scope === "project") {
        projectRoot = await findProjectRoot();
      }
    } else {
      const chosen = await selectScope();
      if (isCancel(chosen)) process.exit(2);
      scope = chosen as "global" | "project";
      if (scope === "project") {
        projectRoot = await findProjectRoot();
      }
    }

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

    // 8. Run install
    const result = await installSkill(args.source, {
      scope,
      projectRoot,
      also,
      ref: args.ref,
      skipScan,
      onWarnings: skipScan ? undefined : warningsCallback,
      onSelectSkills: selectSkillsCallback,
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
