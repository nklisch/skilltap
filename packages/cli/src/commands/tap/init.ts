import { initTap } from "@skilltap/core";
import { defineCommand } from "citty";
import { exitOnError } from "../../ui/exit";
import { setupOutput } from "../../ui/setup";

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
    const out = setupOutput({ json: false, quiet: false });

    const result = await initTap(args.name);
    exitOnError(result, out);
    out.success(`Created ${args.name}/`);
    out.info(`Edit ${args.name}/tap.json to add skills, then push:`);
    out.info(`  cd ${args.name} && git remote add origin <url> && git push`);
    out.info(`Anyone can then add your tap with:`);
    out.info(`  skilltap tap add <name> <url>`);
  },
});
