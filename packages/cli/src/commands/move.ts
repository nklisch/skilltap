import { loadConfig, moveSkill } from "@skilltap/core";
import { defineCommand } from "citty";
import {
  collectRepeatedFlag,
  parseAlsoFlag,
  tryFindProjectRoot,
  validateScopeArg,
} from "../ui/resolve";
import { setupOutput } from "../ui/setup";

export const moveCommand = defineCommand({
  meta: { name: "move", description: "Move a skill between scopes" },
  args: {
    name: {
      type: "positional",
      description: "Skill name to move",
      required: true,
    },
    scope: {
      type: "string",
      description: "Target scope to move into (project | global). Required.",
      valueHint: "project|global",
    },
    also: {
      type: "string",
      required: false,
      description: "Also symlink to agent-specific directory (repeatable)",
      valueHint: "agent",
    },
  },
  async run({ args, rawArgs }) {
    const out = setupOutput({ json: false, quiet: false });
    const configResult = await loadConfig();

    const scope = validateScopeArg(args.scope as string | undefined, out, {
      required: true,
    });

    const repeatedAlso = collectRepeatedFlag(rawArgs, "also");
    const also = parseAlsoFlag(
      repeatedAlso,
      configResult.ok ? configResult.value.defaults.also : [],
    );

    let to: Parameters<typeof moveSkill>[1]["to"];
    let fromProjectRoot: string | undefined;

    if (scope === "global") {
      to = { scope: "global" };
      fromProjectRoot = await tryFindProjectRoot();
    } else {
      const projectRoot = await tryFindProjectRoot();
      if (!projectRoot) {
        out.error(
          "No project root found. Run from inside a project directory.",
        );
        process.exit(1);
      }
      to = { scope: "project", projectRoot };
    }

    const result = await moveSkill(args.name, { to, fromProjectRoot, also });
    if (!result.ok) {
      out.error(result.error.message, result.error.hint);
      process.exit(1);
    }

    const { from, to: destPath } = result.value;
    out.success(`Moved ${args.name}: ${from} → ${destPath}`);
  },
});

export default moveCommand;
