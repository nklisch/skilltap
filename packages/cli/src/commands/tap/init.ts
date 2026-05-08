import { initTap } from "@skilltap/core";
import { defineCommand } from "citty";
import { createOutput } from "../../output";

export default defineCommand({
  meta: {
    name: "skilltap tap init",
    description: "Create a new tap repo",
  },
  args: {
    name: {
      type: "positional",
      description: "Directory name for the new tap",
      required: true,
    },
  },
  async run({ args }) {
    const out = createOutput({ json: false, quiet: false });

    const result = await initTap(args.name);
    if (!result.ok) {
      out.error(result.error.message, result.error.hint);
      process.exit(1);
    }

    out.success(`Created ${args.name}/`);
    out.info(`Edit ${args.name}/tap.json to add skills, then push:`);
    out.info(`  cd ${args.name} && git remote add origin <url> && git push`);
    out.info(`Anyone can then add your tap with:`);
    out.info(`  skilltap tap add <name> <url>`);
  },
});
