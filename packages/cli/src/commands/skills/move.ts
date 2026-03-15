import { findProjectRoot, loadConfig, moveSkill } from "@skilltap/core";
import { defineCommand } from "citty";
import { agentError } from "../../ui/agent-out";
import { errorLine, successLine } from "../../ui/format";
import { parseAlsoFlag } from "../../ui/resolve";

export default defineCommand({
  meta: { name: "move", description: "Move a skill between scopes" },
  args: {
    name: { type: "positional", description: "Skill name to move", required: true },
    global: { type: "boolean", description: "Move to global scope", default: false },
    project: { type: "boolean", description: "Move to project scope", default: false },
    also: { description: "Also symlink to agent-specific directory", valueHint: "agent" },
  },
  async run({ args }) {
    const configResult = await loadConfig();
    const agentMode = configResult.ok && configResult.value["agent-mode"].enabled;

    if (!args.global && !args.project) {
      const msg = "Specify target scope: --global or --project";
      if (agentMode) agentError(msg);
      else errorLine(msg);
      process.exit(1);
    }

    if (args.global && args.project) {
      const msg = "Cannot specify both --global and --project";
      if (agentMode) agentError(msg);
      else errorLine(msg);
      process.exit(1);
    }

    const also = parseAlsoFlag(args.also, configResult.ok ? configResult.value : undefined);

    let to: Parameters<typeof moveSkill>[1]["to"];
    let fromProjectRoot: string | undefined;

    if (args.global) {
      to = { scope: "global" };
      // fromProjectRoot: try to find it for moving from project scope
      fromProjectRoot = await findProjectRoot().catch(() => undefined);
    } else {
      const projectRoot = await findProjectRoot().catch(() => undefined);
      if (!projectRoot) {
        const msg = "No project root found. Run from inside a project directory.";
        if (agentMode) agentError(msg);
        else errorLine(msg);
        process.exit(1);
      }
      to = { scope: "project", projectRoot };
    }

    const result = await moveSkill(args.name, { to, fromProjectRoot, also });
    if (!result.ok) {
      if (agentMode) agentError(result.error.message);
      else errorLine(result.error.message, result.error.hint);
      process.exit(1);
    }

    const { from, to: destPath, record } = result.value;
    const fromScope = from.includes("/.agents/skills/") ? "global" : "project";
    const toScope = record.scope;

    if (agentMode) {
      process.stdout.write(`OK: Moved ${args.name} from ${fromScope} to ${toScope}\n`);
    } else {
      successLine(`Moved ${args.name}: ${from} → ${destPath}`);
    }
  },
});
