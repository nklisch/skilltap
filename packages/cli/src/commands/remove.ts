import { isCancel, spinner } from "@clack/prompts";
import { removeSkill } from "@skilltap/core";
import { defineCommand } from "citty";
import { agentError } from "../ui/agent-out";
import { errorLine, successLine } from "../ui/format";
import { loadPolicyOrExit } from "../ui/policy";
import { confirmRemove } from "../ui/prompts";
import { getInstalledSkillOrExit } from "../ui/resolve";
import { sendEvent } from "../telemetry";

export default defineCommand({
  meta: {
    name: "remove",
    description: "Remove an installed skill",
  },
  args: {
    name: {
      type: "positional",
      description: "Name of installed skill",
      required: true,
    },
    project: {
      type: "boolean",
      description: "Remove from project scope instead of global",
      default: false,
    },
    yes: {
      type: "boolean",
      alias: "y",
      description: "Skip confirmation prompt",
      default: false,
    },
  },
  async run({ args }) {
    const { config, policy } = await loadPolicyOrExit({ yes: args.yes, project: args.project });

    const skill = await getInstalledSkillOrExit(args.name, {
      notFoundHint: "Run 'skilltap list' to see installed skills.",
    });

    const scope = args.project
      ? "project"
      : (skill.scope as "global" | "project" | "linked");

    if (policy.agentMode) {
      const result = await removeSkill(args.name, { scope });
      if (!result.ok) {
        sendEvent(config, "remove", {
          os: process.platform,
          arch: process.arch,
          success: false,
          error_category: result.error.constructor.name,
          scope,
          agent_mode: true,
          ci: Boolean(process.env.CI),
        });
        agentError(result.error.message);
        process.exit(1);
      }
      sendEvent(config, "remove", {
        os: process.platform,
        arch: process.arch,
        success: true,
        scope,
        agent_mode: true,
        ci: Boolean(process.env.CI),
      });
      process.stdout.write(`OK: Removed ${args.name}\n`);
      return;
    }

    if (!args.yes) {
      const confirmed = await confirmRemove(args.name);
      if (isCancel(confirmed) || confirmed === false) {
        process.exit(2);
      }
    }

    const s = spinner();
    s.start(`Removing ${args.name}...`);

    const result = await removeSkill(args.name, { scope });
    if (!result.ok) {
      s.stop("Failed.", 1);
      sendEvent(config, "remove", {
        os: process.platform,
        arch: process.arch,
        success: false,
        error_category: result.error.constructor.name,
        scope,
        agent_mode: false,
        ci: Boolean(process.env.CI),
      });
      errorLine(result.error.message, result.error.hint);
      process.exit(1);
    }

    sendEvent(config, "remove", {
      os: process.platform,
      arch: process.arch,
      success: true,
      scope,
      agent_mode: false,
      ci: Boolean(process.env.CI),
    });
    s.stop("Removed.");
    successLine(`Removed ${args.name}`);
  },
});
