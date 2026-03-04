import type { InstalledSkill } from "@skilltap/core";
import { findProjectRoot, loadInstalled } from "@skilltap/core";
import { defineCommand } from "citty";
import { ansi, errorLine, table, termWidth, truncate } from "../ui/format";
import { formatTrustTier } from "../ui/trust";

export default defineCommand({
  meta: {
    name: "list",
    description: "List installed skills",
  },
  args: {
    global: {
      type: "boolean",
      description: "Show only global skills",
      default: false,
    },
    project: {
      type: "boolean",
      description: "Show only project skills",
      default: false,
    },
    json: {
      type: "boolean",
      description: "Output as JSON",
      default: false,
    },
  },
  async run({ args }) {
    const globalResult = await loadInstalled();
    if (!globalResult.ok) {
      errorLine(globalResult.error.message);
      process.exit(1);
    }

    const projectRoot = await findProjectRoot().catch(() => undefined);
    const projectResult = projectRoot ? await loadInstalled(projectRoot) : null;

    let skills: InstalledSkill[] = [
      ...globalResult.value.skills,
      ...(projectResult?.ok ? projectResult.value.skills : []),
    ];

    if (args.json) {
      process.stdout.write(`${JSON.stringify(skills, null, 2)}\n`);
      return;
    }

    if (args.global) {
      skills = skills.filter((s) => s.scope === "global");
    } else if (args.project) {
      skills = skills.filter((s) => s.scope === "project");
    }

    if (skills.length === 0) {
      process.stdout.write("No skills installed.\n");
      process.stdout.write(
        "Run 'skilltap install <source>' to get started.\n",
      );
      return;
    }

    const globalSkills = skills.filter((s) => s.scope === "global");
    const projectSkills = skills.filter((s) => s.scope === "project");
    const linkedSkills = skills.filter((s) => s.scope === "linked");

    const width = termWidth();

    function printSection(label: string, section: InstalledSkill[]) {
      if (section.length === 0) return;
      const count = section.length;
      process.stdout.write(
        `\n${ansi.bold(label)} (${count} ${count === 1 ? "skill" : "skills"})\n`,
      );

      // Fixed column widths for Name, Ref, Source, Trust — description gets the rest
      const NAME_W = width < 60 ? 15 : 20;
      const REF_W = width < 60 ? 8 : 10;
      const SRC_W = width < 60 ? 16 : 24;
      const TRUST_W = width < 60 ? 12 : 15;
      const PADDING = 2; // between each col
      const INDENT = 2; // leading indent added by table()
      const fixed = NAME_W + REF_W + SRC_W + TRUST_W + PADDING * 4 + INDENT;
      const descW = Math.max(10, width - fixed - 4);

      const rows = section.map((s) => [
        truncate(s.name, NAME_W),
        truncate(s.ref ?? "—", REF_W),
        truncate(s.repo ?? "local", SRC_W),
        formatTrustTier(s.trust),
        truncate(s.description, descW),
      ]);

      process.stdout.write(
        `${table(rows, { header: ["Name", "Ref", "Source", "Trust", "Description"] })}\n`,
      );
    }

    printSection("Global", globalSkills);
    printSection("Project", projectSkills);
    printSection("Linked", linkedSkills);
    process.stdout.write("\n");
  },
});
