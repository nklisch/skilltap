import { confirm, isCancel, log } from "@clack/prompts";
import {
  type AgentAdapter,
  type Config,
  type EffectivePolicy,
  findProjectRoot,
  type SemanticWarning,
  type StaticWarning,
  updateSkill,
} from "@skilltap/core";
import { defineCommand } from "citty";
import {
  agentError,
  agentSecurityBlock,
  agentSkip,
  agentUpToDate,
} from "../ui/agent-out";
import {
  ansi,
  errorLine,
  formatDiffFileLine,
  formatDiffStatSummary,
  formatShaChange,
  successLine,
} from "../ui/format";
import { loadPolicyOrExit } from "../ui/policy";
import {
  resolveAgentForAgentMode,
  resolveAgentInteractive,
} from "../ui/resolve";
import { printSemanticWarnings, printWarnings } from "../ui/scan";
import { sendEvent, telemetryBase } from "../telemetry";

export default defineCommand({
  meta: {
    name: "update",
    description: "Update installed skill(s)",
  },
  args: {
    name: {
      type: "positional",
      description: "Specific skill to update (omit to update all)",
      required: false,
    },
    yes: {
      type: "boolean",
      alias: "y",
      description: "Auto-accept clean updates",
      default: false,
    },
    strict: {
      type: "boolean",
      description: "Skip skills with security warnings in diff",
    },
    semantic: {
      type: "boolean",
      description: "Run semantic scan on updated skills",
      default: false,
    },
  },
  async run({ args }) {
    const name = args.name as string | undefined;

    const { config, policy } = await loadPolicyOrExit({
      strict: args.strict,
      yes: args.yes,
      semantic: args.semantic,
    });

    const projectRoot = await findProjectRoot().catch(() => undefined);

    if (policy.agentMode) {
      return runAgentModeUpdate(name, config, policy, projectRoot);
    }
    return runInteractiveUpdate(name, args, config, policy, projectRoot);
  },
});

// ─── Agent Mode ───────────────────────────────────────────────────────────────

async function runAgentModeUpdate(
  name: string | undefined,
  config: Config,
  policy: EffectivePolicy,
  projectRoot: string | undefined,
): Promise<void> {
  let agent: AgentAdapter | undefined;
  if (policy.scanMode === "semantic") {
    agent = await resolveAgentForAgentMode(config);
  }

  const result = await updateSkill({
    name,
    yes: true,
    strict: true,
    agent,
    semantic: policy.scanMode === "semantic",
    threshold: config.security.threshold,
    projectRoot,

    onProgress(skillName, status) {
      if (status === "upToDate") agentUpToDate(skillName);
      else if (status === "linked") agentSkip(skillName, "is linked.");
    },

    onDiff(skillName, stat, fromSha, toSha) {
      process.stdout.write(
        `Checking ${skillName}... ${fromSha.slice(0, 7)} → ${toSha.slice(0, 7)} (${stat.filesChanged} files changed)\n`,
      );
    },

    onShowWarnings(warnings: StaticWarning[]) {
      agentSecurityBlock(warnings, []);
    },

    async onConfirm() {
      return true;
    },

    onSemanticWarnings(warnings: SemanticWarning[]) {
      agentSecurityBlock([], warnings);
    },
  });

  if (!result.ok) {
    sendEvent(config, "update", {
      ...telemetryBase(true),
      success: false,
      error_category: result.error.constructor.name,
      updated_count: 0,
      up_to_date_count: 0,
    });
    agentError(result.error.message);
    process.exit(1);
  }

  const { updated, skipped, upToDate } = result.value;

  sendEvent(config, "update", {
    ...telemetryBase(true),
    success: true,
    updated_count: updated.length,
    up_to_date_count: upToDate.length,
  });

  for (const skillName of updated) {
    process.stdout.write(`OK: Updated ${skillName}\n`);
  }

  if (updated.length > 0 || skipped.length > 0 || upToDate.length > 0) {
    process.stdout.write(
      `\nUpdated: ${updated.length}   Skipped: ${skipped.length}   Up to date: ${upToDate.length}\n`,
    );
  } else if (!name) {
    process.stdout.write("No skills installed.\n");
  }
}

// ─── Interactive Mode ─────────────────────────────────────────────────────────

async function runInteractiveUpdate(
  name: string | undefined,
  args: { strict?: boolean; semantic: boolean },
  config: Config,
  policy: EffectivePolicy,
  projectRoot: string | undefined,
): Promise<void> {
  const runSemantic =
    policy.scanMode === "semantic" || args.semantic;

  let agent: AgentAdapter | undefined;
  if (runSemantic) {
    agent = await resolveAgentInteractive(config);
    if (!agent) {
      log.warn("No agent CLI found on PATH. Skipping semantic scan.");
    }
  }

  const result = await updateSkill({
    name,
    yes: policy.yes,
    strict: policy.onWarn === "fail",
    agent,
    semantic: runSemantic,
    threshold: config.security.threshold,
    projectRoot,

    onProgress(skillName, status) {
      if (status === "checking") {
        log.step(`Checking ${ansi.bold(skillName)}...`);
      } else if (status === "upToDate") {
        log.info("Already up to date.");
      } else if (status === "linked") {
        log.info("Skipped (linked).");
      }
    },

    onDiff(_skillName, stat, fromSha, toSha) {
      const shaChange = formatShaChange(fromSha, toSha);
      const statSummary = formatDiffStatSummary(stat);
      log.info(`${shaChange} ${ansi.dim(statSummary)}`);
      for (const file of stat.files) {
        process.stdout.write(`${formatDiffFileLine(file)}\n`);
      }
    },

    onShowWarnings(warnings: StaticWarning[], skillName: string) {
      printWarnings(warnings, skillName);
      if (policy.onWarn === "fail") {
        log.warn(
          `Security warnings in ${ansi.bold(skillName)} (strict mode). Skipping.`,
        );
      }
    },

    async onConfirm(skillName: string, hasWarnings: boolean) {
      const message = hasWarnings
        ? `Apply update to ${skillName} despite warnings?`
        : `Apply update to ${skillName}?`;
      const answer = await confirm({ message, initialValue: false });
      if (isCancel(answer)) return false;
      return answer as boolean;
    },

    onSemanticWarnings(warnings: SemanticWarning[], skillName: string) {
      printSemanticWarnings(warnings, skillName);
      if (policy.onWarn === "fail") {
        log.warn(
          `Semantic warnings in ${ansi.bold(skillName)} (strict mode). Skipping.`,
        );
      }
    },
  });

  if (!result.ok) {
    sendEvent(config, "update", {
      ...telemetryBase(false),
      success: false,
      error_category: result.error.constructor.name,
      updated_count: 0,
      up_to_date_count: 0,
    });
    errorLine(result.error.message, result.error.hint);
    process.exit(1);
  }

  const { updated, skipped, upToDate } = result.value;

  sendEvent(config, "update", {
    ...telemetryBase(false),
    success: true,
    updated_count: updated.length,
    up_to_date_count: upToDate.length,
  });

  for (const skillName of updated) {
    successLine(`Updated ${skillName}`);
  }

  if (updated.length > 0 || skipped.length > 0 || upToDate.length > 0) {
    const summary = [
      `Updated: ${updated.length}`,
      `Skipped: ${skipped.length}`,
      `Up to date: ${upToDate.length}`,
    ].join("   ");
    process.stdout.write(`\n${ansi.dim(summary)}\n`);
  } else if (!name) {
    log.info("No skills installed.");
  }
}
