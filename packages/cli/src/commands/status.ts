import { gatherStatus, type StatusReport } from "@skilltap/core";
import { defineCommand } from "citty";
import { ansi, errorLine } from "../ui/format";

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
  },
  async run({ args }) {
    const result = await gatherStatus();
    if (!result.ok) {
      errorLine(result.error.message);
      process.exit(1);
    }
    const report = result.value;

    if (args.json) {
      process.stdout.write(
        `${JSON.stringify(reportToJson(report), null, 2)}\n`,
      );
      return;
    }

    renderStatus(report);
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

function renderStatus(report: StatusReport): void {
  // ── Header ───────────────────────────────────────────────────────────────
  const projectLabel = report.projectRoot
    ? `${ansi.bold(`project: ${shorten(report.projectRoot)}`)}${ansi.dim(report.hasManifest ? " (manifest)" : " (no manifest)")}`
    : ansi.bold("global (no project root)");
  process.stdout.write(
    `\n${ansi.bold("skilltap status")} ${ansi.dim("—")} ${projectLabel}\n\n`,
  );

  // ── Scope + targets ──────────────────────────────────────────────────────
  process.stdout.write(`${ansi.dim("Scope:")} ${report.scope}\n`);
  process.stdout.write(
    `${ansi.dim("Targets:")} ${report.also.length === 0 ? ansi.dim("(none)") : report.also.join(", ")}\n`,
  );
  if (!report.fromV2State) {
    process.stdout.write(
      `${ansi.dim("State:")} reading v1.0 (run ${ansi.bold("skilltap migrate")} to upgrade)\n`,
    );
  }
  process.stdout.write("\n");

  // ── Skills ───────────────────────────────────────────────────────────────
  if (report.skills.length === 0) {
    process.stdout.write(`${ansi.dim("Skills:")} ${ansi.dim("(none)")}\n\n`);
  } else {
    const managed = report.skills.filter((s) => s.scope !== "linked").length;
    const linked = report.skills.filter((s) => s.scope === "linked").length;
    process.stdout.write(
      `${ansi.bold(`Skills`)} ${ansi.dim(`(${managed} managed, ${linked} linked)`)}\n`,
    );
    for (const skill of report.skills) {
      const flag = skill.active ? ansi.green("✓") : ansi.dim("✗");
      const sourceText = skill.source
        ? `${ansi.dim(skill.source)}${skill.ref ? ansi.dim(`@${skill.ref}`) : ""}`
        : ansi.dim("(local)");
      const alsoText =
        skill.also.length > 0 ? ansi.dim(` [${skill.also.join(", ")}]`) : "";
      process.stdout.write(
        `  ${flag} ${skill.name} ${ansi.dim(skill.scope)}${alsoText} ${sourceText}\n`,
      );
    }
    process.stdout.write("\n");
  }

  // ── Plugins ──────────────────────────────────────────────────────────────
  if (report.plugins.length === 0) {
    process.stdout.write(`${ansi.dim("Plugins:")} ${ansi.dim("(none)")}\n\n`);
  } else {
    process.stdout.write(
      `${ansi.bold(`Plugins`)} ${ansi.dim(`(${report.plugins.length})`)}\n`,
    );
    for (const plugin of report.plugins) {
      const flag = plugin.active ? ansi.green("✓") : ansi.dim("✗");
      const sourceText = plugin.source
        ? `${ansi.dim(plugin.source)}${plugin.ref ? ansi.dim(`@${plugin.ref}`) : ""}`
        : ansi.dim("(local)");
      process.stdout.write(
        `  ${flag} ${plugin.name} ${ansi.dim(plugin.scope)} ${ansi.dim(`(${plugin.componentSummary})`)} ${sourceText}\n`,
      );
    }
    process.stdout.write("\n");
  }

  // ── Taps ─────────────────────────────────────────────────────────────────
  process.stdout.write(
    `${ansi.bold("Taps")} ${ansi.dim(`(${report.taps.length})`)}\n`,
  );
  for (const tap of report.taps) {
    const label = tap.builtin
      ? `${tap.name} ${ansi.dim("(built-in)")}`
      : tap.name;
    const typeLabel =
      tap.type === "http" ? ansi.yellow(" (http — deprecated)") : "";
    process.stdout.write(`  ${label}${typeLabel} ${ansi.dim(tap.url)}\n`);
  }
  process.stdout.write("\n");

  // ── Drift ────────────────────────────────────────────────────────────────
  if (report.drift) {
    if (report.drift.inSync) {
      process.stdout.write(
        `${ansi.green("✓")} Manifest in sync with installed state.\n`,
      );
    } else {
      const counts = countByKind(report.drift.items);
      process.stdout.write(
        `${ansi.yellow("Drift:")} ${counts.summary}. Run ${ansi.bold("skilltap sync")} for details.\n`,
      );
    }
  }
}

function countByKind(items: { kind: string }[]): { summary: string } {
  const c: Record<string, number> = {};
  for (const item of items) c[item.kind] = (c[item.kind] ?? 0) + 1;
  const parts: string[] = [];
  if (c["add"]) parts.push(`${c["add"]} to add`);
  if (c["remove"]) parts.push(`${c["remove"]} to remove`);
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
