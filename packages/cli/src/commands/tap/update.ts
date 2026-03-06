import { intro, outro, spinner } from "@clack/prompts";
import { updateTap } from "@skilltap/core";
import { defineCommand } from "citty";
import { ansi, errorLine, successLine } from "../../ui/format";

export default defineCommand({
  meta: {
    name: "skilltap tap update",
    description: "Update tap repo(s)",
  },
  args: {
    name: {
      type: "positional",
      description: "Specific tap to update (omit to update all)",
    },
  },
  async run({ args }) {
    intro("skilltap");

    const s = spinner();
    const tapName = args.name as string | undefined;
    s.start(tapName ? `Updating ${tapName}...` : "Updating all taps...");

    const result = await updateTap(tapName);

    if (!result.ok) {
      s.stop("Failed.");
      errorLine(result.error.message, result.error.hint);
      process.exit(1);
    }

    s.stop("Done.");

    for (const [name, count] of Object.entries(result.value.updated)) {
      successLine(`${ansi.bold(name)} — ${count} skills`);
    }
    for (const name of result.value.http) {
      successLine(`${ansi.bold(name)} — HTTP registry (always up to date)`);
    }

    outro("Complete!");
  },
});
