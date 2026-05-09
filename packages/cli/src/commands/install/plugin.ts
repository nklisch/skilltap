import { outro } from "@clack/prompts";
import { discoverPluginsAt, installSkill, updateSkill } from "@skilltap/core";
import { defineCommand } from "citty";
import { inferAdapter, sendEvent, telemetryBase } from "../../telemetry";
import { componentSummary } from "../../ui/plugin-format";
import { hasRawFlag } from "../../ui/resolve";
import type { CaptureMode, InstallContext, SharedInstallArgs } from "./shared";
import { buildSourceCallbacks, setupInstallContext } from "./shared";

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
    "force-capture": {
      type: "boolean",
      description:
        "Capture an existing standalone install of this plugin into the new source (non-interactive).",
    },
    // NOTE: citty intercepts `--no-capture` as a negation of `--capture` and
    // sets `args.capture = false`. The CLI reads this flag from rawArgs via
    // `hasRawFlag(rawArgs, "no-capture")` instead of the parsed args object.
    "no-capture": {
      type: "boolean",
      description:
        "Skip capture even if the same plugin name exists from another source; install side-by-side.",
    },
  },
  async run({ args, rawArgs }) {
    await runInstallPlugin(
      args as unknown as SharedInstallArgs & { source: string },
      rawArgs,
    );
  },
});

type SourceError = { source: string; message: string; hint?: string };

// Strip the trailing :<plugin-name> or :* from a source string, returning the
// stripped source and the selector. Mirrors the parsing in the github / git
// adapters but operates on the raw string the user supplied — so the CLI can
// rewrite `acme/tools:*` into `acme/tools:auth`, `acme/tools:billing`, ... and
// hand each rewritten source to installSkill.
function splitPluginSelector(source: string): {
  base: string;
  selector?: string;
} {
  // Pull off `@ref` first to avoid swallowing it.
  let s = source;
  let refSuffix = "";
  const atIdx = s.lastIndexOf("@");
  if (atIdx > 4 && !s.slice(atIdx).includes("/")) {
    refSuffix = s.slice(atIdx);
    s = s.slice(0, atIdx);
  }
  const colonIdx = s.lastIndexOf(":");
  if (colonIdx <= 0) return { base: source };
  const tail = s.slice(colonIdx + 1);
  if (tail.length === 0 || tail.includes("/")) return { base: source };
  // Never strip URL-prefix colons (https://, ssh://) — those are followed by `/`.
  return { base: s.slice(0, colonIdx) + refSuffix, selector: tail };
}

async function runInstallPlugin(
  args: SharedInstallArgs & { source: string },
  rawArgs: readonly string[],
): Promise<void> {
  const sources = (args as unknown as { _: string[] })._;

  const ctx = await setupInstallContext(args, rawArgs);
  const { out } = ctx;

  const force = !!(args as { "force-capture"?: boolean })["force-capture"];
  const skip = hasRawFlag(rawArgs, "no-capture");
  if (force && skip) {
    out.error(
      "Cannot use --force-capture and --no-capture together.",
      "Pick one: --force-capture to capture standalones, --no-capture to install side-by-side.",
    );
    process.exit(1);
  }
  const captureMode: CaptureMode = force ? "force" : skip ? "skip" : "prompt";

  const errors: SourceError[] = [];

  for (const source of sources) {
    if (source.startsWith("mcp:")) {
      out.error(
        `The 'mcp:' prefix is no longer accepted here.`,
        `Use 'skilltap install mcp ${source.slice(4)}' to install a standalone MCP server.`,
      );
      process.exit(1);
    }

    const { selector, base } = splitPluginSelector(source);
    if (selector === "*") {
      const expandErrors = await runMultiPluginInstall(
        ctx,
        args,
        base,
        source,
        captureMode,
      );
      errors.push(...expandErrors);
      continue;
    }

    const sourceErrors = await runSinglePluginInstall(
      ctx,
      args,
      source,
      selector,
      captureMode,
    );
    errors.push(...sourceErrors);
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

async function runMultiPluginInstall(
  ctx: InstallContext,
  args: SharedInstallArgs & { source: string },
  base: string,
  originalSource: string,
  captureMode: CaptureMode,
): Promise<SourceError[]> {
  const { out, config } = ctx;
  const errors: SourceError[] = [];

  const discovered = await discoverPluginsAt(base, {
    gitHost: config.default_git_host,
  });
  if (!discovered.ok) {
    return [
      {
        source: originalSource,
        message: discovered.error.message,
        hint: discovered.error.hint,
      },
    ];
  }
  const { manifests, cleanup } = discovered.value;
  try {
    if (manifests.length === 0) {
      return [
        {
          source: originalSource,
          message: `No plugin manifest found in '${originalSource}'.`,
          hint: `Use 'skilltap install skill ${base}' for skill-only repos.`,
        },
      ];
    }
    out.info(
      `Discovered ${manifests.length} plugin(s) in '${base}': ${manifests
        .map((m) => m.name)
        .join(", ")}`,
    );
    for (const manifest of manifests) {
      // Rewrite source so the per-plugin install line shows the actual name.
      const perPluginSource = `${base}:${manifest.name}`;
      const sourceErrors = await runSinglePluginInstall(
        ctx,
        args,
        perPluginSource,
        manifest.name,
        captureMode,
      );
      errors.push(...sourceErrors);
    }
  } finally {
    await cleanup();
  }
  return errors;
}

async function runSinglePluginInstall(
  ctx: InstallContext,
  args: SharedInstallArgs & { source: string },
  source: string,
  selectName: string | undefined,
  captureMode: CaptureMode,
): Promise<SourceError[]> {
  const { out, config, policy, scope, projectRoot, also, runSemantic, agent } =
    ctx;
  const errors: SourceError[] = [];

  const {
    progress: p,
    logScanResults,
    installOptions,
  } = buildSourceCallbacks(ctx, source, captureMode);

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
    selectName,
    pluginSkipCapture: captureMode === "skip",
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
    return errors;
  }

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
    return errors;
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

  return errors;
}
