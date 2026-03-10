import { getTapInfo, loadConfig } from "@skilltap/core";
import { defineCommand } from "citty";
import { agentError } from "../../ui/agent-out";
import { ansi, errorLine, table } from "../../ui/format";

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
    const configResult = await loadConfig();
    const agentMode = configResult.ok && configResult.value["agent-mode"].enabled;

    const result = await getTapInfo(args.name);
    if (!result.ok) {
      if (agentMode) {
        agentError(result.error.message);
        process.exit(1);
      }
      errorLine(result.error.message, result.error.hint);
      process.exit(1);
    }

    const info = result.value;

    if (args.json) {
      process.stdout.write(`${JSON.stringify(info, null, 2)}\n`);
      return;
    }

    if (agentMode) {
      process.stdout.write(`name: ${info.name}\n`);
      process.stdout.write(`type: ${info.type}\n`);
      process.stdout.write(`url: ${info.url}\n`);
      if (info.localPath) process.stdout.write(`path: ${info.localPath}\n`);
      if (info.lastFetched) process.stdout.write(`last-fetched: ${info.lastFetched}\n`);
      process.stdout.write(`skills: ${info.skillCount}\n`);
      return;
    }

    const rows: string[][] = [
      [ansi.dim("name"), info.name],
      [ansi.dim("type"), info.type],
      [ansi.dim("url"), info.url],
    ];
    if (info.localPath) rows.push([ansi.dim("path"), info.localPath]);
    if (info.lastFetched) rows.push([ansi.dim("last fetched"), info.lastFetched]);
    rows.push([ansi.dim("skills"), String(info.skillCount)]);

    process.stdout.write("\n");
    process.stdout.write(table(rows));
    process.stdout.write("\n\n");
  },
});
