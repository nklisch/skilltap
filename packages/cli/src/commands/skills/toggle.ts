import { disableSkill, enableSkill } from "@skilltap/core";
import { defineCommand } from "citty";
import { exitWithError } from "../../ui/agent-out";
import { successLine } from "../../ui/format";
import { loadPolicyOrExit } from "../../ui/policy";
import { tryFindProjectRoot } from "../../ui/resolve";

function makeToggleCommand(action: "enable" | "disable") {
  const coreFn = action === "enable" ? enableSkill : disableSkill;
  const label = action === "enable" ? "Enabled" : "Disabled";
  const description = action === "enable"
    ? "Re-enable a previously disabled skill"
    : "Temporarily disable a skill (hide from agents)";

  return defineCommand({
    meta: { name: action, description },
    args: {
      name: { type: "positional", description: `Skill name to ${action}`, required: true },
      global: { type: "boolean", description: `${label} global skill`, default: false },
      project: { type: "boolean", description: `${label} project skill`, default: false },
    },
    async run({ args }) {
      const { policy } = await loadPolicyOrExit({ project: args.project, global: args.global });
      const scope = args.project ? "project" : args.global ? "global" : undefined;
      const projectRoot = scope === "project" ? await tryFindProjectRoot() : undefined;
      const result = await coreFn(args.name, { scope, projectRoot });
      if (!result.ok) exitWithError(policy.agentMode, result.error.message, result.error.hint);
      if (policy.agentMode) {
        process.stdout.write(`OK: ${label} ${args.name}\n`);
        return;
      }
      successLine(`${label} ${args.name}`);
    },
  });
}

export const enableCommand = makeToggleCommand("enable");
export const disableCommand = makeToggleCommand("disable");
