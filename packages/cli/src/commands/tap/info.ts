import { getTapInfo } from "@skilltap/core";
import { defineCommand } from "citty";
import { ansi, errorLine, jsonLine, table } from "../../ui/format";

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
    const result = await getTapInfo(args.name);
    if (!result.ok) {
      errorLine(result.error.message, result.error.hint);
      process.exit(1);
    }

    const info = result.value;

    if (args.json) {
      jsonLine(info);
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

    process.stdout.write("\n");
    process.stdout.write(table(rows));
    process.stdout.write("\n\n");
  },
});
