import { mkdir, readlink, symlink, unlink } from "node:fs/promises";
import { join } from "node:path";
import { globalBase, isSymlinkAt, resolvedDirExists } from "../../fs";
import { skillInstallDir } from "../../paths";
import type { InstalledJson } from "../../schemas/installed";
import { AGENT_PATHS } from "../../symlink";
import type { DoctorCheck, DoctorIssue } from "../types";

export async function checkSymlinks(installed: InstalledJson, projectRoot?: string): Promise<DoctorCheck> {
  const issues: DoctorIssue[] = [];
  let total = 0;
  let valid = 0;

  for (const skill of installed.skills) {
    if (skill.also.length === 0) continue;

    const isLinked = skill.scope === "linked";
    const isProject =
      (skill.scope === "project" && !!projectRoot) ||
      (isLinked && !!projectRoot && !!skill.path?.startsWith(join(projectRoot, ".agents")));
    const expectedTarget = isLinked
      ? (skill.path ?? skillInstallDir(skill.name, "global"))
      : isProject
        ? skillInstallDir(skill.name, "project", projectRoot)
        : skillInstallDir(skill.name, "global");
    const base = isProject ? projectRoot! : globalBase();

    for (const agent of skill.also) {
      const agentRelDir = AGENT_PATHS[agent];
      if (!agentRelDir) continue;

      const linkPath = join(base, agentRelDir, skill.name);
      total++;

      const isLink = await isSymlinkAt(linkPath);
      if (!isLink) {
        const skillExists = await resolvedDirExists(expectedTarget);
        const fixDesc = skillExists
          ? "recreated symlink"
          : "removed (skill no longer installed)";
        issues.push({
          message: `${skill.name}: missing symlink at ${linkPath}`,
          fixable: true,
          fixDescription: fixDesc,
          fix: skillExists
            ? async () => {
                await mkdir(join(linkPath, ".."), { recursive: true });
                await symlink(expectedTarget, linkPath, "dir").catch(() => {});
              }
            : async () => {
                // Nothing to do — orphan record cleanup handled by checkSkills fix
              },
        });
        continue;
      }

      let target: string | null = null;
      try {
        target = await readlink(linkPath);
      } catch {
        // ignore
      }

      if (target !== expectedTarget) {
        issues.push({
          message: `${skill.name}: symlink at ${linkPath} points to wrong target`,
          fixable: true,
          fixDescription: "recreated symlink",
          fix: async () => {
            await unlink(linkPath).catch(() => {});
            await mkdir(join(linkPath, ".."), { recursive: true });
            await symlink(expectedTarget, linkPath, "dir").catch(() => {});
          },
        });
      } else {
        valid++;
      }
    }
  }

  if (issues.length === 0) {
    return {
      name: "symlinks",
      status: "pass",
      detail: `${total} symlinks, ${valid} valid`,
    };
  }
  return {
    name: "symlinks",
    status: "warn",
    detail: `${total} symlinks, ${valid} valid`,
    issues,
  };
}
