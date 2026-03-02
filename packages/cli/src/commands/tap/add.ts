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
      description: "URL of the tap (git repo or HTTP registry)",
      required: true,
    },
    type: {
      type: "string",
      description: "Tap type: 'git' or 'http' (auto-detected if omitted)",
    },
  },
  async run({ args }) {
    intro("skilltap");

    const typeOverride = args.type as "git" | "http" | undefined;
    if (typeOverride && typeOverride !== "git" && typeOverride !== "http") {
      errorLine(`Invalid tap type '${typeOverride}'. Must be 'git' or 'http'.`);
      process.exit(1);
    }

    const s = spinner();
    s.start("Adding tap...");

    const result = await addTap(args.name, args.url, typeOverride);

    if (!result.ok) {
      s.stop("Failed.", 1);
      errorLine(result.error.message, result.error.hint);
      process.exit(1);
    }

    s.stop("Done.");
    const typeLabel = result.value.type === "http" ? "HTTP registry" : "git";
    successLine(
      `Added tap '${args.name}' (${typeLabel}, ${result.value.skillCount} skills)`,
    );
    outro("Complete!");
  },
});
