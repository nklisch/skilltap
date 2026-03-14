import { intro, isCancel, log, outro, spinner } from "@clack/prompts";
import { createStepLogger } from "../ui/install-steps";
import type {
  AgentAdapter,
  Config,
  EffectivePolicy,
  ScannedSkill,
  SemanticWarning,
  StaticWarning,
  TapEntry,
} from "@skilltap/core";
import {
  findProjectRoot,
  installSkill,
  saveConfig,
  skillInstallDir,
  updateSkill,
} from "@skilltap/core";
import { defineCommand } from "citty";
import {
  agentError,
  agentSecurityBlock,
  agentSuccess,
} from "../ui/agent-out";
import { errorLine, successLine } from "../ui/format";
import { createInstallCallbacks } from "../ui/install-callbacks";
import { loadPolicyOrExit } from "../ui/policy";
import {
  confirmSaveDefault,
  selectAgents,
} from "../ui/prompts";
import {
  parseAlsoFlag,
  resolveAgentForAgentMode,
  resolveScope,
  resolveSemanticInteractive,
} from "../ui/resolve";
import { inferAdapter, sendEvent, telemetryBase } from "../telemetry";

export default defineCommand({
  meta: {
    name: "install",
    description: "Install a skill from a URL, tap name, or local path",
  },
  args: {
    source: {
      type: "positional",
      description: "Git URL, github:owner/repo, tap skill name, or local path",
      required: true,
    },
    project: {
      type: "boolean",
      description: "Install to .agents/skills/ in current project",
      default: false,
    },
    global: {
      type: "boolean",
      description: "Install to ~/.agents/skills/",
      default: false,
    },
    also: {
      description: "Create symlink in agent-specific directory",
      valueHint: "agent",
    },
    ref: {
      description: "Branch or tag to install",
      valueHint: "ref",
    },
    "skip-scan": {
      type: "boolean",
      description: "Skip security scanning (not available in agent mode)",
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
    "no-strict": {
      type: "boolean",
      description: "Override config on_warn=fail for this invocation",
    },
    quiet: {
      type: "boolean",
      description: "Suppress install step details (overrides verbose = true in config)",
    },
    semantic: {
      type: "boolean",
      description: "Force semantic scan",
      default: false,
    },
  },
  async run({ args }) {
    const { config, policy } = await loadPolicyOrExit({
      strict: args.strict,
      noStrict: args["no-strict"],
      skipScan: args["skip-scan"],
      yes: args.yes,
      semantic: args.semantic,
      project: args.project,
      global: args.global,
    });

    const verbose = args.quiet ? false : config.verbose;
    const sources = args._ as string[];

    if (policy.agentMode) {
      return runAgentMode(sources, args, config, policy);
    }
    return runInteractiveMode(sources, args, config, policy, verbose);
  },
});

// ─── Agent Mode ───────────────────────────────────────────────────────────────

async function runAgentMode(
  sources: string[],
  args: { ref?: string; also?: string },
  config: Config,
  policy: EffectivePolicy,
): Promise<void> {
  const scope = policy.scope as "global" | "project";
  const projectRoot =
    scope === "project" ? await findProjectRoot() : undefined;

  let agent: AgentAdapter | undefined;
  if (policy.scanMode === "semantic") {
    agent = await resolveAgentForAgentMode(config);
  }

  const also = parseAlsoFlag(args.also, config);

  for (const source of sources) {
    const result = await installSkill(source, {
      scope,
      projectRoot,
      also,
      ref: args.ref,
      skipScan: false,
      gitHost: config.default_git_host,
      onWarnings: async (
        warnings: StaticWarning[],
      ): Promise<boolean> => {
        agentSecurityBlock(warnings, []);
        process.exit(1);
        return false;
      },
      onSelectSkills: async (skills: ScannedSkill[]): Promise<string[]> =>
        skills.map((s) => s.name),
      onSelectTap: async (matches: TapEntry[]): Promise<TapEntry | null> =>
        matches[0] ?? null,
      agent,
      semantic: policy.scanMode === "semantic",
      threshold: config.security.threshold,
      onSemanticWarnings: async (
        warnings: SemanticWarning[],
      ): Promise<boolean> => {
        agentSecurityBlock([], warnings);
        process.exit(1);
        return false;
      },
    });

    if (!result.ok) {
      sendEvent(config, "install", {
        ...telemetryBase(true),
        adapter: inferAdapter(source),
        success: false,
        error_category: result.error.constructor.name,
        skill_count: 0,
        scan_mode: policy.scanMode,
        scope,
      });
      agentError(result.error.message);
      process.exit(1);
    }

    sendEvent(config, "install", {
      ...telemetryBase(true),
      adapter: inferAdapter(source),
      success: true,
      skill_count: result.value.records.length,
      scan_mode: policy.scanMode,
      scope,
    });

    for (const record of result.value.records) {
      const installDir = skillInstallDir(record.name, scope, projectRoot);
      agentSuccess(record.name, installDir, record.ref, record.trust);
    }

    for (const name of result.value.updates) {
      const updateResult = await updateSkill({
        name,
        yes: true,
        projectRoot,
        agent,
        semantic: policy.scanMode === "semantic",
        threshold: config.security.threshold,
        onSemanticWarnings: (warnings) => {
          agentSecurityBlock([], warnings);
          process.exit(1);
        },
      });
      if (!updateResult.ok) {
        agentError(updateResult.error.message);
        process.exit(1);
      }
      const { updated, upToDate } = updateResult.value;
      if (updated.includes(name)) {
        process.stdout.write(`OK: Updated ${name}\n`);
      } else if (upToDate.includes(name)) {
        process.stdout.write(`OK: ${name} is already up to date.\n`);
      }
    }
  }
}

// ─── Interactive Mode ─────────────────────────────────────────────────────────

async function runInteractiveMode(
  sources: string[],
  args: {
    ref?: string;
    also?: string;
    semantic: boolean;
  },
  config: Config,
  policy: EffectivePolicy,
  verbose: boolean,
): Promise<void> {
  const { onWarn, skipScan } = policy;
  let also = parseAlsoFlag(args.also, config);

  const { runSemantic, agent } = await resolveSemanticInteractive(policy, args, config);

  intro("skilltap");

  // Scope: policy already resolved from flags + config. Only prompt if still "".
  let scope: "global" | "project";
  let projectRoot: string | undefined;
  if (policy.scope) {
    scope = policy.scope as "global" | "project";
    if (scope === "project") projectRoot = await findProjectRoot();
  } else {
    const resolved = await resolveScope({}, undefined);
    scope = resolved.scope;
    projectRoot = resolved.projectRoot;
  }

  // Prompt for agent symlinks unless --also was explicitly passed, --yes is set,
  // or the user already has a saved default in config
  if (!args.also && !policy.yes && !config.defaults.also.length) {
    const selected = await selectAgents(also);
    if (isCancel(selected)) process.exit(2);
    also = selected as string[];

    // Offer to save if selection differs from config default
    const configAlso = config.defaults.also;
    const differs =
      also.length !== configAlso.length ||
      also.some((a) => !configAlso.includes(a));
    if (differs) {
      const save = await confirmSaveDefault(
        "Save agent selection as default?",
      );
      if (!isCancel(save) && save) {
        config.defaults.also = also;
        await saveConfig(config);
      }
    }
  }

  const errors: { source: string; message: string; hint?: string }[] = [];

  for (const source of sources) {
    const s = spinner();
    s.start(`Fetching ${source}...`);

    const steps = createStepLogger(verbose);
    const { callbacks, logScanResults } = createInstallCallbacks({
      spinner: s, onWarn, skipScan, agent, yes: policy.yes, source, steps,
    });

    const result = await installSkill(source, {
      scope,
      projectRoot,
      also,
      ref: args.ref,
      skipScan,
      gitHost: config.default_git_host,
      agent,
      semantic: runSemantic,
      threshold: config.security.threshold,
      ...callbacks,
    });

    if (!result.ok) {
      s.stop();
      sendEvent(config, "install", {
        ...telemetryBase(false),
        adapter: inferAdapter(source),
        success: false,
        error_category: result.error.constructor.name,
        skill_count: 0,
        scan_mode: policy.scanMode,
        scope,
      });
      errors.push({ source, message: result.error.message, hint: result.error.hint });
      continue;
    }

    s.stop();
    logScanResults();

    sendEvent(config, "install", {
      ...telemetryBase(false),
      adapter: inferAdapter(source),
      success: true,
      skill_count: result.value.records.length,
      scan_mode: policy.scanMode,
      scope,
    });

    for (const record of result.value.records) {
      const installDir = skillInstallDir(record.name, scope, projectRoot);
      successLine(`Installed ${record.name} → ${installDir}`);
    }

    // Run updates for skills that were already installed
    for (const name of result.value.updates) {
      const updateResult = await updateSkill({
        name,
        yes: policy.yes,
        projectRoot,
        agent,
        semantic: runSemantic,
        threshold: config.security.threshold,
      });
      if (!updateResult.ok) {
        errorLine(updateResult.error.message, updateResult.error.hint);
      } else {
        const { updated, upToDate } = updateResult.value;
        if (updated.includes(name)) successLine(`Updated ${name}`);
        else if (upToDate.includes(name)) log.info(`${name} is already up to date.`);
      }
    }
  }

  if (errors.length > 0) {
    for (const { source, message, hint } of errors) {
      errorLine(`${source}: ${message}`, hint);
    }
    outro("Finished with errors.");
    process.exit(1);
  }

  outro("Complete!");
}
