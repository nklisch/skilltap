import { findProjectRoot, installMcp } from "@skilltap/core";
import { defineCommand } from "citty";
import { setupOutput } from "../../ui/setup";
import { loadPolicyOrExit } from "../../ui/policy";
import { parseAlsoFlag } from "../../ui/resolve";

export const mcpCommand = defineCommand({
  meta: { name: "mcp", description: "Install a standalone MCP server" },
  args: {
    source: {
      type: "positional",
      description:
        "Source: git URL, github:owner/repo, npm:@scope/pkg, or local path",
      required: true,
    },
    project: {
      type: "boolean",
      description: "Install to project scope",
      default: false,
    },
    global: {
      type: "boolean",
      description: "Install to global scope",
      default: false,
    },
    also: {
      description: "Comma-separated agent dirs to inject into",
      valueHint: "agents",
    },
    yes: {
      type: "boolean",
      alias: "y",
      description: "Auto-accept prompts",
      default: false,
    },
    quiet: {
      type: "boolean",
      description: "Suppress output details",
      default: false,
    },
    json: {
      type: "boolean",
      description: "Output as JSON",
      default: false,
    },
  },
  async run({ args }) {
    const out = setupOutput(args);
    const { config, policy } = await loadPolicyOrExit({
      yes: args.yes,
      project: args.project,
      global: args.global,
    });

    const sources = (args as any)._ as string[];

    for (const source of sources) {
      // Reject the mcp: prefix — it's an internal state convention, not a user input form
      if (source.startsWith("mcp:")) {
        out.error(
          `The 'mcp:' prefix is no longer accepted as user input.`,
          `Just pass the source directly: 'skilltap install mcp ${source.slice(4)}'`,
        );
        process.exit(1);
      }
    }

    const scope = (policy.scope || "project") as "global" | "project";
    const projectRoot =
      scope === "project" ? await findProjectRoot() : undefined;
    const agents = parseAlsoFlag(args.also, config);
    const effectiveAgents = agents.length > 0 ? agents : ["claude-code"];

    for (const source of sources) {
      // installMcp uses the mcp: prefix internally to parse the slug
      // and store state. Prepend it here so the internal convention is preserved.
      const internalSource = `mcp:${source}`;
      const result = await installMcp(internalSource, {
        scope,
        projectRoot,
        agents: effectiveAgents,
        gitHost: config.default_git_host,
      });

      if (!result.ok) {
        out.error(result.error.message, result.error.hint);
        process.exit(1);
      }

      const r = result.value;
      out.success(
        `Installed ${r.records.length} MCP server${r.records.length === 1 ? "" : "s"} from ${source} → ${r.agents.join(", ")}`,
      );
      for (const record of r.records) {
        out.success(`  • ${record.name}`);
      }
    }
  },
});
