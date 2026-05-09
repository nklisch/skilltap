import { intro, outro } from "@clack/prompts";
import { addTap, loadConfig, parseGitHubTapShorthand } from "@skilltap/core";
import { defineCommand } from "citty";
import { exitOnError } from "../../ui/exit";
import { setupOutput } from "../../ui/setup";

export default defineCommand({
  meta: {
    name: "skilltap tap add",
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
      description: "Git repository URL",
      required: false,
    },
  },
  async run({ args }) {
    const out = setupOutput({ json: false, quiet: false });
    const configResult = await loadConfig();

    let tapName: string;
    let tapUrl: string;
    const url = args.url as string | undefined;

    if (url) {
      tapName = args.name;
      tapUrl = url;
    } else {
      const shorthand = parseGitHubTapShorthand(
        args.name,
        configResult.ok ? configResult.value.default_git_host : undefined,
      );
      if (!shorthand) {
        out.error(
          `Cannot parse '${args.name}' as GitHub shorthand.`,
          "Use 'skilltap tap add <name> <url>' or 'skilltap tap add owner/repo'.",
        );
        process.exit(1);
      }
      tapName = shorthand.name;
      tapUrl = shorthand.url;
    }

    intro("skilltap");

    const p = out.progress("Adding tap...");

    const result = await addTap(tapName, tapUrl);
    exitOnError(result, out, { onError: () => p.fail("Failed.") });
    p.succeed("Done.");
    const typeLabel = "git";
    out.success(
      `Added tap '${tapName}' (${typeLabel}, ${result.value.skillCount} skills)`,
    );
    outro("Complete!");
  },
});
