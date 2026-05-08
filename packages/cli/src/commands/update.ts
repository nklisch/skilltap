import { isCancel, log } from "@clack/prompts";
import {
  type Config,
  type EffectivePolicy,
  fetchSkillUpdateStatus,
  formatOrphanReason,
  type OrphanRecord,
  type SemanticWarning,
  type StaticWarning,
  updateSkill,
  updateTap,
  writeSkillUpdateCache,
} from "@skilltap/core";
import { defineCommand } from "citty";
import { createOutput } from "../output";
import { sendEvent, telemetryBase } from "../telemetry";
import { footerConfirm as confirm } from "../ui/footer";
import {
  ansi,
  formatDiffFileLine,
  formatDiffStatSummary,
  formatShaChange,
  formatUnifiedDiff,
} from "../ui/format";
import { loadPolicyOrExit } from "../ui/policy";
import {
  resolveSemanticInteractive,
  tryFindProjectRoot,
} from "../ui/resolve";
import { printSemanticWarnings, printWarnings } from "../ui/scan";

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
      description:
        "Check for updates without applying them. Refreshes the update cache.",
      default: false,
    },
    force: {
      type: "boolean",
      alias: "f",
      description:
        "Force update even if skill appears up to date (re-applies and re-scans).",
      default: false,
    },
  },
  async run({ args }) {
    const out = createOutput({ json: args.json, quiet: false });
    const name = args.name as string | undefined;
    const projectRoot = await tryFindProjectRoot();

    if (args.check) {
      return runCheckMode(out, projectRoot, args.json);
    }

    const { config, policy } = await loadPolicyOrExit({
      strict: args.strict,
      yes: args.yes,
      semantic: args.semantic,
    });

    await refreshTapIndexes(out);

    return runUpdate(out, name, args, config, policy, projectRoot, args.force);
  },
});

// ─── Tap Refresh ──────────────────────────────────────────────────────────────

async function refreshTapIndexes(out: ReturnType<typeof createOutput>): Promise<void> {
  const p = out.progress("Refreshing tap indexes...");
  const result = await updateTap();
  if (!result.ok) {
    p.fail(`Could not refresh tap indexes: ${result.error.message}`);
  } else {
    p.succeed("Tap indexes refreshed.");
  }
}

// ─── Check Mode ───────────────────────────────────────────────────────────────

async function runCheckMode(
  out: ReturnType<typeof createOutput>,
  projectRoot: string | undefined,
  json = false,
): Promise<void> {
  const pr = projectRoot ?? null;

  if (json || !process.stdout.isTTY) {
    const updates = await fetchSkillUpdateStatus(pr);
    await writeSkillUpdateCache(updates, pr);
    out.json({ updatesAvailable: updates });
    return;
  }

  const p = out.progress("Checking skills for updates…");
  const updates = await fetchSkillUpdateStatus(pr);
  await writeSkillUpdateCache(updates, pr);
  p.succeed(
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

// ─── Update ───────────────────────────────────────────────────────────────────

async function runUpdate(
  out: ReturnType<typeof createOutput>,
  name: string | undefined,
  args: { strict?: boolean; semantic: boolean; json?: boolean },
  config: Config,
  policy: EffectivePolicy,
  projectRoot: string | undefined,
  force = false,
): Promise<void> {
  const { runSemantic, agent } = await resolveSemanticInteractive(
    policy,
    args,
    config,
  );

  let semProgress: ReturnType<typeof out.progress> | null = null;

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
      if (semProgress) {
        semProgress.succeed();
        semProgress = null;
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
      if (policy.yes) {
        for (const o of orphans) {
          log.warn(
            `Stale record "${o.record.name}" (${formatOrphanReason(o.reason)}). Auto-removing.`,
          );
        }
        return orphans.map((o) => o.record.name);
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
      semProgress = out.progress(`Semantic scan of ${ansi.bold(skillName)}...`);
    },

    onSemanticProgress(
      completed: number,
      total: number,
      score: number,
      reason: string,
    ) {
      const flag =
        score >= (config.security.threshold ?? 5)
          ? ` — ⚠ ${reason.length > 60 ? `${reason.slice(0, 59)}…` : reason}`
          : "";
      semProgress?.update(`Semantic scan: chunk ${completed}/${total}${flag}`);
    },

    onSemanticWarnings(warnings: SemanticWarning[], skillName: string) {
      if (semProgress) {
        semProgress.fail();
        semProgress = null;
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
      ...telemetryBase(),
      success: false,
      error_category: result.error.constructor.name,
      updated_count: 0,
      up_to_date_count: 0,
    });
    out.error(result.error.message, result.error.hint);
    process.exit(1);
  }

  const { updated, skipped, upToDate } = result.value;

  sendEvent(config, "update", {
    ...telemetryBase(),
    success: true,
    updated_count: updated.length,
    up_to_date_count: upToDate.length,
  });

  if (args.json) {
    out.json({ updated, skipped, upToDate });
    return;
  }

  for (const skillName of updated) {
    out.success(`Updated ${skillName}`);
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
