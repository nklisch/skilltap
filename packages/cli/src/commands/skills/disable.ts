import { disableSkill, findProjectRoot } from "@skilltap/core";
import { defineCommand } from "citty";
import { agentError } from "../../ui/agent-out";
import { errorLine, successLine } from "../../ui/format";
import { loadPolicyOrExit } from "../../ui/policy";

export default defineCommand({
  meta: {
    name: "disable",
    description: "Temporarily disable a skill (hide from agents)",
  },
  args: {
    name: {
      type: "positional",
      description: "Skill name to disable",
      required: true,
    },
    global: {
      type: "boolean",
      description: "Disable global skill",
      default: false,
    },
    project: {
      type: "boolean",
      description: "Disable project skill",
      default: false,
    },
  },
  async run({ args }) {
    const { policy } = await loadPolicyOrExit({ project: args.project, global: args.global });

    const scope = args.project ? "project" : args.global ? "global" : undefined;
    const projectRoot = scope === "project" ? await findProjectRoot().catch(() => undefined) : undefined;

    const result = await disableSkill(args.name, { scope, projectRoot });

    if (!result.ok) {
      if (policy.agentMode) {
        agentError(result.error.message);
        process.exit(1);
      }
      errorLine(result.error.message, result.error.hint);
      process.exit(1);
    }

    if (policy.agentMode) {
      process.stdout.write(`OK: Disabled ${args.name}\n`);
      return;
    }

    successLine(`Disabled ${args.name}`);
  },
});
