import { defineCommand } from "citty";
import { loadInstalled, removeSkill } from "@skilltap/core";
import { errorLine, successLine } from "../ui/format";

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
    const installedResult = await loadInstalled();
    if (!installedResult.ok) {
      errorLine(installedResult.error.message);
      process.exit(1);
    }

    const skill = installedResult.value.skills.find(
      (s) => s.name === args.name && s.scope === "linked",
    );

    if (!skill) {
      errorLine(
        `Skill '${args.name}' is not linked`,
        "Use 'skilltap remove' for non-linked skills.",
      );
      process.exit(1);
    }

    const result = await removeSkill(args.name, { scope: "linked" });
    if (!result.ok) {
      errorLine(result.error.message, result.error.hint);
      process.exit(1);
    }

    successLine(`Unlinked ${args.name}`);
  },
});
