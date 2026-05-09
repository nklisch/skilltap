import { removeMcp } from "@skilltap/core";
import { defineCommand } from "citty";
import { setupRemoveContext } from "./shared";

export const mcpRemoveCommand = defineCommand({
  meta: {
    name: "mcp",
    description: "Remove a standalone MCP server",
  },
  args: {
    name: {
      type: "positional",
      description:
        "Source of the MCP server to remove (the source passed to install)",
      required: true,
    },
    scope: {
      type: "string",
      description:
        "Install scope to remove from (project | global). Defaults to smart-scope (project inside a git repo, global otherwise).",
      valueHint: "project|global",
    },
    yes: {
      type: "boolean",
      alias: "y",
      description: "Skip confirmation prompt",
      default: false,
    },
    json: {
      type: "boolean",
      description: "Output as JSON",
      default: false,
    },
  },
  async run({ args }) {
    const ctx = await setupRemoveContext(args);
    const { out, scope, projectRoot } = ctx;

    const sources = ((args._ as string[] | undefined) ?? []).filter(
      (n): n is string => typeof n === "string" && n.length > 0,
    );

    let anyFail = false;
    for (const source of sources) {
      if (source.startsWith("mcp:")) {
        out.error(
          `The 'mcp:' prefix is no longer accepted as user input.`,
          `Just pass the source directly: 'skilltap remove mcp ${source.slice(4)}'`,
        );
        process.exit(1);
      }
      // Prepend mcp: prefix for internal state lookup — state stores sources with this prefix
      const normalizedSource = `mcp:${source}`;

      const result = await removeMcp(normalizedSource, {
        scope,
        projectRoot,
      });
      if (!result.ok) {
        out.error(result.error.message, result.error.hint);
        anyFail = true;
        continue;
      }
      const r = result.value;
      out.success(
        `Removed ${r.removed} MCP server${r.removed === 1 ? "" : "s"} from ${source} (agents: ${r.agents.join(", ")})`,
      );
      for (const name of r.names) {
        out.success(`  • ${name}`);
      }
    }
    if (anyFail) process.exit(1);
  },
});
