import { lstat } from "node:fs/promises";
import { join } from "node:path";
import { AGENT_PATHS, findProjectRoot, globalBase, loadInstalled, loadTaps } from "@skilltap/core";
import { defineCommand } from "citty";
import { ansi, errorLine } from "../ui/format";
import { formatTrustLabel, formatTrustTier } from "../ui/trust";

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
    // Try installed first (global + project)
    const globalInstalledResult = await loadInstalled();
    if (!globalInstalledResult.ok) {
      errorLine(globalInstalledResult.error.message);
      process.exit(1);
    }

    const projectRoot = await findProjectRoot().catch(() => undefined);
    const projectInstalledResult = projectRoot ? await loadInstalled(projectRoot) : null;

    const allSkills = [
      ...globalInstalledResult.value.skills,
      ...(projectInstalledResult?.ok ? projectInstalledResult.value.skills : []),
    ];

    const skill = allSkills.find((s) => s.name === args.name);

    if (skill) {
      if (args.json) {
        process.stdout.write(`${JSON.stringify(skill, null, 2)}\n`);
        return;
      }

      const base = skill.scope === "project" ? (projectRoot ?? process.cwd()) : globalBase();
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

      const rows: [string, string][] = [
        ["name:", ansi.bold(skill.name)],
        ["description:", skill.description || "—"],
        ["scope:", skill.scope],
        ["source:", skill.repo ?? "local"],
        ["ref:", skill.ref ?? "—"],
        ["sha:", skill.sha ? skill.sha.slice(0, 7) : "—"],
        ["trust:", skill.trust ? formatTrustLabel(skill.trust) : ansi.dim("○ unverified")],
        ["path:", skillPath],
        ["agents:", activeAgents.length > 0 ? activeAgents.join(", ") : "none"],
        ["installed:", skill.installedAt],
        ["updated:", skill.updatedAt],
      ];
      // Append provenance details when available
      if (skill.trust?.tier === "provenance") {
        if (skill.trust.npm) {
          rows.push(["  source:", skill.trust.npm.sourceRepo]);
          if (skill.trust.npm.buildWorkflow)
            rows.push(["  build:", skill.trust.npm.buildWorkflow]);
          if (skill.trust.npm.transparency)
            rows.push(["  log:", skill.trust.npm.transparency]);
        } else if (skill.trust.github) {
          rows.push([
            "  repo:",
            `${skill.trust.github.owner}/${skill.trust.github.repo}`,
          ]);
          if (skill.trust.github.workflow)
            rows.push(["  build:", skill.trust.github.workflow]);
        }
      }

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

        const tapTrust = tapEntry.skill.trust?.verified
          ? ansi.dim("◆ verified by tap")
          : undefined;
        const rows = [
          ["name:", ansi.bold(tapEntry.skill.name)],
          ["description:", tapEntry.skill.description || "—"],
          ["status:", ansi.dim("(available)")],
          ["tap:", tapEntry.tapName],
          ["source:", tapEntry.skill.repo],
          ["tags:", tapEntry.skill.tags.length > 0 ? tapEntry.skill.tags.join(", ") : "—"],
          ...(tapTrust ? [["trust:", tapTrust] as [string, string]] : []),
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
