import { getTapInfo } from "@skilltap/core";
import { defineCommand } from "citty";
import { exitWithError, outputJson } from "../../ui/agent-out";
import { ansi, table } from "../../ui/format";
import { isAgentMode } from "../../ui/policy";

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
    const agentMode = await isAgentMode();

    const result = await getTapInfo(args.name);
    if (!result.ok) exitWithError(agentMode, result.error.message, result.error.hint);

    const info = result.value;

    if (args.json) {
      outputJson(info);
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
