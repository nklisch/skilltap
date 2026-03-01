import { defineCommand } from "citty";
import { loadInstalled } from "@skilltap/core";
import type { InstalledSkill } from "@skilltap/core";
import { ansi, errorLine, table, termWidth, truncate } from "../ui/format";

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
    const result = await loadInstalled();
    if (!result.ok) {
      errorLine(result.error.message);
      process.exit(1);
    }

    let { skills } = result.value;

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
        "Run 'skilltap install <source>' to install a skill.\n",
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

      // Fixed column widths for Name, Ref, Source — description gets the rest
      const NAME_W = 20;
      const REF_W = 10;
      const SRC_W = 24;
      const PADDING = 2; // between each col
      const INDENT = 2; // leading indent added by table()
      const fixed = NAME_W + REF_W + SRC_W + PADDING * 3 + INDENT;
      const descW = Math.max(10, width - fixed - 4);

      const rows = section.map((s) => [
        truncate(s.name, NAME_W),
        truncate(s.ref ?? "—", REF_W),
        truncate(s.repo ?? "local", SRC_W),
        truncate(s.description, descW),
      ]);

      process.stdout.write(
        `${table(rows, { header: ["Name", "Ref", "Source", "Description"] })}\n`,
      );
    }

    printSection("Global", globalSkills);
    printSection("Project", projectSkills);
    printSection("Linked", linkedSkills);
    process.stdout.write("\n");
  },
});
