import { resolve } from "node:path";
import { linkSkill, loadConfig } from "@skilltap/core";
import { defineCommand } from "citty";
import { createOutput } from "../../output";
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
    const out = createOutput({ json: false, quiet: false });
    const configResult = await loadConfig();
    const config = configResult.ok ? configResult.value : undefined;

    // Resolve the local path (expand ~ and relative paths)
    const rawPath = args.path.replace(/^~/, process.env.HOME ?? "~");
    const localPath = resolve(process.cwd(), rawPath);

    const { scope, projectRoot } = await resolveScope(args);
    const also = parseAlsoFlag(args.also, config);

    const result = await linkSkill(localPath, { scope, projectRoot, also });
    if (!result.ok) {
      out.error(result.error.message, result.error.hint);
      process.exit(1);
    }

    const skill = result.value;
    out.success(`Linked ${skill.name} → ${skill.path}`);
    for (const agent of also) {
      out.success(`  Also linked for ${agent}`);
    }
  },
});
