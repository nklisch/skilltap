import { isCancel, spinner } from "@clack/prompts";
import { loadInstalled, removeSkill } from "@skilltap/core";
import { defineCommand } from "citty";
import { errorLine, successLine } from "../ui/format";
import { confirmRemove } from "../ui/prompts";

export default defineCommand({
  meta: {
    name: "remove",
    description: "Remove an installed skill",
  },
  args: {
    name: {
      type: "positional",
      description: "Name of installed skill",
      required: true,
    },
    project: {
      type: "boolean",
      description: "Remove from project scope instead of global",
      default: false,
    },
    yes: {
      type: "boolean",
      alias: "y",
      description: "Skip confirmation prompt",
      default: false,
    },
  },
  async run({ args }) {
    const installedResult = await loadInstalled();
    if (!installedResult.ok) {
      errorLine(installedResult.error.message);
      process.exit(1);
    }

    const skill = installedResult.value.skills.find(
      (s) => s.name === args.name,
    );
    if (!skill) {
      errorLine(
        `Skill '${args.name}' is not installed`,
        "Run 'skilltap list' to see installed skills.",
      );
      process.exit(1);
    }

    const scope = args.project
      ? "project"
      : (skill.scope as "global" | "project" | "linked");

    if (!args.yes) {
      const confirmed = await confirmRemove(args.name);
      if (isCancel(confirmed) || confirmed === false) {
        process.exit(2);
      }
    }

    const s = spinner();
    s.start(`Removing ${args.name}...`);

    const result = await removeSkill(args.name, { scope });
    if (!result.ok) {
      s.stop("Failed.", 1);
      errorLine(result.error.message, result.error.hint);
      process.exit(1);
    }

    s.stop("Removed.");
    successLine(`Removed ${args.name}`);
  },
});
