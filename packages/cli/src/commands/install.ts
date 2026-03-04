import { intro, isCancel, log, outro, spinner } from "@clack/prompts";
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
  resolveAgentInteractive,
  resolveScope,
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

    if (policy.agentMode) {
      return runAgentMode(args, config, policy);
    }
    return runInteractiveMode(args, config, policy);
  },
});

// ─── Agent Mode ───────────────────────────────────────────────────────────────

async function runAgentMode(
  args: { source: string; ref?: string; also?: string },
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

  const result = await installSkill(args.source, {
    scope,
    projectRoot,
    also,
    ref: args.ref,
    skipScan: false,
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
      adapter: inferAdapter(args.source),
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
    adapter: inferAdapter(args.source),
    success: true,
    skill_count: result.value.records.length,
    scan_mode: policy.scanMode,
    scope,
  });

  for (const record of result.value.records) {
    const installDir = skillInstallDir(record.name, scope, projectRoot);
    agentSuccess(record.name, installDir, record.ref, record.trust);
  }
}

// ─── Interactive Mode ─────────────────────────────────────────────────────────

async function runInteractiveMode(
  args: {
    source: string;
    ref?: string;
    also?: string;
    semantic: boolean;
  },
  config: Config,
  policy: EffectivePolicy,
): Promise<void> {
  const { onWarn, skipScan } = policy;
  let also = parseAlsoFlag(args.also, config);

  const runSemantic =
    policy.scanMode === "semantic" || args.semantic;

  let agent: AgentAdapter | undefined;
  if (runSemantic || config.security.scan === "semantic") {
    agent = await resolveAgentInteractive(config);
    if (!agent && runSemantic) {
      log.warn("No agent CLI found on PATH. Skipping semantic scan.");
    }
  }

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

  const s = spinner();
  s.start(`Cloning ${args.source}...`);

  const callbacks = createInstallCallbacks({
    spinner: s, onWarn, skipScan, agent, yes: policy.yes,
  });

  const result = await installSkill(args.source, {
    scope,
    projectRoot,
    also,
    ref: args.ref,
    skipScan,
    agent,
    semantic: runSemantic,
    threshold: config.security.threshold,
    ...callbacks,
  });

  if (!result.ok) {
    s.stop("Failed.", 1);
    sendEvent(config, "install", {
      ...telemetryBase(false),
      adapter: inferAdapter(args.source),
      success: false,
      error_category: result.error.constructor.name,
      skill_count: 0,
      scan_mode: policy.scanMode,
      scope,
    });
    errorLine(result.error.message, result.error.hint);
    process.exit(1);
  }

  sendEvent(config, "install", {
    ...telemetryBase(false),
    adapter: inferAdapter(args.source),
    success: true,
    skill_count: result.value.records.length,
    scan_mode: policy.scanMode,
    scope,
  });

  s.stop("Done.");

  for (const record of result.value.records) {
    const installDir = skillInstallDir(record.name, scope, projectRoot);
    successLine(`Installed ${record.name} → ${installDir}`);
  }

  // Run updates for skills that were already installed
  for (const name of result.value.updates) {
    const updateResult = await updateSkill({ name, yes: policy.yes, projectRoot });
    if (!updateResult.ok) {
      errorLine(updateResult.error.message, updateResult.error.hint);
    } else {
      const { updated, upToDate } = updateResult.value;
      if (updated.includes(name)) successLine(`Updated ${name}`);
      else if (upToDate.includes(name)) log.info(`${name} is already up to date.`);
    }
  }

  outro("Complete!");
}
