import { copyFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { z } from "zod/v4";
import { getConfigDir } from "../../config";
import { fileExists } from "../../fs";
import type { InstalledJson } from "../../schemas/installed";
import { InstalledJsonSchema } from "../../schemas/installed";
import type { DoctorCheck, DoctorIssue } from "../types";

async function readInstalledFile(
  file: string,
  label: string,
  issues: DoctorIssue[],
): Promise<InstalledJson | null> {
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
        await writeFile(file, JSON.stringify({ version: 1, skills: [] }, null, 2));
      },
    });
    return null;
  }

  const result = InstalledJsonSchema.safeParse(raw);
  if (!result.success) {
    const backupFile = `${file}.bak`;
    const backupName = `${label}.bak`;
    issues.push({
      message: `${label} is invalid: ${z.prettifyError(result.error)}`,
      fixable: true,
      fixDescription: `backed up to ${backupName}, created fresh`,
      fix: async () => {
        await copyFile(file, backupFile).catch(() => {});
        await writeFile(file, JSON.stringify({ version: 1, skills: [] }, null, 2));
      },
    });
    return null;
  }

  return result.data;
}

export async function checkInstalled(projectRoot?: string): Promise<{
  check: DoctorCheck;
  installed: InstalledJson | null;
}> {
  const globalFile = join(getConfigDir(), "installed.json");
  const issues: DoctorIssue[] = [];

  const globalInstalled = await readInstalledFile(globalFile, "installed.json", issues);
  const projectInstalled = projectRoot
    ? await readInstalledFile(
        join(projectRoot, ".agents", "installed.json"),
        ".agents/installed.json",
        issues,
      )
    : null;

  const allSkills = [
    ...(globalInstalled?.skills ?? []),
    ...(projectInstalled?.skills ?? []),
  ];
  const merged: InstalledJson = { version: 1 as const, skills: allSkills };

  if (issues.length > 0) {
    return { check: { name: "installed", status: "fail", issues }, installed: merged };
  }

  const globalCount = globalInstalled?.skills.length ?? 0;
  const projectCount = projectInstalled?.skills.length ?? 0;
  const total = allSkills.length;

  let detail: string;
  if (!globalInstalled && !projectInstalled) {
    detail = "0 skills (no installed.json)";
  } else if (projectInstalled !== null) {
    detail = `${total} skill${total === 1 ? "" : "s"} (${globalCount} global, ${projectCount} project)`;
  } else {
    detail = `${total} skill${total === 1 ? "" : "s"}`;
  }

  return { check: { name: "installed", status: "pass", detail }, installed: merged };
}
