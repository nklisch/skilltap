import { cancel, isCancel } from "@clack/prompts";
import { footerConfirm as confirm } from "../../ui/footer";
import { removeTap } from "@skilltap/core";
import { defineCommand } from "citty";
import { errorLine, successLine } from "../../ui/format";

export default defineCommand({
  meta: {
    name: "skilltap tap remove",
    description: "Remove a tap",
  },
  args: {
    name: {
      type: "positional",
      description: "Tap name to remove",
      required: true,
    },
    yes: {
      type: "boolean",
      alias: "y",
      description: "Skip confirmation prompt",
      default: false,
    },
  },
  async run({ args }) {
    if (!args.yes) {
      const confirmed = await confirm({
        message: `Remove tap '${args.name}'? Installed skills from this tap will not be affected.`,
        initialValue: false,
      });
      if (isCancel(confirmed)) {
        cancel("Operation cancelled.");
        process.exit(130);
      }
      if (!confirmed) {
        process.exit(130);
      }
    }

    const result = await removeTap(args.name);
    if (!result.ok) {
      errorLine(result.error.message, result.error.hint);
      process.exit(1);
    }

    successLine(`Removed tap '${args.name}'`);
  },
});
