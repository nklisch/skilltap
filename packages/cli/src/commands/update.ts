import { isCancel, log, spinner } from "@clack/prompts";
import { footerConfirm as confirm } from "../ui/footer";
import {
  type AgentAdapter,
  type Config,
  type EffectivePolicy,
  fetchSkillUpdateStatus,
  formatOrphanReason,
  type OnOrphansFound,
  type OrphanRecord,
  type SemanticWarning,
  type StaticWarning,
  updateSkill,
  updateTap,
  writeSkillUpdateCache,
} from "@skilltap/core";
import { defineCommand } from "citty";
import {
  agentError,
  agentSecurityBlock,
  agentSkip,
  agentUpToDate,
  outputJson,
} from "../ui/agent-out";
import {
  ansi,
  errorLine,
  formatDiffFileLine,
  formatDiffStatSummary,
  formatShaChange,
  formatUnifiedDiff,
  successLine,
} from "../ui/format";
import { loadPolicyOrExit } from "../ui/policy";
import {
  resolveAgentForAgentMode,
  resolveSemanticInteractive,
  tryFindProjectRoot,
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
    json: {
      type: "boolean",
      description: "Output result as JSON",
      default: false,
    },
    check: {
      type: "boolean",
      alias: "c",
      description: "Check for updates without applying them. Refreshes the update cache.",
      default: false,
    },
    force: {
      type: "boolean",
      alias: "f",
      description: "Force update even if skill appears up to date (re-applies and re-scans).",
      default: false,
    },
  },
  async run({ args }) {
    const name = args.name as string | undefined;
    const projectRoot = await tryFindProjectRoot();

    if (args.check) {
      return runCheckMode(projectRoot, args.json);
    }

    const { config, policy } = await loadPolicyOrExit({
      strict: args.strict,
      yes: args.yes,
      semantic: args.semantic,
    });

    await refreshTapIndexes(policy.agentMode);

    if (policy.agentMode) {
      return runAgentModeUpdate(name, config, policy, projectRoot, args.json, args.force);
    }
    return runInteractiveUpdate(name, args, config, policy, projectRoot, args.force);
  },
});

// ─── Tap Refresh ──────────────────────────────────────────────────────────────

async function refreshTapIndexes(agentMode: boolean): Promise<void> {
  if (agentMode) {
    process.stdout.write("Refreshing tap indexes...\n");
    const result = await updateTap();
    if (!result.ok) {
      process.stdout.write(`Warning: Could not refresh tap indexes: ${result.error.message}\n`);
    }
    return;
  }

  const s = spinner();
  s.start("Refreshing tap indexes...");
  const result = await updateTap();
  if (!result.ok) {
    s.stop(`Could not refresh tap indexes: ${result.error.message}`);
  } else {
    s.stop("Tap indexes refreshed.");
  }
}

// ─── Check Mode ───────────────────────────────────────────────────────────────

async function runCheckMode(
  projectRoot: string | undefined,
  json = false,
): Promise<void> {
  const pr = projectRoot ?? null;

  if (json || !process.stdout.isTTY) {
    const updates = await fetchSkillUpdateStatus(pr);
    await writeSkillUpdateCache(updates, pr);
    outputJson({ updatesAvailable: updates });
    return;
  }

  const { spinner } = await import("@clack/prompts");
  const s = spinner();
  s.start("Checking skills for updates…");
  const updates = await fetchSkillUpdateStatus(pr);
  await writeSkillUpdateCache(updates, pr);
  s.stop(
    updates.length === 0
      ? "All skills are up to date."
      : `${updates.length} skill update${updates.length === 1 ? "" : "s"} available.`,
  );

  if (updates.length > 0) {
    for (const name of updates) {
      log.step(`${ansi.bold(name)} — update available`);
    }
    process.stdout.write(`\nRun ${ansi.bold("skilltap update")} to apply.\n`);
  }
}

// ─── Agent Mode ───────────────────────────────────────────────────────────────

async function runAgentModeUpdate(
  name: string | undefined,
  config: Config,
  policy: EffectivePolicy,
  projectRoot: string | undefined,
  useJson = false,
  force = false,
): Promise<void> {
  let agent: AgentAdapter | undefined;
  if (policy.scanMode === "semantic") {
    agent = await resolveAgentForAgentMode(config);
  }

  const result = await updateSkill({
    name,
    yes: true,
    strict: true,
    force,
    agent,
    semantic: policy.scanMode === "semantic",
    threshold: config.security.threshold,
    projectRoot,

    onProgress(skillName, status) {
      if (status === "upToDate") agentUpToDate(skillName);
      else if (status === "linked") agentSkip(skillName, "is linked.");
      else if (status === "local") agentSkip(skillName, "is local (no remote).");
      else if (status === "removed-upstream") {
        process.stdout.write(`${skillName}: removed from upstream repo\n`);
      }
    },

    async onOrphansFound(orphans: OrphanRecord[]) {
      for (const o of orphans) {
        process.stdout.write(
          `warning: Stale record "${o.record.name}" — ${formatOrphanReason(o.reason)}. Auto-removing.\n`,
        );
      }
      return orphans.map((o) => o.record.name);
    },

    async onSkillRemovedUpstream(skillName: string) {
      process.stdout.write(
        `warning: "${skillName}" removed from upstream repo. Auto-removing record.\n`,
      );
      return "remove" as const;
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

  if (useJson) {
    outputJson({ updated, skipped, upToDate });
    return;
  }

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
  force = false,
): Promise<void> {
  const { runSemantic, agent } = await resolveSemanticInteractive(policy, args, config);

  let semSpinner: ReturnType<typeof spinner> | null = null;

  const result = await updateSkill({
    name,
    yes: policy.yes,
    strict: policy.onWarn === "fail",
    force,
    agent,
    semantic: runSemantic,
    threshold: config.security.threshold,
    projectRoot,

    onProgress(skillName, status) {
      if (semSpinner) {
        semSpinner.stop();
        semSpinner = null;
      }
      if (status === "checking") {
        log.step(`Checking ${ansi.bold(skillName)}...`);
      } else if (status === "upToDate") {
        log.info("Already up to date.");
      } else if (status === "linked") {
        log.info("Skipped (linked).");
      } else if (status === "local") {
        log.info("Skipped (local, no remote).");
      } else if (status === "removed-upstream") {
        log.warn("Removed from upstream.");
      }
    },

    async onOrphansFound(orphans: OrphanRecord[]) {
      if (orphans.length === 0) return [];
      log.warn(`Found ${orphans.length} stale record(s):`);
      for (const o of orphans) {
        log.warn(`  ${o.record.name}: ${formatOrphanReason(o.reason)}`);
      }
      const shouldClean = await confirm({
        message: "Remove stale records? (directories are already gone)",
        initialValue: true,
      });
      if (isCancel(shouldClean)) process.exit(130);
      if (!shouldClean) return [];
      return orphans.map((o) => o.record.name);
    },

    async onSkillRemovedUpstream(skillName: string) {
      log.warn(`"${skillName}" was removed from the upstream repo.`);
      const action = await confirm({
        message: `Remove "${skillName}" from installed.json?`,
        initialValue: true,
      });
      if (isCancel(action)) process.exit(130);
      return !action ? ("skip" as const) : ("remove" as const);
    },

    onDiff(_skillName, stat, fromSha, toSha, rawDiff) {
      const level = config.updates.show_diff;
      if (level === "none") return;
      const shaChange = formatShaChange(fromSha, toSha);
      const statSummary = formatDiffStatSummary(stat);
      log.info(`${shaChange} ${ansi.dim(statSummary)}`);
      for (const file of stat.files) {
        process.stdout.write(`${formatDiffFileLine(file)}\n`);
      }
      if (level === "full" && rawDiff.trim()) {
        process.stdout.write(`\n${formatUnifiedDiff(rawDiff)}\n`);
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
      if (isCancel(answer)) process.exit(130);
      return answer as boolean;
    },

    onSemanticScanStart(skillName: string) {
      semSpinner = spinner();
      semSpinner.start(`Semantic scan of ${ansi.bold(skillName)}...`);
    },

    onSemanticProgress(completed: number, total: number, score: number, reason: string) {
      const flag = score >= (config.security.threshold ?? 5) ? ` — ⚠ ${reason.length > 60 ? `${reason.slice(0, 59)}…` : reason}` : "";
      semSpinner?.message(`Semantic scan: chunk ${completed}/${total}${flag}`);
    },

    onSemanticWarnings(warnings: SemanticWarning[], skillName: string) {
      if (semSpinner) {
        semSpinner.stop();
        semSpinner = null;
      }
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
