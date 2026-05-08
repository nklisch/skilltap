import {
  BUILTIN_TAP,
  ensureBuiltinTap,
  isBuiltinTapCloned,
  loadConfig,
  loadTaps,
} from "@skilltap/core";
import { defineCommand } from "citty";
import { ansi, table } from "../../ui/format";
import { createOutput } from "../../output";

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
    const out = createOutput({ json: args.json, quiet: false });

    const configResult = await loadConfig();
    if (!configResult.ok) {
      out.error(configResult.error.message, configResult.error.hint);
      process.exit(1);
    }
    const config = configResult.value;

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
        out.json([]);
      } else {
        out.info(
          `No taps configured. Run 'skilltap tap add <name> <url>' to add one.`,
        );
      }
      process.exit(0);
    }

    const tapsResult = await loadTaps();
    if (!tapsResult.ok) {
      out.error(tapsResult.error.message, tapsResult.error.hint);
      process.exit(1);
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
      out.json(tapList);
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
        ansi.dim("git"),
        tap.url,
        `${counts[tap.name] ?? 0} skills`,
      ]);
    }

    out.raw("\n");
    out.raw(table(rows));
    out.raw("\n\n");
  },
});
