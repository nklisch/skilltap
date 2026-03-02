import { loadConfig, loadTaps } from "@skilltap/core";
import { defineCommand } from "citty";
import { ansi, errorLine, table } from "../../ui/format";

export default defineCommand({
  meta: {
    name: "list",
    description: "List configured taps",
  },
  async run() {
    const configResult = await loadConfig();
    if (!configResult.ok) {
      errorLine(configResult.error.message, configResult.error.hint);
      process.exit(1);
    }
    const config = configResult.value;

    if (config.taps.length === 0) {
      process.stdout.write(
        `No taps configured. Run 'skilltap tap add <name> <url>' to add one.\n`,
      );
      process.exit(0);
    }

    const tapsResult = await loadTaps();
    if (!tapsResult.ok) {
      errorLine(tapsResult.error.message, tapsResult.error.hint);
      process.exit(1);
    }

    // Count skills per tap
    const counts: Record<string, number> = {};
    for (const entry of tapsResult.value) {
      counts[entry.tapName] = (counts[entry.tapName] ?? 0) + 1;
    }

    const rows = config.taps.map((tap) => [
      ansi.bold(tap.name),
      tap.type === "http" ? ansi.dim("http") : ansi.dim("git"),
      tap.url,
      `${counts[tap.name] ?? 0} skills`,
    ]);

    process.stdout.write("\n");
    process.stdout.write(table(rows));
    process.stdout.write("\n\n");
  },
});
