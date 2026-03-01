import { lstat } from "node:fs/promises";
import { join } from "node:path";
import { AGENT_PATHS, globalBase, loadInstalled, loadTaps } from "@skilltap/core";
import { defineCommand } from "citty";
import { ansi, errorLine } from "../ui/format";

export default defineCommand({
  meta: {
    name: "info",
    description: "Show details about an installed or available skill",
  },
  args: {
    name: {
      type: "positional",
      description: "Skill name",
      required: true,
    },
    json: {
      type: "boolean",
      description: "Output as JSON",
      default: false,
    },
  },
  async run({ args }) {
    // Try installed first
    const installedResult = await loadInstalled();
    if (!installedResult.ok) {
      errorLine(installedResult.error.message);
      process.exit(1);
    }

    const skill = installedResult.value.skills.find((s) => s.name === args.name);

    if (skill) {
      if (args.json) {
        process.stdout.write(`${JSON.stringify(skill, null, 2)}\n`);
        return;
      }

      const base = skill.scope === "project" ? process.cwd() : globalBase();
      const skillPath = join(base, ".agents", "skills", skill.name);

      const agentStatus = await Promise.all(
        Object.entries(AGENT_PATHS).map(async ([agent, dir]) => {
          const path = join(base, dir, skill.name);
          const exists = await lstat(path)
            .then(() => true)
            .catch(() => false);
          return { agent, exists };
        }),
      );

      const activeAgents = agentStatus
        .filter((a) => a.exists)
        .map((a) => a.agent);

      const rows = [
        ["name:", ansi.bold(skill.name)],
        ["description:", skill.description || "—"],
        ["scope:", skill.scope],
        ["source:", skill.repo ?? "local"],
        ["ref:", skill.ref ?? "—"],
        ["sha:", skill.sha ? skill.sha.slice(0, 7) : "—"],
        ["path:", skillPath],
        ["agents:", activeAgents.length > 0 ? activeAgents.join(", ") : "none"],
        ["installed:", skill.installedAt],
        ["updated:", skill.updatedAt],
      ];

      for (const [key, val] of rows) {
        process.stdout.write(`${ansi.dim(key.padEnd(13))} ${val}\n`);
      }
      return;
    }

    // Not installed — check taps
    const tapsResult = await loadTaps();
    if (tapsResult.ok) {
      const tapEntry = tapsResult.value.find((e) => e.skill.name === args.name);
      if (tapEntry) {
        if (args.json) {
          process.stdout.write(
            `${JSON.stringify({ ...tapEntry.skill, tap: tapEntry.tapName, status: "available" }, null, 2)}\n`,
          );
          return;
        }

        const rows = [
          ["name:", ansi.bold(tapEntry.skill.name)],
          ["description:", tapEntry.skill.description || "—"],
          ["status:", ansi.dim("(available)")],
          ["tap:", tapEntry.tapName],
          ["source:", tapEntry.skill.repo],
          ["tags:", tapEntry.skill.tags.length > 0 ? tapEntry.skill.tags.join(", ") : "—"],
        ];

        for (const [key, val] of rows) {
          process.stdout.write(`${ansi.dim(key.padEnd(13))} ${val}\n`);
        }
        process.stdout.write(
          `\nRun 'skilltap install ${args.name}' to install.\n`,
        );
        return;
      }
    }

    errorLine(
      `Skill '${args.name}' is not installed`,
      `Run 'skilltap find ${args.name}' to search`,
    );
    process.exit(1);
  },
});
