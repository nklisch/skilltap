import { lstat } from "node:fs/promises";
import { join } from "node:path";
import { AGENT_PATHS, globalBase } from "@skilltap/core";
import { defineCommand } from "citty";
import { ansi } from "../ui/format";
import { getInstalledSkillOrExit } from "../ui/resolve";

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
  },
  async run({ args }) {
    const skill = await getInstalledSkillOrExit(args.name, {
      notFoundHint: `Run 'skilltap find ${args.name}' to search`,
    });

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
  },
});
