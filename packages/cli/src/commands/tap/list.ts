import { BUILTIN_TAP, ensureBuiltinTap, isBuiltinTapCloned, loadConfig, loadTaps } from "@skilltap/core";
import { defineCommand } from "citty";
import { exitWithError } from "../../ui/agent-out";
import { ansi, errorLine, table } from "../../ui/format";

export default defineCommand({
  meta: {
    name: "skilltap tap list",
    description: "List configured taps",
  },
  args: {
    json: {
      type: "boolean",
      description: "Output as JSON",
      default: false,
    },
  },
  async run({ args }) {
    const configResult = await loadConfig();
    if (!configResult.ok) {
      errorLine(configResult.error.message, configResult.error.hint);
      process.exit(1);
    }
    const config = configResult.value;
    const agentMode = config["agent-mode"].enabled;

    const hasBuiltin = config.builtin_tap !== false;

    // Ensure built-in tap is cloned before listing
    if (hasBuiltin) {
      const alreadyCloned = await isBuiltinTapCloned();
      if (!alreadyCloned) {
        await ensureBuiltinTap();
      }
    }

    if (!hasBuiltin && config.taps.length === 0) {
      if (args.json) {
        process.stdout.write("[]\n");
      } else {
        process.stdout.write(
          `No taps configured. Run 'skilltap tap add <name> <url>' to add one.\n`,
        );
      }
      process.exit(0);
    }

    const tapsResult = await loadTaps();
    if (!tapsResult.ok) {
      exitWithError(agentMode, tapsResult.error.message, tapsResult.error.hint);
    }

    // Count skills per tap
    const counts: Record<string, number> = {};
    for (const entry of tapsResult.value) {
      counts[entry.tapName] = (counts[entry.tapName] ?? 0) + 1;
    }

    if (args.json) {
      const tapList = [];
      if (hasBuiltin) {
        tapList.push({
          name: BUILTIN_TAP.name,
          type: "git",
          url: BUILTIN_TAP.url,
          builtin: true,
          skillCount: counts[BUILTIN_TAP.name] ?? 0,
        });
      }
      for (const tap of config.taps) {
        tapList.push({
          name: tap.name,
          type: tap.type,
          url: tap.url,
          builtin: false,
          skillCount: counts[tap.name] ?? 0,
        });
      }
      process.stdout.write(`${JSON.stringify(tapList, null, 2)}\n`);
      return;
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
