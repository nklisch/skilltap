import * as p from "@clack/prompts";
import {
  checkForUpdate,
  downloadAndInstall,
  fetchLatestVersion,
  isCompiledBinary,
  VERSION,
} from "@skilltap/core";
import { defineCommand } from "citty";
import { ansi } from "../ui/format";
import { setupOutput } from "../ui/setup";

export default defineCommand({
  meta: {
    name: "self-update",
    description: "Update skilltap to the latest release",
  },
  args: {
    force: {
      type: "boolean",
      description:
        "Bypass cache and re-install even if already on the latest version",
      default: false,
    },
  },
  async run({ args }) {
    const out = setupOutput({ json: false, quiet: false });
    const force = args.force as boolean;

    p.intro(`${ansi.bold("skilltap self-update")}`);

    const checkProgress = out.progress("Checking for latest release…");

    let latest: string;
    let updateType: string | undefined;

    if (force) {
      // Bypass cache entirely — fetch from GitHub directly
      const fetched = await fetchLatestVersion();
      latest = fetched ?? VERSION;
      updateType = undefined;
    } else {
      // Normal path: check cache, background refresh
      const result = await checkForUpdate(VERSION, 0);

      if (!result) {
        checkProgress.succeed(
          `${ansi.green("✓")} Already on the latest version (v${VERSION})`,
        );
        p.outro("Nothing to do.");
        return;
      }

      latest = result.latest;
      updateType = result.type;
    }

    if (latest === VERSION && !force) {
      checkProgress.succeed(
        `${ansi.green("✓")} Already on the latest version (v${VERSION})`,
      );
      p.outro("Nothing to do.");
      return;
    }

    if (latest !== VERSION) {
      checkProgress.succeed(
        `Update available: ${ansi.dim(`v${VERSION}`)} → ${ansi.bold(`v${latest}`)}${updateType ? ` ${ansi.dim(`(${updateType})`)}` : ""}`,
      );
    } else {
      checkProgress.succeed(`Already on v${VERSION} — reinstalling`);
    }

    if (!isCompiledBinary()) {
      p.note(
        `You appear to be running from source (bun run).\nUpdate via bun: ${ansi.bold("bun update -g skilltap")}\nOr via npm:    ${ansi.bold("npm install -g skilltap")}`,
        "Dev install detected",
      );
      p.outro("Self-update skipped.");
      return;
    }

    const downloadProgress = out.progress(`Downloading v${latest}…`);

    const installResult = await downloadAndInstall(latest);

    if (!installResult.ok) {
      downloadProgress.fail(ansi.red("Download failed"));
      out.error(installResult.error.message, installResult.error.hint);
      process.exit(1);
    }

    downloadProgress.succeed(`${ansi.green("✓")} Updated to v${latest}`);
    p.outro("Changes take effect on the next run.");
  },
});
