import { runMigrate } from "@skilltap/core";
import { defineCommand } from "citty";
import { ansi } from "../ui/format";
import { tryFindProjectRoot } from "../ui/resolve";
import { setupOutput } from "../ui/setup";

export default defineCommand({
  meta: {
    name: "migrate",
    description: "Migrate legacy config and state to current format (one-shot).",
  },
  args: {
    json: {
      type: "boolean",
      description: "Output as JSON",
      default: false,
    },
  },
  async run({ args }) {
    const out = setupOutput(args);
    const useJson = args.json as boolean;
    const projectRoot = await tryFindProjectRoot();

    const result = await runMigrate({ projectRoot });

    if (!result.ok) {
      if (useJson) {
        out.json({
          ok: false,
          error: result.error.message,
          hint: result.error.hint,
        });
      } else {
        out.error(result.error.message, result.error.hint);
      }
      process.exit(1);
    }

    const report = result.value;

    if (useJson) {
      out.json({
        ok: true,
        alreadyMigrated: report.alreadyMigrated,
        scopes: report.scopes,
        changes: report.changes,
        warnings: report.warnings,
        doctorReport: report.doctorReport,
      });
      return;
    }

    if (report.alreadyMigrated) {
      out.raw(`${ansi.green("✓")} Already migrated. Nothing to do.\n`);
      return;
    }

    out.raw(`\n${ansi.bold("skilltap migrate")} — legacy → state.json\n\n`);

    if (report.changes.written.length > 0) {
      out.raw(`${ansi.green("Wrote:")}\n`);
      for (const path of report.changes.written) {
        out.raw(`  ${ansi.green("+")} ${path}\n`);
      }
      out.raw("\n");
    }

    if (report.changes.renamed.length > 0) {
      out.raw(`${ansi.dim("Renamed:")}\n`);
      for (const { from, to } of report.changes.renamed) {
        out.raw(`  ${ansi.dim(from)} → ${ansi.dim(to)}\n`);
      }
      out.raw("\n");
    }

    if (report.warnings.length > 0) {
      out.raw(`${ansi.yellow("Behavior changes:")}\n`);
      for (const warning of report.warnings) {
        out.raw(`  ${ansi.yellow("!")} Behavior change: ${warning}\n`);
      }
      out.raw("\n");
    }

    out.raw(
      `${ansi.green("✓")} Migrated ${report.scopes.join(" and ") || "configuration"}.\n`,
    );

    if (report.doctorReport) {
      const failed = report.doctorReport.checks.filter(
        (c) => c.status === "fail",
      );
      const warned = report.doctorReport.checks.filter(
        (c) => c.status === "warn",
      );
      if (failed.length === 0 && warned.length === 0) {
        out.raw(`${ansi.green("✓")} Doctor: all checks passed.\n`);
      } else {
        if (failed.length > 0) {
          out.raw(
            `${ansi.red("✗")} Doctor: ${failed.length} check(s) failed — your environment may have other issues unrelated to migration. Run ${ansi.bold("skilltap doctor")} for details.\n`,
          );
        }
        if (warned.length > 0) {
          out.raw(
            `${ansi.yellow("!")} Doctor: ${warned.length} check(s) warning — run ${ansi.bold("skilltap doctor")} for details.\n`,
          );
        }
      }
    }
  },
});
