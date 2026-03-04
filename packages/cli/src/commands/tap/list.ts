import { BUILTIN_TAP, loadConfig, loadTaps } from "@skilltap/core";
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

    const hasBuiltin = config.builtin_tap !== false;
    if (!hasBuiltin && config.taps.length === 0) {
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

    const rows: string[][] = [];

    if (hasBuiltin) {
      rows.push([
        ansi.bold(BUILTIN_TAP.name) + ansi.dim(" (built-in)"),
        ansi.dim("git"),
        BUILTIN_TAP.url,
        `${counts[BUILTIN_TAP.name] ?? 0} skills`,
      ]);
    }

    for (const tap of config.taps) {
      rows.push([
        ansi.bold(tap.name),
        tap.type === "http" ? ansi.dim("http") : ansi.dim("git"),
        tap.url,
        `${counts[tap.name] ?? 0} skills`,
      ]);
    }

    process.stdout.write("\n");
    process.stdout.write(table(rows));
    process.stdout.write("\n\n");
  },
});
