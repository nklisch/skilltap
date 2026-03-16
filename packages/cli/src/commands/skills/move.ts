import { loadConfig, moveSkill } from "@skilltap/core";
import { defineCommand } from "citty";
import { exitWithError } from "../../ui/agent-out";
import { successLine } from "../../ui/format";
import { parseAlsoFlag, tryFindProjectRoot } from "../../ui/resolve";

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
      exitWithError(agentMode, "Specify target scope: --global or --project");
    }

    if (args.global && args.project) {
      exitWithError(agentMode, "Cannot specify both --global and --project");
    }

    const also = parseAlsoFlag(args.also, configResult.ok ? configResult.value : undefined);

    let to: Parameters<typeof moveSkill>[1]["to"];
    let fromProjectRoot: string | undefined;

    if (args.global) {
      to = { scope: "global" };
      // fromProjectRoot: try to find it for moving from project scope
      fromProjectRoot = await tryFindProjectRoot();
    } else {
      const projectRoot = await tryFindProjectRoot();
      if (!projectRoot) {
        exitWithError(agentMode, "No project root found. Run from inside a project directory.");
      }
      to = { scope: "project", projectRoot };
    }

    const result = await moveSkill(args.name, { to, fromProjectRoot, also });
    if (!result.ok) exitWithError(agentMode, result.error.message, result.error.hint);

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
