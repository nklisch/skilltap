import type { DoctorCheck, DoctorIssue, Output } from "@skilltap/core";
import { runDoctor } from "@skilltap/core";
import { defineCommand } from "citty";
import { ansi } from "../ui/format";
import { tryFindProjectRoot } from "../ui/resolve";
import { setupOutput } from "../ui/setup";

export default defineCommand({
  meta: {
    name: "doctor",
    description: "Check skilltap environment and state",
  },
  args: {
    json: {
      type: "boolean",
      description: "Output as JSON",
      default: false,
    },
    fix: {
      type: "boolean",
      description: "Auto-fix issues where possible",
      default: false,
    },
  },
  async run({ args }) {
    const out = setupOutput(args);
    const useJson = args.json as boolean;
    const fix = args.fix as boolean;

    if (useJson) {
      await runJson(out, fix);
      return;
    }

    await runInteractive(out, fix);
  },
});

// ─── Interactive output ───────────────────────────────────────────────────────

function statusSymbol(status: DoctorCheck["status"]): string {
  switch (status) {
    case "pass":
      return ansi.green("◇");
    case "warn":
      return ansi.yellow("⚠");
    case "fail":
      return ansi.red("✗");
  }
}

function issuePrefix(): string {
  return ansi.dim("│");
}

function printCheck(out: Output, check: DoctorCheck, fix: boolean): void {
  const sym = statusSymbol(check.status);
  const detail = check.detail ? `: ${check.detail}` : "";
  const suffix = check.status === "pass" ? ` ${ansi.green("✓")}` : "";
  out.raw(`${sym} ${check.name}${detail}${suffix}\n`);

  if (check.issues) {
    for (const issue of check.issues) {
      printIssue(out, issue, fix);
    }
  }

  if (check.info) {
    for (const line of check.info) {
      out.raw(`${issuePrefix()}  ${ansi.dim(line)}\n`);
    }
  }
}

function printIssue(out: Output, issue: DoctorIssue, fix: boolean): void {
  const prefix = issuePrefix();
  if (fix && issue.fixed) {
    const fixText = issue.fixDescription
      ? ` — ${issue.fixDescription} ${ansi.green("✓")}`
      : ` ${ansi.green("✓")}`;
    out.raw(`${prefix}  ${issue.message}${fixText}\n`);
  } else if (fix && issue.fixable && !issue.fixed) {
    out.raw(`${prefix}  ${issue.message} ${ansi.red("(fix failed)")}\n`);
  } else if (fix && !issue.fixable) {
    out.raw(
      `${prefix}  ${ansi.dim("(cannot auto-fix — ")}${issue.message}${ansi.dim(")")}\n`,
    );
  } else {
    out.raw(`${prefix}  ${issue.message}\n`);
  }
}

async function runInteractive(out: Output, fix: boolean): Promise<void> {
  out.raw(`\n${ansi.dim("┌")} skilltap doctor\n${ansi.dim("│")}\n`);

  const projectRoot = await tryFindProjectRoot();
  const result = await runDoctor({
    fix,
    projectRoot,
    onCheck: (check) => printCheck(out, check, fix),
  });

  out.raw(`${ansi.dim("│")}\n`);

  // Summary
  const allIssues = result.checks.flatMap((c) => c.issues ?? []);
  const totalIssues = allIssues.length;
  const fixedCount = allIssues.filter((i) => i.fixed).length;
  const unfixable = allIssues.filter((i) => !i.fixable && !i.fixed).length;
  const hasFailures = result.checks.some(
    (c) => c.status === "fail" && !c.fixed,
  );

  if (totalIssues === 0) {
    out.raw(`${ansi.dim("└")} ${ansi.green("✓")} Everything looks good!\n\n`);
    process.exit(0);
  }

  if (fix) {
    if (fixedCount > 0 && unfixable === 0 && !hasFailures) {
      out.raw(
        `${ansi.dim("└")} ${ansi.green("✓")} Fixed ${fixedCount} ${fixedCount === 1 ? "issue" : "issues"}.\n\n`,
      );
    } else if (fixedCount > 0) {
      const remaining = totalIssues - fixedCount;
      out.raw(
        `${ansi.dim("└")} ${ansi.yellow("⚠")} Fixed ${fixedCount} of ${totalIssues} ${totalIssues === 1 ? "issue" : "issues"}. ${remaining} ${remaining === 1 ? "requires" : "require"} manual action.\n\n`,
      );
    } else {
      out.raw(
        `${ansi.dim("└")} ${ansi.yellow("⚠")} ${totalIssues} ${totalIssues === 1 ? "issue" : "issues"} found. None could be auto-fixed.\n\n`,
      );
    }
  } else {
    out.raw(
      `${ansi.dim("└")} ${ansi.yellow("⚠")} ${totalIssues} ${totalIssues === 1 ? "issue" : "issues"} found. Run '${ansi.bold("skilltap doctor --fix")}' to auto-fix where possible.\n\n`,
    );
  }

  if (hasFailures) process.exit(1);
}

// ─── JSON output ──────────────────────────────────────────────────────────────

async function runJson(out: Output, fix: boolean): Promise<void> {
  const projectRoot = await tryFindProjectRoot();
  const result = await runDoctor({ fix, projectRoot });

  const output = {
    ok: result.ok,
    checks: result.checks.map((c) => ({
      name: c.name,
      status: c.fixed ? ("pass" as const) : c.status,
      ...(c.detail ? { detail: c.detail } : {}),
      ...(c.info && c.info.length > 0 ? { info: c.info } : {}),
      ...(c.fixed ? { fixed: true } : {}),
      ...(c.fixDescription ? { fixDescription: c.fixDescription } : {}),
      ...(c.issues
        ? {
            issues: c.issues.map((i) => ({
              message: i.message,
              fixable: i.fixable,
              ...(i.fixed !== undefined ? { fixed: i.fixed } : {}),
              ...(i.fixDescription ? { fixDescription: i.fixDescription } : {}),
            })),
          }
        : {}),
    })),
  };

  out.json(output);

  if (!result.ok) process.exit(1);
}
