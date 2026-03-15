import { defineCommand } from "citty";
import { discoverSkills, findProjectRoot, loadConfig } from "@skilltap/core";
import type { DiscoveredSkill } from "@skilltap/core";
import { ansi, table, termWidth, truncate } from "../../ui/format";

export default defineCommand({
  meta: {
    name: "skills",
    description: "Manage installed skills",
  },
  args: {
    global: { type: "boolean", description: "Show only global skills", default: false },
    project: { type: "boolean", description: "Show only project skills", default: false },
    unmanaged: { type: "boolean", description: "Show only unmanaged skills", default: false },
    json: { type: "boolean", description: "Output as JSON", default: false },
  },
  subCommands: {
    info: () => import("./info").then((m) => m.default),
    remove: () => import("./remove").then((m) => m.default),
    link: () => import("./link").then((m) => m.default),
    unlink: () => import("./unlink").then((m) => m.default),
    adopt: () => import("./adopt").then((m) => m.default),
    move: () => import("./move").then((m) => m.default),
  },
  async run({ args }) {
    // If a subcommand was matched, citty still calls this run — bail out
    if ((args._ as string[])?.length > 0) return;

    const configResult = await loadConfig();
    const agentMode = configResult.ok && configResult.value["agent-mode"].enabled;

    const projectRoot = await findProjectRoot().catch(() => undefined);

    const discoverOpts = args.global
      ? { global: true as const, projectRoot, unmanagedOnly: args.unmanaged }
      : args.project
        ? { project: true as const, projectRoot, unmanagedOnly: args.unmanaged }
        : { projectRoot, unmanagedOnly: args.unmanaged };

    const discoverResult = await discoverSkills(discoverOpts);

    if (!discoverResult.ok) {
      process.stderr.write(`error: ${discoverResult.error.message}\n`);
      process.exit(1);
    }

    const { skills } = discoverResult.value;

    if (args.json) {
      process.stdout.write(`${JSON.stringify(skills, null, 2)}\n`);
      return;
    }

    if (skills.length === 0) {
      process.stdout.write("No skills found.\n");
      process.stdout.write("Run 'skilltap install <source>' to get started.\n");
      return;
    }

    if (agentMode) {
      // Plain text format for agent mode
      for (const skill of skills) {
        const primaryLoc = skill.locations[0];
        if (!primaryLoc) continue;
        const scope = primaryLoc.source.scope.toUpperCase();
        const status = skill.managed
          ? skill.record?.scope === "linked" ? "linked" : "managed"
          : "unmanaged";
        const agent =
          primaryLoc.source.type === "agent-specific"
            ? primaryLoc.source.agent.toUpperCase().replace(/-/g, "_")
            : "AGENTS";
        const extra = skill.managed && skill.record
          ? skill.record.scope !== "linked"
            ? `source=${skill.record.repo ?? "local"}`
            : `path=${skill.record.path ?? ""}`
          : skill.gitRemote
            ? `remote=${skill.gitRemote}`
            : "";
        process.stdout.write(
          `${scope} ${status} ${skill.name}${extra ? ` ${extra}` : ""}\n`,
        );
      }
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
      const STATUS_W = 10;
      const AGENTS_W = width < 60 ? 16 : 20;
      const SRC_W = width < 60 ? 16 : 24;

      const rows = section.map((s) => {
        const isLinked = s.record?.scope === "linked";
        const statusLabel = isLinked
          ? ansi.cyan("linked")
          : ansi.green("managed");
        const agents = s.record?.also && s.record.also.length > 0
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
