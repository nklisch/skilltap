import { join } from "node:path";
import { getConfigDir, loadConfig } from "@skilltap/core";
import { defineCommand } from "citty";
import { errorLine, successLine } from "../../ui/format";

function resolveEditor(): string {
  return process.env.VISUAL || process.env.EDITOR || "nano";
}

export default defineCommand({
  meta: {
    name: "skilltap config edit",
    description: "Open config.toml in your editor",
  },
  async run() {
    if (!process.stdin.isTTY) {
      errorLine("'skilltap config edit' must be run interactively.");
      process.exit(1);
    }

    // Ensure config file exists and capture pre-edit state
    const preResult = await loadConfig();
    if (!preResult.ok) {
      errorLine(preResult.error.message, preResult.error.hint);
      process.exit(1);
    }

    const configFile = join(getConfigDir(), "config.toml");
    const backup = await Bun.file(configFile).text();

    const editorStr = resolveEditor();
    const editorArgs = editorStr.split(" ");

    let proc: ReturnType<typeof Bun.spawn>;
    try {
      proc = Bun.spawn([...editorArgs, configFile], {
        stdin: "inherit",
        stdout: "inherit",
        stderr: "inherit",
      });
    } catch {
      errorLine(
        `Could not launch editor: ${editorStr}`,
        "Set $EDITOR or $VISUAL to your preferred editor.",
      );
      process.exit(1);
    }

    const exitCode = await proc.exited;
    if (exitCode !== 0) {
      errorLine(`Editor exited with code ${exitCode} — no changes saved.`);
      process.exit(1);
    }

    const postResult = await loadConfig();
    if (!postResult.ok) {
      await Bun.write(configFile, backup);
      errorLine("Config is invalid — reverted to previous version.");
      process.stderr.write(`  ${postResult.error.message}\n`);
      process.exit(1);
    }

    successLine(`Saved ${configFile}`);
  },
});
