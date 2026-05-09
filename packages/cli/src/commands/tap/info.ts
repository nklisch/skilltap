import { getTapInfo } from "@skilltap/core";
import { defineCommand } from "citty";
import { ansi, table } from "../../ui/format";
import { exitOnError } from "../../ui/exit";
import { setupOutput } from "../../ui/setup";

export default defineCommand({
  meta: {
    name: "skilltap tap info",
    description: "Show details for a configured tap",
  },
  args: {
    name: {
      type: "positional",
      description: "Tap name",
      required: true,
    },
    json: {
      type: "boolean",
      description: "Output as JSON",
      default: false,
    },
  },
  async run({ args }) {
    const out = setupOutput(args);

    const result = await getTapInfo(args.name);
    exitOnError(result, out);
    const info = result.value;

    if (args.json) {
      out.json(info);
      return;
    }

    const rows: string[][] = [
      [ansi.dim("name"), info.name],
      [ansi.dim("type"), info.type],
      [ansi.dim("url"), info.url],
    ];
    if (info.localPath) rows.push([ansi.dim("path"), info.localPath]);
    if (info.lastFetched)
      rows.push([ansi.dim("last fetched"), info.lastFetched]);
    rows.push([ansi.dim("skills"), String(info.skillCount)]);

    out.raw("\n");
    out.raw(table(rows));
    out.raw("\n\n");
  },
});
