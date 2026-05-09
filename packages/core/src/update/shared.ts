import { mkdir } from "node:fs/promises";
import { dirname, join } from "node:path";
import { $ } from "bun";
import { skillInstallDir } from "../paths";
import type { InstalledSkill } from "../schemas/installed";
import type { StaticWarning } from "../security";
import { scanSemantic } from "../security/semantic";
import { wrapShell } from "../shell";
import { createAgentSymlinks, removeAgentSymlinks } from "../symlink";
import type { Result, UserError } from "../types";
import { ok } from "../types";
import type { UpdateOptions, UpdateResult } from "./types";

/** Decide whether the user wants to skip this update based on warnings and confirmation. */
export async function shouldSkipUpdate(
  warnings: StaticWarning[],
  options: UpdateOptions,
  skillName: string,
): Promise<boolean> {
  if (warnings.length > 0) {
    options.onShowWarnings?.(warnings, skillName);
    if (options.strict) return true;
    const confirmed = await options.onConfirm?.(skillName, true);
    if (confirmed === false) return true;
  } else if (!options.yes) {
    const confirmed = await options.onConfirm?.(skillName, false);
    if (confirmed === false) return true;
  }
  return false;
}

/** Re-copy a multi-skill's subdirectory from cache to install path after pull. */
export async function recopyMultiSkill(
  workDir: string,
  record: InstalledSkill,
  projectRoot?: string,
): Promise<Result<void, UserError>> {
  if (record.path === null) return ok(undefined);
  const skillSrc = join(workDir, record.path);
  const destDir = skillInstallDir(
    record.name,
    record.scope as "global" | "project",
    projectRoot,
  );
  const rmResult = await wrapShell(
    () => $`rm -rf ${destDir}`.quiet().then(() => undefined),
    `Failed to remove old skill directory '${record.name}'`,
  );
  if (!rmResult.ok) return rmResult;

  await mkdir(dirname(destDir), { recursive: true });

  return wrapShell(
    () => $`cp -r ${skillSrc} ${destDir}`.quiet().then(() => undefined),
    `Failed to copy updated skill '${record.name}'`,
    "Check disk space and permissions.",
  );
}

/** Remove and re-create agent symlinks for a skill (idempotent). Skips disabled skills. */
export async function refreshAgentSymlinks(
  record: InstalledSkill,
  projectRoot?: string,
): Promise<void> {
  if (record.also.length === 0) return;
  if (record.active === false) return;
  const scope = record.scope as "global" | "project";
  await removeAgentSymlinks(record.name, record.also, scope, projectRoot);
  const installDir = skillInstallDir(record.name, scope, projectRoot);
  await createAgentSymlinks(
    record.name,
    installDir,
    record.also,
    scope,
    projectRoot,
  );
}

/** Run semantic scan on a skill directory after an update. Returns whether to skip the skill. */
export async function runUpdateSemanticScan(
  installDir: string,
  skillName: string,
  options: UpdateOptions,
): Promise<boolean> {
  if (!options.semantic || !options.agent) return false;
  options.onSemanticScanStart?.(skillName);
  const semResult = await scanSemantic(installDir, options.agent, {
    threshold: options.threshold,
    onProgress: options.onSemanticProgress,
  });
  if (semResult.ok && semResult.value.length > 0) {
    options.onSemanticWarnings?.(semResult.value, skillName);
    if (options.strict) return true;
  }
  return false;
}

export function skipSkill(
  result: UpdateResult,
  options: UpdateOptions,
  name: string,
): Result<void, never> {
  result.skipped.push(name);
  options.onProgress?.(name, "skipped");
  return ok(undefined);
}

export function patchRecord(
  installed: { skills: InstalledSkill[] },
  record: InstalledSkill,
  updates: Partial<InstalledSkill>,
): void {
  const idx = installed.skills.indexOf(record);
  if (idx !== -1) {
    installed.skills[idx] = { ...record, ...updates };
  }
}

export type SkillGroup =
  | { type: "linked"; skill: InstalledSkill }
  | { type: "local"; skill: InstalledSkill }
  | { type: "npm"; skill: InstalledSkill }
  | { type: "git-standalone"; skill: InstalledSkill }
  | { type: "git-multi"; repo: string; skills: InstalledSkill[] };

/** Group skills by update strategy. Multi-skill records sharing a repo cache are grouped together. */
export function groupSkillsByRepo(skills: InstalledSkill[]): SkillGroup[] {
  const multiGroups = new Map<string, InstalledSkill[]>();
  const solo: SkillGroup[] = [];

  for (const skill of skills) {
    if (skill.scope === "linked") {
      solo.push({ type: "linked", skill });
      continue;
    }
    if (!skill.repo) {
      solo.push({ type: "local", skill });
      continue;
    }
    if (skill.repo.startsWith("npm:")) {
      solo.push({ type: "npm", skill });
      continue;
    }
    if (skill.path !== null) {
      const existing = multiGroups.get(skill.repo);
      if (existing) {
        existing.push(skill);
      } else {
        multiGroups.set(skill.repo, [skill]);
      }
    } else {
      solo.push({ type: "git-standalone", skill });
    }
  }

  const groups: SkillGroup[] = [...solo];
  for (const [repo, skills] of multiGroups) {
    groups.push({ type: "git-multi", repo, skills });
  }
  return groups;
}
