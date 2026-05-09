import { findProjectRoot, installMcp } from "@skilltap/core";
import { defineCommand } from "citty";
import { setupOutput } from "../../ui/setup";
import { loadPolicyOrExit } from "../../ui/policy";
import { collectRepeatedFlag, parseAlsoFlag, resolveScope } from "../../ui/resolve";

export const mcpCommand = defineCommand({
  meta: { name: "mcp", description: "Install a standalone MCP server" },
  args: {
    source: {
      type: "positional",
      description:
        "Source: git URL, github:owner/repo, npm:@scope/pkg, or local path",
      required: true,
    },
    scope: {
      type: "string",
      description:
        "Install scope (project | global). Defaults to smart-scope (project inside a git repo, global otherwise).",
      valueHint: "project|global",
    },
    also: {
      type: "string",
      required: false,
      description: "Agent dirs to inject into (repeatable)",
      valueHint: "agent",
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
  async run({ args, rawArgs }) {
    const out = setupOutput(args);

    const scopeArg = args.scope as string | undefined;
    if (
      scopeArg !== undefined &&
      scopeArg !== "project" &&
      scopeArg !== "global"
    ) {
      out.error(
        `Invalid --scope value '${scopeArg}'. Use 'project' or 'global'.`,
      );
      process.exit(1);
    }
    const scopeFlag = scopeArg as "project" | "global" | undefined;

    const { config, policy } = await loadPolicyOrExit({
      yes: args.yes,
      scope: scopeFlag,
    });

    const sources = (args as any)._ as string[];

    for (const source of sources) {
      if (source.startsWith("mcp:")) {
        out.error(
          `The 'mcp:' prefix is no longer accepted as user input.`,
          `Just pass the source directly: 'skilltap install mcp ${source.slice(4)}'`,
        );
        process.exit(1);
      }
    }

    let scope: "global" | "project";
    let projectRoot: string | undefined;
    let inferredScope = false;
    if (policy.scope) {
      scope = policy.scope as "global" | "project";
      if (scope === "project") projectRoot = await findProjectRoot();
    } else {
      const resolved = await resolveScope({}, undefined);
      scope = resolved.scope;
      projectRoot = resolved.projectRoot;
      inferredScope = resolved.inferred ?? false;
    }

    if (inferredScope) {
      out.info(`scope: ${scope} (inferred from cwd)`);
    }

    const repeatedAlso = collectRepeatedFlag(rawArgs, "also");
    const agents = parseAlsoFlag(repeatedAlso, config.defaults.also);
    const effectiveAgents = agents.length > 0 ? agents : ["claude-code"];

    for (const source of sources) {
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
