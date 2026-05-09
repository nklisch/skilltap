import type { DiscoveredSkill, Output, StatusReport } from "@skilltap/core";
import { discoverSkills, gatherStatus } from "@skilltap/core";
import { defineCommand } from "citty";
import { ansi, table, termWidth, truncate } from "../ui/format";
import { exitOnError } from "../ui/exit";
import { tryFindProjectRoot } from "../ui/resolve";
import { setupOutput } from "../ui/setup";

export default defineCommand({
  meta: {
    name: "status",
    description: "Show installed skills, plugins, taps, and project drift",
  },
  args: {
    json: {
      type: "boolean",
      description: "Output as JSON",
      default: false,
    },
    unmanaged: {
      type: "boolean",
      description: "Show unmanaged skills (on disk but not in state)",
      default: false,
    },
    disabled: {
      type: "boolean",
      description: "Show only disabled items",
      default: false,
    },
    active: {
      type: "boolean",
      description: "Show only active items",
      default: false,
    },
    scope: {
      type: "string",
      description: "Filter by scope: 'global' or 'project'",
    },
  },
  async run({ args }) {
    const out = setupOutput(args);
    const projectRoot = await tryFindProjectRoot();

    // Filter modes: --unmanaged, --disabled, --active
    if (args.unmanaged) {
      return runUnmanagedMode(
        out,
        args as { scope?: string; json: boolean },
        projectRoot,
      );
    }

    const result = await gatherStatus();
    exitOnError(result, out);
    let report = result.value;

    // Apply --disabled / --active filters
    if (args.disabled) {
      report = {
        ...report,
        skills: report.skills.filter((s) => !s.active),
        plugins: report.plugins.filter((p) => !p.active),
      };
    } else if (args.active) {
      report = {
        ...report,
        skills: report.skills.filter((s) => s.active),
        plugins: report.plugins.filter((p) => p.active),
      };
    }

    // Apply scope filter
    if (args.scope === "global" || args.scope === "project") {
      const scopeVal = args.scope;
      report = {
        ...report,
        skills: report.skills.filter((s) => s.scope === scopeVal),
        plugins: report.plugins.filter((p) => p.scope === scopeVal),
      };
    }

    if (args.json) {
      out.json(reportToJson(report));
      return;
    }

    renderStatus(out, report);
  },
});

function reportToJson(report: StatusReport): unknown {
  return {
    projectRoot: report.projectRoot,
    hasManifest: report.hasManifest,
    scope: report.scope,
    also: report.also,
    fromV2State: report.fromV2State,
    skills: report.skills,
    plugins: report.plugins,
    taps: report.taps,
    drift: report.drift,
  };
}

function renderStatus(out: Output, report: StatusReport): void {
  // ── Header ───────────────────────────────────────────────────────────────
  const projectLabel = report.projectRoot
    ? `${ansi.bold(`project: ${shorten(report.projectRoot)}`)}${ansi.dim(report.hasManifest ? " (manifest)" : " (no manifest)")}`
    : ansi.bold("global (no project root)");
  out.raw(
    `\n${ansi.bold("skilltap status")} ${ansi.dim("—")} ${projectLabel}\n\n`,
  );

  // ── Scope + targets ──────────────────────────────────────────────────────
  out.raw(`${ansi.dim("Scope:")} ${report.scope}\n`);
  out.raw(
    `${ansi.dim("Targets:")} ${report.also.length === 0 ? ansi.dim("(none)") : report.also.join(", ")}\n`,
  );
  if (!report.fromV2State) {
    out.raw(
      `${ansi.dim("State:")} reading v1.0 (run ${ansi.bold("skilltap migrate")} to upgrade)\n`,
    );
  }
  out.raw("\n");

  // ── Skills ───────────────────────────────────────────────────────────────
  if (report.skills.length === 0) {
    out.raw(`${ansi.dim("Skills:")} ${ansi.dim("(none)")}\n\n`);
  } else {
    const managed = report.skills.filter((s) => s.scope !== "linked").length;
    const linked = report.skills.filter((s) => s.scope === "linked").length;
    out.raw(
      `${ansi.bold(`Skills`)} ${ansi.dim(`(${managed} managed, ${linked} linked)`)}\n`,
    );
    for (const skill of report.skills) {
      const flag = skill.active ? ansi.green("✓") : ansi.dim("✗");
      const sourceText = skill.source
        ? `${ansi.dim(skill.source)}${skill.ref ? ansi.dim(`@${skill.ref}`) : ""}`
        : ansi.dim("(local)");
      const alsoText =
        skill.also.length > 0 ? ansi.dim(` [${skill.also.join(", ")}]`) : "";
      out.raw(
        `  ${flag} ${skill.name} ${ansi.dim(skill.scope)}${alsoText} ${sourceText}\n`,
      );
    }
    out.raw("\n");
  }

  // ── Plugins ──────────────────────────────────────────────────────────────
  if (report.plugins.length === 0) {
    out.raw(`${ansi.dim("Plugins:")} ${ansi.dim("(none)")}\n\n`);
  } else {
    out.raw(
      `${ansi.bold(`Plugins`)} ${ansi.dim(`(${report.plugins.length})`)}\n`,
    );
    for (const plugin of report.plugins) {
      const flag = plugin.active ? ansi.green("✓") : ansi.dim("✗");
      const sourceText = plugin.source
        ? `${ansi.dim(plugin.source)}${plugin.ref ? ansi.dim(`@${plugin.ref}`) : ""}`
        : ansi.dim("(local)");
      out.raw(
        `  ${flag} ${plugin.name} ${ansi.dim(plugin.scope)} ${ansi.dim(`(${plugin.componentSummary})`)} ${sourceText}\n`,
      );
    }
    out.raw("\n");
  }

  // ── Taps ─────────────────────────────────────────────────────────────────
  out.raw(`${ansi.bold("Taps")} ${ansi.dim(`(${report.taps.length})`)}\n`);
  for (const tap of report.taps) {
    const label = tap.builtin
      ? `${tap.name} ${ansi.dim("(built-in)")}`
      : tap.name;
    const typeLabel =
      tap.type === "http"
        ? ansi.yellow(" (http — removed in v2.0; run 'skilltap migrate')")
        : "";
    out.raw(`  ${label}${typeLabel} ${ansi.dim(tap.url)}\n`);
  }
  out.raw("\n");

  // ── Drift ────────────────────────────────────────────────────────────────
  if (report.drift) {
    if (report.drift.inSync) {
      out.raw(`${ansi.green("✓")} Manifest in sync with installed state.\n`);
    } else {
      const counts = countByKind(report.drift.items);
      out.raw(
        `${ansi.yellow("Drift:")} ${counts.summary}. Run ${ansi.bold("skilltap sync")} for details.\n`,
      );
    }
  }
}

function countByKind(items: { kind: string }[]): { summary: string } {
  const c: Record<string, number> = {};
  for (const item of items) c[item.kind] = (c[item.kind] ?? 0) + 1;
  const parts: string[] = [];
  if (c.add) parts.push(`${c.add} to add`);
  if (c.remove) parts.push(`${c.remove} to remove`);
  if (c["ref-mismatch"]) parts.push(`${c["ref-mismatch"]} ref mismatch`);
  if (c["lock-stale"]) parts.push(`${c["lock-stale"]} lock stale`);
  if (c["lock-missing"]) parts.push(`${c["lock-missing"]} lock missing`);
  if (c["lock-orphan"]) parts.push(`${c["lock-orphan"]} lock orphan`);
  return { summary: parts.length === 0 ? "no changes" : parts.join(", ") };
}

function shorten(path: string): string {
  const home = process.env.HOME;
  if (home && path.startsWith(home)) return `~${path.slice(home.length)}`;
  return path;
}

// ─── Unmanaged mode ───────────────────────────────────────────────────────────

async function runUnmanagedMode(
  out: Output,
  args: { scope?: string; json: boolean },
  projectRoot: string | undefined,
): Promise<void> {
  const discoverOpts =
    args.scope === "global"
      ? { global: true as const, unmanagedOnly: true, projectRoot }
      : args.scope === "project"
        ? { project: true as const, unmanagedOnly: true, projectRoot }
        : { unmanagedOnly: true, projectRoot };

  const discoverResult = await discoverSkills(discoverOpts);
  if (!discoverResult.ok) {
    out.error(discoverResult.error.message, discoverResult.error.hint);
    process.exit(1);
  }

  const skills = discoverResult.value.skills;

  if (args.json) {
    out.json(skills);
    return;
  }

  if (skills.length === 0) {
    out.info("No unmanaged skills found.");
    return;
  }

  const width = termWidth();
  const count = skills.length;
  out.raw(
    `\n${ansi.bold("Unmanaged skills")} (${count} ${count === 1 ? "skill" : "skills"})\n`,
  );

  const NAME_W = width < 60 ? 15 : 20;
  const SRC_W = width < 60 ? 20 : 32;

  const rows = skills.map((s: DiscoveredSkill) => [
    truncate(s.name, NAME_W),
    ansi.yellow("unmanaged"),
    truncate(s.gitRemote ?? "(local)", SRC_W),
  ]);

  out.raw(`${table(rows, { header: ["Name", "Status", "Source"] })}\n`);
  out.raw(
    `\nRun 'skilltap adopt <name>' to take ownership of an unmanaged skill.\n`,
  );
}
