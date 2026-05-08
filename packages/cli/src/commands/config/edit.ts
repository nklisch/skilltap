import { join } from "node:path";
import { getConfigDir, loadConfig } from "@skilltap/core";
import { defineCommand } from "citty";
import { setupOutput } from "../../ui/setup";

function resolveEditor(): string {
  return process.env.VISUAL || process.env.EDITOR || "nano";
}

export default defineCommand({
  meta: {
    name: "skilltap config edit",
    description: "Open config.toml in your editor",
  },
  async run() {
    const out = setupOutput({ json: false, quiet: false });

    if (!process.stdin.isTTY) {
      out.error("'skilltap config edit' must be run interactively.");
      process.exit(1);
    }

    // Ensure config file exists and capture pre-edit state
    const preResult = await loadConfig();
    if (!preResult.ok) {
      out.error(preResult.error.message, preResult.error.hint);
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
      out.error(
        `Could not launch editor: ${editorStr}`,
        "Set $EDITOR or $VISUAL to your preferred editor.",
      );
      process.exit(1);
    }

    const exitCode = await proc.exited;
    if (exitCode !== 0) {
      out.error(`Editor exited with code ${exitCode} — no changes saved.`);
      process.exit(1);
    }

    const postResult = await loadConfig();
    if (!postResult.ok) {
      await Bun.write(configFile, backup);
      out.error("Config is invalid — reverted to previous version.");
      out.info(`  ${postResult.error.message}`);
      process.exit(1);
    }

    out.success(`Saved ${configFile}`);
  },
});
