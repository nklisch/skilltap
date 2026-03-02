import {
  checkForUpdate,
  downloadAndInstall,
  isCompiledBinary,
  VERSION,
} from "@skilltap/core";
import { defineCommand } from "citty";
import * as p from "@clack/prompts";
import { ansi } from "../ui/format";

export default defineCommand({
  meta: {
    name: "self-update",
    description: "Update skilltap to the latest release",
  },
  args: {
    force: {
      type: "boolean",
      description: "Re-install even if already on the latest version",
      default: false,
    },
  },
  async run({ args }) {
    const force = args.force as boolean;

    p.intro(`${ansi.bold("skilltap self-update")}`);

    const spin = p.spinner();
    spin.start("Checking for latest release…");

    // Force a fresh check by passing interval_hours = 0
    const result = await checkForUpdate(VERSION, 0);

    if (!result && !force) {
      spin.stop(`${ansi.green("✓")} Already on the latest version (v${VERSION})`);
      p.outro("Nothing to do.");
      return;
    }

    const latest = result?.latest ?? VERSION;
    const updateType = result?.type;

    if (!result) {
      spin.stop(`No update found — current version is v${VERSION}`);
      if (!force) {
        p.outro("Nothing to do.");
        return;
      }
    } else {
      spin.stop(
        `Update available: ${ansi.dim(`v${VERSION}`)} → ${ansi.bold(`v${latest}`)} ${ansi.dim(`(${updateType})`)}`,
      );
    }

    if (!isCompiledBinary()) {
      p.note(
        `You appear to be running from source (bun run).\nUpdate via bun: ${ansi.bold("bun update -g skilltap")}\nOr via npm:    ${ansi.bold("npm install -g skilltap")}`,
        "Dev install detected",
      );
      p.outro("Self-update skipped.");
      return;
    }

    const spin2 = p.spinner();
    spin2.start(`Downloading v${latest}…`);

    const installResult = await downloadAndInstall(latest);

    if (!installResult.ok) {
      spin2.stop(ansi.red("Download failed"));
      p.log.error(installResult.error.message);
      if (installResult.error.hint) {
        p.log.info(installResult.error.hint);
      }
      process.exit(1);
    }

    spin2.stop(`${ansi.green("✓")} Updated to v${latest}`);
    p.outro("Changes take effect on the next run.");
  },
});
