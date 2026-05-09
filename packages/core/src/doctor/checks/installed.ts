import { copyFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { z } from "zod/v4";
import { getConfigDir } from "../../dirs";
import { fileExists } from "../../fs";
import type { InstalledSkill } from "../../schemas/installed";
import { LegacyInstalledJsonSchema } from "../../migrate/legacy-schemas";
import type { LegacyInstalledJson } from "../../migrate/legacy-schemas";
import { loadState } from "../../state/load";
import type { DoctorCheck, DoctorIssue } from "../types";

async function readInstalledFile(
  file: string,
  label: string,
  issues: DoctorIssue[],
): Promise<LegacyInstalledJson | null> {
  if (!(await fileExists(file))) return null;

  let raw: unknown;
  try {
    raw = await Bun.file(file).json();
  } catch (e) {
    const backupFile = `${file}.bak`;
    const backupName = `${label}.bak`;
    issues.push({
      message: `${label} is corrupt: ${e}`,
      fixable: true,
      fixDescription: `backed up to ${backupName}, created fresh`,
      fix: async () => {
        await copyFile(file, backupFile).catch(() => {});
        await writeFile(
          file,
          JSON.stringify({ version: 1, skills: [] }, null, 2),
        );
      },
    });
    return null;
  }

  const result = LegacyInstalledJsonSchema.safeParse(raw);
  if (!result.success) {
    const backupFile = `${file}.bak`;
    const backupName = `${label}.bak`;
    issues.push({
      message: `${label} is invalid: ${z.prettifyError(result.error)}`,
      fixable: true,
      fixDescription: `backed up to ${backupName}, created fresh`,
      fix: async () => {
        await copyFile(file, backupFile).catch(() => {});
        await writeFile(
          file,
          JSON.stringify({ version: 1, skills: [] }, null, 2),
        );
      },
    });
    return null;
  }

  return result.data;
}

export async function checkInstalled(projectRoot?: string): Promise<{
  check: DoctorCheck;
  installed: InstalledSkill[] | null;
}> {
  const globalFile = join(getConfigDir(), "installed.json");
  const issues: DoctorIssue[] = [];

  // state.json is canonical. For diagnostic purposes, also check installed.json
  // so the doctor can advise unmigrated v0.x users to run `migrate`.
  const globalState = await loadState();
  const projectState = projectRoot ? await loadState(projectRoot) : null;

  const globalSkills =
    globalState.ok && globalState.value.skills.length > 0
      ? globalState.value.skills
      : ((await readInstalledFile(globalFile, "installed.json", issues))
          ?.skills ?? null);
  const projectSkills =
    projectState?.ok && projectState.value.skills.length > 0
      ? projectState.value.skills
      : projectRoot
        ? ((
            await readInstalledFile(
              join(projectRoot, ".agents", "installed.json"),
              ".agents/installed.json",
              issues,
            )
          )?.skills ?? null)
        : null;

  const merged: InstalledSkill[] = [
    ...(globalSkills ?? []),
    ...(projectSkills ?? []),
  ];

  if (issues.length > 0) {
    return {
      check: { name: "installed", status: "fail", issues },
      installed: merged,
    };
  }

  const globalCount = globalSkills?.length ?? 0;
  const projectCount = projectSkills?.length ?? 0;
  const total = merged.length;

  let detail: string;
  if (globalSkills === null && projectSkills === null) {
    detail = "0 skills (no skill records found)";
  } else if (projectSkills !== null) {
    detail = `${total} skill${total === 1 ? "" : "s"} (${globalCount} global, ${projectCount} project)`;
  } else {
    detail = `${total} skill${total === 1 ? "" : "s"}`;
  }

  return {
    check: { name: "installed", status: "pass", detail },
    installed: merged,
  };
}
