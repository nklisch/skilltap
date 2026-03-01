import { intro, outro, spinner } from "@clack/prompts";
import { addTap } from "@skilltap/core";
import { defineCommand } from "citty";
import { errorLine, successLine } from "../../ui/format";

export default defineCommand({
  meta: {
    name: "add",
    description: "Add a tap",
  },
  args: {
    name: {
      type: "positional",
      description: "Local name for this tap",
      required: true,
    },
    url: {
      type: "positional",
      description: "Git URL of the tap repo",
      required: true,
    },
  },
  async run({ args }) {
    intro("skilltap");

    const s = spinner();
    s.start("Cloning tap...");

    const result = await addTap(args.name, args.url);

    if (!result.ok) {
      s.stop("Failed.", 1);
      errorLine(result.error.message, result.error.hint);
      process.exit(1);
    }

    s.stop("Done.");
    successLine(`Added tap '${args.name}' (${result.value.skillCount} skills)`);
    outro("Complete!");
  },
});
