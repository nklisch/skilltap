import { isCancel, log } from "@clack/prompts";
import {
  type Config,
  type EffectivePolicy,
  fetchSkillUpdateStatus,
  formatOrphanReason,
  loadState,
  type OrphanRecord,
  type Output,
  type SemanticWarning,
  type StaticWarning,
  updateSkill,
  updateTap,
  writeSkillUpdateCache,
} from "@skilltap/core";
import { defineCommand } from "citty";
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
import { resolveSemanticInteractive, tryFindProjectRoot } from "../ui/resolve";
import { printSemanticWarnings, printWarnings } from "../ui/scan";
import { setupOutput } from "../ui/setup";

const VALID_UPDATE_TYPES = ["skill", "plugin", "mcp"] as const;
type UpdateType = (typeof VALID_UPDATE_TYPES)[number];

export default defineCommand({
  meta: {
    name: "update",
    description:
      "Update installed skills, plugins, and MCP servers. Bare = all.",
  },
  args: {
    type: {
      type: "positional",
      required: false,
      description: "skill | plugin | mcp. Omit to update everything.",
    },
    name: {
      type: "positional",
      required: false,
      description: "Specific name. Omit to update all of the chosen type.",
    },
    scope: {
      type: "string",
      description:
        "Install scope to update (project | global). Defaults to smart-scope (project inside a git repo, global otherwise).",
      valueHint: "project|global",
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
    "skip-scan": {
      type: "boolean",
      description: "Skip security scanning",
      default: false,
    },
    quiet: {
      type: "boolean",
      description: "Suppress output details",
      default: false,
    },
  },
  async run({ args }) {
    const out = setupOutput(args);

    // Validate type if provided — catch "update bogus" early
    const typeArg = args.type as string | undefined;
    if (typeArg && !VALID_UPDATE_TYPES.includes(typeArg as UpdateType)) {
      out.error(
        `Invalid type: "${typeArg}".`,
        `Valid types: ${VALID_UPDATE_TYPES.join(", ")}. Or omit type to update everything.`,
      );
      process.exit(1);
    }

    const updateType = typeArg as UpdateType | undefined;
    const name = args.name as string | undefined;

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

    const projectRoot =
      scopeFlag === "global" ? undefined : await tryFindProjectRoot();

    if (args.check) {
      return runCheckMode(out, projectRoot, args.json ?? false);
    }

    const { config, policy } = await loadPolicyOrExit({
      strict: args.strict,
      yes: args.yes,
      scope: scopeFlag,
    });

    await refreshTapIndexes(out);

    return runUpdate(
      out,
      updateType,
      name,
      args,
      config,
      policy,
      projectRoot,
      args.force ?? false,
    );
  },
});

// ─── Tap Refresh ──────────────────────────────────────────────────────────────

async function refreshTapIndexes(out: Output): Promise<void> {
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
  out: Output,
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
    for (const skillName of updates) {
      log.step(`${ansi.bold(skillName)} — update available`);
    }
    out.raw(`\nRun ${ansi.bold("skilltap update")} to apply.\n`);
  }
}

// ─── Update Dispatch ──────────────────────────────────────────────────────────

async function runUpdate(
  out: Output,
  type: UpdateType | undefined,
  name: string | undefined,
  args: {
    strict?: boolean;
    semantic: boolean;
    json?: boolean;
    "skip-scan"?: boolean;
  },
  config: Config,
  policy: EffectivePolicy,
  projectRoot: string | undefined,
  force = false,
): Promise<void> {
  // Dispatch by type:
  //   undefined → update all skills (+ note plugins/mcp coming soon)
  //   "skill"   → update skills only (optionally one by name)
  //   "plugin"  → not yet implemented in core
  //   "mcp"     → not yet implemented in core
  if (type === "plugin") {
    await runUpdatePlugins(out, name, projectRoot);
    return;
  }
  if (type === "mcp") {
    await runUpdateMcps(out, name, projectRoot);
    return;
  }

  // skill or undefined: update skills
  await runUpdateSkills(out, name, args, config, policy, projectRoot, force);
}

async function runUpdatePlugins(
  out: Output,
  name: string | undefined,
  projectRoot: string | undefined,
): Promise<void> {
  const stateResult = await loadState(projectRoot);
  if (!stateResult.ok) {
    out.error(stateResult.error.message);
    process.exit(1);
  }
  const plugins = stateResult.value.plugins;

  const targets = name ? plugins.filter((p) => p.name === name) : plugins;

  if (name && targets.length === 0) {
    out.error(
      `Plugin '${name}' is not installed.`,
      "Run 'skilltap status' to see installed plugins.",
    );
    process.exit(1);
  }

  if (targets.length === 0) {
    log.info("No plugins installed.");
    return;
  }

  // Plugin update (re-install from source) is not yet implemented in core.
  // The CLI surface is wired so users get a clear message instead of a stub.
  out.info(
    `Plugin update is not yet implemented. Re-install with: skilltap install plugin <source>`,
  );
}

async function runUpdateMcps(
  out: Output,
  name: string | undefined,
  projectRoot: string | undefined,
): Promise<void> {
  const stateResult = await loadState(projectRoot);
  if (!stateResult.ok) {
    out.error(stateResult.error.message);
    process.exit(1);
  }
  const mcpServers = stateResult.value.mcpServers;

  const targets = name ? mcpServers.filter((m) => m.name === name) : mcpServers;

  if (name && targets.length === 0) {
    out.error(
      `MCP server '${name}' is not installed.`,
      "Run 'skilltap status' to see installed MCP servers.",
    );
    process.exit(1);
  }

  if (targets.length === 0) {
    log.info("No MCP servers installed.");
    return;
  }

  // MCP update (re-install from source) is not yet implemented in core.
  // The CLI surface is wired so users get a clear message instead of a stub.
  out.info(
    `MCP server update is not yet implemented. Re-install with: skilltap install mcp <source>`,
  );
}

// ─── Skill Update ─────────────────────────────────────────────────────────────

async function runUpdateSkills(
  out: Output,
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
    threshold: config.scanner.threshold,
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
        message: `Remove "${skillName}" from skilltap?`,
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
        out.raw(`${formatDiffFileLine(file)}\n`);
      }
      if (level === "full" && rawDiff.trim()) {
        out.raw(`\n${formatUnifiedDiff(rawDiff)}\n`);
      }
    },

    onShowWarnings(warnings: StaticWarning[], skillName: string) {
      printWarnings(warnings, skillName, out);
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
        score >= (config.scanner.threshold ?? 5)
          ? ` — ⚠ ${reason.length > 60 ? `${reason.slice(0, 59)}…` : reason}`
          : "";
      semProgress?.update(`Semantic scan: chunk ${completed}/${total}${flag}`);
    },

    onSemanticWarnings(warnings: SemanticWarning[], skillName: string) {
      if (semProgress) {
        semProgress.fail();
        semProgress = null;
      }
      printSemanticWarnings(warnings, skillName, out);
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
    out.raw(`\n${ansi.dim(summary)}\n`);
  } else if (!name) {
    log.info("No skills installed.");
  }
}
