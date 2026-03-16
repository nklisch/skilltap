import { resolve } from "node:path";
import { linkSkill } from "@skilltap/core";
import { defineCommand } from "citty";
import { exitWithError } from "../../ui/agent-out";
import { successLine } from "../../ui/format";
import { isAgentMode } from "../../ui/policy";
import { parseAlsoFlag, resolveScope } from "../../ui/resolve";

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
    const agentMode = await isAgentMode();

    // Resolve the local path (expand ~ and relative paths)
    const rawPath = args.path.replace(/^~/, process.env.HOME ?? "~");
    const localPath = resolve(process.cwd(), rawPath);

    const { scope, projectRoot } = await resolveScope(args);
    const also = parseAlsoFlag(args.also);

    const result = await linkSkill(localPath, { scope, projectRoot, also });
    if (!result.ok) exitWithError(agentMode, result.error.message, result.error.hint);

    const skill = result.value;
    if (agentMode) {
      process.stdout.write(`OK: Linked ${skill.name} → ${skill.path}\n`);
      for (const agent of also) {
        process.stdout.write(`OK: Also linked for ${agent}\n`);
      }
    } else {
      successLine(`Linked ${skill.name} → ${skill.path}`);
      for (const agent of also) {
        successLine(`  Also linked for ${agent}`);
      }
    }
  },
});
