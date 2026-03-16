import { readdir } from "node:fs/promises";
import { join } from "node:path";
import { loadInstalled, saveInstalled } from "../../config";
import { globalBase, resolvedDirExists } from "../../fs";
import { skillDisabledDir, skillInstallDir } from "../../paths";
import type { InstalledJson } from "../../schemas/installed";
import type { DoctorCheck, DoctorIssue } from "../types";

export async function checkSkills(installed: InstalledJson, projectRoot?: string): Promise<DoctorCheck> {
  const issues: DoctorIssue[] = [];
  const globalTracked = new Set<string>();
  const projectTracked = new Set<string>();

  for (const skill of installed.skills) {
    if (skill.scope === "project") {
      projectTracked.add(skill.name);
    } else if (skill.scope === "linked") {
      if (projectRoot && skill.path?.startsWith(join(projectRoot, ".agents"))) {
        projectTracked.add(skill.name);
      } else {
        globalTracked.add(skill.name);
      }
    } else {
      globalTracked.add(skill.name);
    }

    if (skill.scope === "linked") {
      if (skill.path && !(await resolvedDirExists(skill.path))) {
        const skillName = skill.name;
        issues.push({
          message: `${skillName}: symlink target ${skill.path} does not exist`,
          fixable: true,
          fixDescription: `removed from installed.json`,
          fix: async () => {
            const r = await loadInstalled();
            if (!r.ok) return;
            await saveInstalled({
              ...r.value,
              skills: r.value.skills.filter((s) => s.name !== skillName),
            });
          },
        });
      }
      continue;
    }

    const isProject = skill.scope === "project" && !!projectRoot;
    const installDir = skill.active === false
      ? (isProject ? skillDisabledDir(skill.name, "project", projectRoot) : skillDisabledDir(skill.name, "global"))
      : (isProject ? skillInstallDir(skill.name, "project", projectRoot) : skillInstallDir(skill.name, "global"));

    if (!(await resolvedDirExists(installDir))) {
      const skillName = skill.name;
      const skillScope = skill.scope as "global" | "project";
      const capturedRoot = projectRoot;
      issues.push({
        message: `${skillName}: recorded in installed.json but directory missing at ${installDir}`,
        fixable: true,
        fixDescription: `removed from installed.json`,
        fix: async () => {
          const effectiveRoot = skillScope === "project" ? capturedRoot : undefined;
          const r = await loadInstalled(effectiveRoot);
          if (!r.ok) return;
          await saveInstalled(
            { ...r.value, skills: r.value.skills.filter((s) => s.name !== skillName) },
            effectiveRoot,
          );
        },
      });
    }
  }

  // Global orphan scan
  const globalSkillsDir = join(globalBase(), ".agents", "skills");
  if (await resolvedDirExists(globalSkillsDir)) {
    try {
      const entries = await readdir(globalSkillsDir, { withFileTypes: true });
      for (const entry of entries) {
        if (!entry.isDirectory() && !entry.isSymbolicLink()) continue;
        if (entry.name === ".disabled") continue;
        if (!globalTracked.has(entry.name)) {
          issues.push({
            message: `${entry.name}: directory exists at ${join(globalSkillsDir, entry.name)} but not tracked in installed.json`,
            fixable: false,
          });
        }
      }
    } catch {
      // ignore
    }
  }

  // Project orphan scan (only when there are project-tracked skills)
  if (projectRoot && projectTracked.size > 0) {
    const projectSkillsDir = join(projectRoot, ".agents", "skills");
    if (await resolvedDirExists(projectSkillsDir)) {
      try {
        const entries = await readdir(projectSkillsDir, { withFileTypes: true });
        for (const entry of entries) {
          if (!entry.isDirectory() && !entry.isSymbolicLink()) continue;
          if (entry.name === ".disabled") continue;
          if (!projectTracked.has(entry.name)) {
            issues.push({
              message: `${entry.name}: directory exists at ${join(projectSkillsDir, entry.name)} but not tracked in installed.json`,
              fixable: false,
            });
          }
        }
      } catch {
        // ignore
      }
    }
  }

  const total = installed.skills.length;
  const missing = issues.filter((i) => i.fixable).length;
  const onDisk = total - missing;

  if (issues.length === 0) {
    return {
      name: "skills",
      status: "pass",
      detail: `${total} installed, ${total} on disk`,
    };
  }
  return {
    name: "skills",
    status: "warn",
    detail: `${total} installed, ${onDisk} on disk`,
    issues,
  };
}
