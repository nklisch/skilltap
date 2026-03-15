import { enableSkill, findProjectRoot } from "@skilltap/core";
import { defineCommand } from "citty";
import { agentError } from "../../ui/agent-out";
import { errorLine, successLine } from "../../ui/format";
import { loadPolicyOrExit } from "../../ui/policy";

export default defineCommand({
  meta: {
    name: "enable",
    description: "Re-enable a previously disabled skill",
  },
  args: {
    name: {
      type: "positional",
      description: "Skill name to enable",
      required: true,
    },
    global: {
      type: "boolean",
      description: "Enable global skill",
      default: false,
    },
    project: {
      type: "boolean",
      description: "Enable project skill",
      default: false,
    },
  },
  async run({ args }) {
    const { policy } = await loadPolicyOrExit({ project: args.project, global: args.global });

    const scope = args.project ? "project" : args.global ? "global" : undefined;
    const projectRoot = scope === "project" ? await findProjectRoot().catch(() => undefined) : undefined;

    const result = await enableSkill(args.name, { scope, projectRoot });

    if (!result.ok) {
      if (policy.agentMode) {
        agentError(result.error.message);
        process.exit(1);
      }
      errorLine(result.error.message, result.error.hint);
      process.exit(1);
    }

    if (policy.agentMode) {
      process.stdout.write(`OK: Enabled ${args.name}\n`);
      return;
    }

    successLine(`Enabled ${args.name}`);
  },
});
