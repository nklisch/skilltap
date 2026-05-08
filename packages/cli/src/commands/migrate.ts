import { runMigrate } from "@skilltap/core";
import { defineCommand } from "citty";
import { ansi } from "../ui/format";
import { tryFindProjectRoot } from "../ui/resolve";
import { createOutput } from "../output";

export default defineCommand({
  meta: {
    name: "migrate",
    description: "Migrate v1.0 setup to v2.0 (one-shot).",
  },
  args: {
    json: {
      type: "boolean",
      description: "Output as JSON",
      default: false,
    },
  },
  async run({ args }) {
    const out = createOutput({ json: args.json, quiet: false });
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
        out.error(result.error.message);
        if (result.error.hint)
          process.stderr.write(`${ansi.dim("hint:")} ${result.error.hint}\n`);
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
      });
      return;
    }

    if (report.alreadyMigrated) {
      process.stdout.write(
        `${ansi.green("✓")} Already on v2.0. Nothing to do.\n`,
      );
      return;
    }

    process.stdout.write(
      `\n${ansi.bold("skilltap migrate")} — v1.0 → v2.0\n\n`,
    );

    if (report.changes.written.length > 0) {
      process.stdout.write(`${ansi.green("Wrote:")}\n`);
      for (const path of report.changes.written) {
        process.stdout.write(`  ${ansi.green("+")} ${path}\n`);
      }
      process.stdout.write("\n");
    }

    if (report.changes.renamed.length > 0) {
      process.stdout.write(`${ansi.dim("Renamed:")}\n`);
      for (const { from, to } of report.changes.renamed) {
        process.stdout.write(`  ${ansi.dim(from)} → ${ansi.dim(to)}\n`);
      }
      process.stdout.write("\n");
    }

    if (report.warnings.length > 0) {
      process.stdout.write(`${ansi.yellow("Warnings:")}\n`);
      for (const warning of report.warnings) {
        process.stdout.write(`  ${ansi.yellow("!")} ${warning}\n`);
      }
      process.stdout.write("\n");
    }

    process.stdout.write(
      `${ansi.green("✓")} Migrated ${report.scopes.join(" and ") || "configuration"}. ` +
        `Run ${ansi.bold("skilltap doctor")} to verify.\n`,
    );
  },
});
