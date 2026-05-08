import { removeMcpInstall } from "@skilltap/core";
import { defineCommand } from "citty";
import { createOutput } from "../../output";
import { loadPolicyOrExit } from "../../ui/policy";
import { tryFindProjectRoot } from "../../ui/resolve";

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
    project: {
      type: "boolean",
      description: "Remove from project scope",
      default: false,
    },
    global: {
      type: "boolean",
      description: "Remove from global scope",
      default: false,
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
    const out = createOutput({ json: args.json, quiet: false });
    const { policy } = await loadPolicyOrExit({
      yes: args.yes,
      project: args.project,
      global: args.global,
    });

    const sources = ((args._ as string[] | undefined) ?? []).filter(
      (n): n is string => typeof n === "string" && n.length > 0,
    );

    const scope = (policy.scope || "project") as "global" | "project";
    const projectRoot =
      scope === "project" ? await tryFindProjectRoot() : undefined;

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

      const result = await removeMcpInstall(normalizedSource, {
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
