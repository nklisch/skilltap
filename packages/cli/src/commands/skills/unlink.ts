import { removeSkill } from "@skilltap/core";
import { defineCommand } from "citty";
import { errorLine, successLine } from "../../ui/format";
import { getInstalledSkillOrExit } from "../../ui/resolve";

export default defineCommand({
  meta: {
    name: "unlink",
    description: "Remove a linked skill",
  },
  args: {
    name: {
      type: "positional",
      description: "Name of linked skill",
      required: true,
    },
  },
  async run({ args }) {
    await getInstalledSkillOrExit(args.name, {
      filter: (s) => s.scope === "linked",
      notFoundMessage: `Skill '${args.name}' is not linked`,
      notFoundHint: "Use 'skilltap remove' for non-linked skills.",
    });

    const result = await removeSkill(args.name, { scope: "linked" });
    if (!result.ok) {
      errorLine(result.error.message, result.error.hint);
      process.exit(1);
    }

    successLine(`Unlinked ${args.name}`);
  },
});
