import { outro } from "@clack/prompts";
import {
  installSkill,
  updateSkill,
} from "@skilltap/core";
import { defineCommand } from "citty";
import { componentSummary } from "../../ui/plugin-format";
import { inferAdapter, sendEvent, telemetryBase } from "../../telemetry";
import { buildSourceCallbacks, setupInstallContext } from "./shared";
import type { SharedInstallArgs } from "./shared";

export const pluginCommand = defineCommand({
  meta: { name: "plugin", description: "Install a plugin" },
  args: {
    source: {
      type: "positional",
      description:
        "Git URL, github:owner/repo, tap plugin ref (tap/plugin), or local path",
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
      description: "Create symlink in agent-specific directory (repeatable)",
      valueHint: "agent",
    },
    ref: {
      description: "Branch or tag to install",
      valueHint: "ref",
    },
    "skip-scan": {
      type: "boolean",
      description: "Skip security scanning",
      default: false,
    },
    yes: {
      type: "boolean",
      alias: "y",
      description: "Auto-accept prompts",
      default: false,
    },
    strict: {
      type: "boolean",
      description: "Abort on any security warning",
    },
    quiet: {
      type: "boolean",
      description: "Suppress install step details",
    },
    semantic: {
      type: "boolean",
      description: "Force semantic scan",
      default: false,
    },
    json: {
      type: "boolean",
      description: "Output as JSON",
      default: false,
    },
  },
  async run({ args, rawArgs }) {
    await runInstallPlugin(
      args as unknown as SharedInstallArgs & { source: string },
      rawArgs,
    );
  },
});

async function runInstallPlugin(
  args: SharedInstallArgs & { source: string },
  rawArgs: readonly string[],
): Promise<void> {
  const sources = (args as any)._ as string[];

  const ctx = await setupInstallContext(args, rawArgs);
  const { out, config, policy, scope, projectRoot, also, runSemantic, agent } = ctx;

  const errors: { source: string; message: string; hint?: string }[] = [];

  for (const source of sources) {
    if (source.startsWith("mcp:")) {
      out.error(
        `The 'mcp:' prefix is no longer accepted here.`,
        `Use 'skilltap install mcp ${source.slice(4)}' to install a standalone MCP server.`,
      );
      process.exit(1);
    }

    const { progress: p, logScanResults, installOptions } = buildSourceCallbacks(ctx, source);

    // Override onPluginDetected: for `install plugin`, auto-proceed as plugin.
    // If no plugin manifest is detected (onPluginDetected never called), we
    // detect that via the absence of pluginRecord in the result and error.
    let pluginDetected = false;

    const result = await installSkill(source, {
      scope,
      projectRoot,
      also,
      ref: args.ref,
      skipScan: policy.skipScan,
      gitHost: config.default_git_host,
      agent,
      semantic: runSemantic,
      threshold: config.scanner.threshold,
      ...installOptions,
      onPluginDetected: async (_manifest) => {
        pluginDetected = true;
        return "plugin";
      },
    });

    if (!result.ok) {
      p.fail();
      sendEvent(config, "install", {
        ...telemetryBase(),
        adapter: inferAdapter(source),
        success: false,
        error_category: result.error.constructor.name,
        skill_count: 0,
        scan_mode: policy.scanMode,
        scope,
      });
      errors.push({
        source,
        message: result.error.message,
        hint: result.error.hint,
      });
      continue;
    }

    // If no plugin manifest was detected, this is a skill-only repo — error
    if (!pluginDetected && !result.value.pluginRecord) {
      p.fail();
      out.error(
        `No plugin manifest found in '${source}'.`,
        `Use 'skilltap install skill ${source}' to install skills from this repo.`,
      );
      errors.push({
        source,
        message: `No plugin manifest found in '${source}'.`,
        hint: `Use 'skilltap install skill ${source}' for skill-only repos.`,
      });
      continue;
    }

    p.succeed();
    logScanResults();

    sendEvent(config, "install", {
      ...telemetryBase(),
      adapter: inferAdapter(source),
      success: true,
      skill_count: result.value.records.length,
      scan_mode: policy.scanMode,
      scope,
    });

    if (result.value.pluginRecord) {
      const pr = result.value.pluginRecord;
      const summary = componentSummary(pr);
      out.success(`Installed plugin ${pr.name} (${summary})`);
      const cap = result.value.captured;
      if (cap && cap.skills.length + cap.mcpServers.length > 0) {
        out.success(
          `Captured ${cap.skills.length} standalone skill(s), ${cap.mcpServers.length} MCP server(s) into "${pr.name}".`,
        );
        const forced = cap.forcedCrossSource;
        if (forced.skills.length + forced.mcpServers.length > 0) {
          const { log } = await import("@clack/prompts");
          const names = [...forced.skills, ...forced.mcpServers].join(", ");
          log.warn(`  ⚠ Force-captured (cross-source override): ${names}`);
        }
      }
    }

    for (const name of result.value.updates) {
      const updateResult = await updateSkill({
        name,
        yes: policy.yes,
        projectRoot,
        agent,
        semantic: runSemantic,
        threshold: config.scanner.threshold,
      });
      if (!updateResult.ok) {
        out.error(updateResult.error.message, updateResult.error.hint);
      } else {
        const { updated, upToDate } = updateResult.value;
        if (updated.includes(name)) out.success(`Updated ${name}`);
        else if (upToDate.includes(name)) {
          const { log } = await import("@clack/prompts");
          log.info(`${name} is already up to date.`);
        }
      }
    }
  }

  if (errors.length > 0) {
    for (const { source, message, hint } of errors) {
      out.error(`${source}: ${message}`, hint);
    }
    outro("Finished with errors.");
    process.exit(1);
  }

  outro("Complete!");
}
