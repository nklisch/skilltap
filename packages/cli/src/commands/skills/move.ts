import { loadConfig, moveSkill } from "@skilltap/core";
import { defineCommand } from "citty";
import { createOutput } from "../../output";
import { parseAlsoFlag, tryFindProjectRoot } from "../../ui/resolve";

export default defineCommand({
  meta: { name: "move", description: "Move a skill between scopes" },
  args: {
    name: {
      type: "positional",
      description: "Skill name to move",
      required: true,
    },
    global: {
      type: "boolean",
      description: "Move to global scope",
      default: false,
    },
    project: {
      type: "boolean",
      description: "Move to project scope",
      default: false,
    },
    also: {
      description: "Also symlink to agent-specific directory",
      valueHint: "agent",
    },
  },
  async run({ args }) {
    const out = createOutput({ json: false, quiet: false });
    const configResult = await loadConfig();

    if (!args.global && !args.project) {
      out.error("Specify target scope: --global or --project");
      process.exit(1);
    }

    if (args.global && args.project) {
      out.error("Cannot specify both --global and --project");
      process.exit(1);
    }

    const also = parseAlsoFlag(
      args.also,
      configResult.ok ? configResult.value : undefined,
    );

    let to: Parameters<typeof moveSkill>[1]["to"];
    let fromProjectRoot: string | undefined;

    if (args.global) {
      to = { scope: "global" };
      // fromProjectRoot: try to find it for moving from project scope
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
