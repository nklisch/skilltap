import { defineCommand } from "citty";
import { resolve } from "node:path";
import { isCancel } from "@clack/prompts";
import {
  findProjectRoot,
  linkSkill,
  VALID_AGENT_IDS,
} from "@skilltap/core";
import { errorLine, successLine } from "../ui/format";
import { selectScope } from "../ui/prompts";

export default defineCommand({
  meta: {
    name: "link",
    description: "Symlink a local skill directory into the install path",
  },
  args: {
    path: {
      type: "positional",
      description: "Path to local skill directory (must contain SKILL.md)",
      required: true,
    },
    project: {
      type: "boolean",
      description: "Link to project scope instead of global",
      default: false,
    },
    global: {
      type: "boolean",
      description: "Link to global scope (~/.agents/skills/)",
      default: false,
    },
    also: {
      description: "Also symlink to agent-specific directory",
      valueHint: "agent",
    },
  },
  async run({ args }) {
    // Resolve the local path (expand ~ and relative paths)
    const rawPath = args.path.replace(/^~/, process.env.HOME ?? "~");
    const localPath = resolve(process.cwd(), rawPath);

    // Determine scope
    let scope: "global" | "project";
    let projectRoot: string | undefined;

    if (args.project) {
      scope = "project";
      projectRoot = await findProjectRoot();
    } else if (args.global) {
      scope = "global";
    } else {
      const chosen = await selectScope();
      if (isCancel(chosen)) process.exit(2);
      scope = chosen as "global" | "project";
      if (scope === "project") {
        projectRoot = await findProjectRoot();
      }
    }

    // Parse --also
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
    }

    const result = await linkSkill(localPath, { scope, projectRoot, also });
    if (!result.ok) {
      errorLine(result.error.message, result.error.hint);
      process.exit(1);
    }

    const skill = result.value;
    successLine(`Linked ${skill.name} → ${skill.path}`);
    for (const agent of also) {
      successLine(`  Also linked for ${agent}`);
    }
  },
});
