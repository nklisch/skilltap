import { cancel, isCancel } from "@clack/prompts";
import { removeTap } from "@skilltap/core";
import { defineCommand } from "citty";
import { footerConfirm as confirm } from "../../ui/footer";
import { exitOnError } from "../../ui/exit";
import { setupOutput } from "../../ui/setup";

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
    const out = setupOutput({ json: false, quiet: false });

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
    exitOnError(result, out);
    out.success(`Removed tap '${args.name}'`);
  },
});
