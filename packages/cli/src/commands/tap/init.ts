import { initTap } from "@skilltap/core";
import { defineCommand } from "citty";
import { errorLine, successLine } from "../../ui/format";

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
    const result = await initTap(args.name);
    if (!result.ok) {
      errorLine(result.error.message, result.error.hint);
      process.exit(1);
    }

    successLine(`Created ${args.name}/`);
    process.stdout.write(`\n`);
    process.stdout.write(
      `Edit ${args.name}/tap.json to add skills, then push:\n`,
    );
    process.stdout.write(
      `  cd ${args.name} && git remote add origin <url> && git push\n`,
    );
    process.stdout.write(`\n`);
    process.stdout.write(`Anyone can then add your tap with:\n`);
    process.stdout.write(`  skilltap tap add <name> <url>\n`);
  },
});
