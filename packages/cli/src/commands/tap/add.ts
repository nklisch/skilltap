import { intro, outro, spinner } from "@clack/prompts";
import { addTap, loadConfig, parseGitHubTapShorthand } from "@skilltap/core";
import { defineCommand } from "citty";
import { agentError, exitWithError } from "../../ui/agent-out";
import { errorLine, successLine } from "../../ui/format";

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
      description: "URL of the tap (git repo or HTTP registry)",
      required: false,
    },
    type: {
      type: "string",
      description: "Tap type: 'git' or 'http' (auto-detected if omitted)",
    },
  },
  async run({ args }) {
    const configResult = await loadConfig();
    const agentMode = configResult.ok && configResult.value["agent-mode"].enabled;

    const typeOverride = args.type as "git" | "http" | undefined;
    if (typeOverride && typeOverride !== "git" && typeOverride !== "http") {
      exitWithError(agentMode, `Invalid tap type '${typeOverride}'. Must be 'git' or 'http'.`);
    }

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
        exitWithError(
          agentMode,
          `Cannot parse '${args.name}' as GitHub shorthand.`,
          "Use 'skilltap tap add <name> <url>' or 'skilltap tap add owner/repo'.",
        );
      }
      tapName = shorthand.name;
      tapUrl = shorthand.url;
    }

    if (agentMode) {
      const result = await addTap(tapName, tapUrl, typeOverride);
      if (!result.ok) {
        agentError(result.error.message);
        process.exit(1);
      }
      const typeLabel = result.value.type === "http" ? "HTTP registry" : "git";
      process.stdout.write(`OK: Added tap '${tapName}' (${typeLabel}, ${result.value.skillCount} skills)\n`);
      return;
    }

    intro("skilltap");

    const s = spinner();
    s.start("Adding tap...");

    const result = await addTap(tapName, tapUrl, typeOverride);

    if (!result.ok) {
      s.stop("Failed.");
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
