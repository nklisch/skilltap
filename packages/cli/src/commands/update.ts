import { confirm, isCancel, log } from "@clack/prompts";
import { type StaticWarning, updateSkill } from "@skilltap/core";
import { defineCommand } from "citty";
import {
  ansi,
  errorLine,
  formatDiffFileLine,
  formatDiffStatSummary,
  formatShaChange,
  successLine,
} from "../ui/format";
import { printWarnings } from "../ui/scan";

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
  },
  async run({ args }) {
    const name = args.name as string | undefined;

    const result = await updateSkill({
      name,
      yes: args.yes,
      strict: args.strict,

      onProgress(skillName, status) {
        if (status === "checking") {
          log.step(`Checking ${ansi.bold(skillName)}...`);
        } else if (status === "upToDate") {
          log.info("Already up to date.");
        } else if (status === "linked") {
          log.info("Skipped (linked).");
        }
        // "updated" and "skipped" are logged after the fact with context
      },

      onDiff(skillName, stat, fromSha, toSha) {
        const shaChange = formatShaChange(fromSha, toSha);
        const statSummary = formatDiffStatSummary(stat);
        log.info(`${shaChange} ${ansi.dim(statSummary)}`);
        for (const file of stat.files) {
          process.stdout.write(`${formatDiffFileLine(file)}\n`);
        }
      },

      onShowWarnings(warnings: StaticWarning[], skillName: string) {
        printWarnings(warnings, skillName);
        if (args.strict) {
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
    });

    if (!result.ok) {
      errorLine(result.error.message, result.error.hint);
      process.exit(1);
    }

    const { updated, skipped, upToDate } = result.value;

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
  },
});
