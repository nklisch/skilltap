import { intro, outro, spinner } from "@clack/prompts";
import { addTap, parseGitHubTapShorthand } from "@skilltap/core";
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
      description: "Tap name or GitHub shorthand (owner/repo)",
      required: true,
    },
    url: {
      type: "positional",
      description: "URL of the tap (git repo or HTTP registry)",
      required: false,
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

    let tapName: string;
    let tapUrl: string;
    const url = args.url as string | undefined;

    if (url) {
      tapName = args.name;
      tapUrl = url;
    } else {
      const shorthand = parseGitHubTapShorthand(args.name);
      if (!shorthand) {
        errorLine(
          `Cannot parse '${args.name}' as GitHub shorthand.`,
          "Use 'skilltap tap add <name> <url>' or 'skilltap tap add owner/repo'.",
        );
        process.exit(1);
      }
      tapName = shorthand.name;
      tapUrl = shorthand.url;
    }

    const s = spinner();
    s.start("Adding tap...");

    const result = await addTap(tapName, tapUrl, typeOverride);

    if (!result.ok) {
      s.stop("Failed.", 1);
      errorLine(result.error.message, result.error.hint);
      process.exit(1);
    }

    s.stop("Done.");
    const typeLabel = result.value.type === "http" ? "HTTP registry" : "git";
    successLine(
      `Added tap '${tapName}' (${typeLabel}, ${result.value.skillCount} skills)`,
    );
    outro("Complete!");
  },
});
