import type { DiscoveredSkill } from "@skilltap/core";
import { discoverSkills } from "@skilltap/core";
import { defineCommand } from "citty";
import { createOutput } from "../../output";
import { ansi, table, termWidth, truncate } from "../../ui/format";
import { tryFindProjectRoot } from "../../ui/resolve";

export default defineCommand({
  meta: {
    name: "skills",
    description: "Manage installed skills",
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
    unmanaged: {
      type: "boolean",
      description: "Show only unmanaged skills",
      default: false,
    },
    json: { type: "boolean", description: "Output as JSON", default: false },
    disabled: {
      type: "boolean",
      description: "Show only disabled skills",
      default: false,
    },
    active: {
      type: "boolean",
      description: "Show only active skills",
      default: false,
    },
  },
  subCommands: {
    info: () => import("./info").then((m) => m.default),
    remove: () => import("./remove").then((m) => m.default),
    link: () => import("./link").then((m) => m.default),
    unlink: () => import("./unlink").then((m) => m.default),
    adopt: () => import("./adopt").then((m) => m.default),
    move: () => import("./move").then((m) => m.default),
    disable: () => import("./toggle").then((m) => m.disableCommand),
    enable: () => import("./toggle").then((m) => m.enableCommand),
  },
  async run({ args }) {
    // If a subcommand was matched, citty still calls this run — bail out
    if ((args._ as string[])?.length > 0) return;

    const out = createOutput({ json: args.json, quiet: false });
    const projectRoot = await tryFindProjectRoot();

    const discoverOpts = args.global
      ? { global: true as const, projectRoot, unmanagedOnly: args.unmanaged }
      : args.project
        ? { project: true as const, projectRoot, unmanagedOnly: args.unmanaged }
        : { projectRoot, unmanagedOnly: args.unmanaged };

    const discoverResult = await discoverSkills(discoverOpts);

    if (!discoverResult.ok) {
      out.error(discoverResult.error.message);
      process.exit(1);
    }

    let { skills } = discoverResult.value;

    if (args.disabled) {
      skills = skills.filter((s) => s.record?.active === false);
    } else if (args.active) {
      skills = skills.filter((s) => s.record?.active !== false);
    }

    if (args.json) {
      out.json(skills);
      return;
    }

    if (skills.length === 0) {
      out.info("No skills found.");
      out.info("Run 'skilltap install <source>' to get started.");
      return;
    }

    // Interactive table output
    const width = termWidth();

    // Split into managed/linked and unmanaged
    const managed = skills.filter((s) => s.managed);
    const unmanaged = skills.filter((s) => !s.managed);

    function printManagedSection(label: string, section: DiscoveredSkill[]) {
      if (section.length === 0) return;
      const count = section.length;
      process.stdout.write(
        `\n${ansi.bold(label)} (${count} ${count === 1 ? "skill" : "skills"})\n`,
      );

      const NAME_W = width < 60 ? 15 : 20;
      const _STATUS_W = 10;
      const AGENTS_W = width < 60 ? 16 : 20;
      const SRC_W = width < 60 ? 16 : 24;

      const rows = section.map((s) => {
        const isDisabled = s.record?.active === false;
        const isLinked = s.record?.scope === "linked";
        const statusLabel = isDisabled
          ? ansi.dim("disabled")
          : isLinked
            ? ansi.cyan("linked")
            : ansi.green("managed");
        const agents =
          s.record?.also && s.record.also.length > 0
            ? s.record.also.join(", ")
            : "—";
        const source = s.record?.repo ?? "local";
        return [
          truncate(s.name, NAME_W),
          statusLabel,
          truncate(agents, AGENTS_W),
          truncate(source, SRC_W),
        ];
      });

      process.stdout.write(
        `${table(rows, { header: ["Name", "Status", "Agents", "Source"] })}\n`,
      );
    }

    function printUnmanagedSection(label: string, section: DiscoveredSkill[]) {
      if (section.length === 0) return;
      const count = section.length;
      process.stdout.write(
        `\n${ansi.bold(label)} (${count} ${count === 1 ? "skill" : "skills"})\n`,
      );

      const NAME_W = width < 60 ? 15 : 20;
      const SRC_W = width < 60 ? 20 : 32;

      const rows = section.map((s) => [
        truncate(s.name, NAME_W),
        ansi.yellow("unmanaged"),
        truncate(s.gitRemote ?? "(local)", SRC_W),
      ]);

      process.stdout.write(
        `${table(rows, { header: ["Name", "Status", "Source"] })}\n`,
      );
    }

    // Group managed by scope
    const globalManaged = managed.filter((s) => {
      const loc = s.locations[0];
      return loc?.source.scope === "global";
    });
    const projectManaged = managed.filter((s) => {
      const loc = s.locations[0];
      return loc?.source.scope === "project";
    });

    // Group unmanaged by scope
    const globalUnmanaged = unmanaged.filter((s) => {
      const loc = s.locations[0];
      return loc?.source.scope === "global";
    });
    const projectUnmanaged = unmanaged.filter((s) => {
      const loc = s.locations[0];
      return loc?.source.scope === "project";
    });

    printManagedSection("Global (.agents/skills/)", globalManaged);
    printUnmanagedSection("Global — unmanaged", globalUnmanaged);
    printManagedSection("Project (.agents/skills/)", projectManaged);
    printUnmanagedSection("Project — unmanaged", projectUnmanaged);

    process.stdout.write("\n");
  },
});
