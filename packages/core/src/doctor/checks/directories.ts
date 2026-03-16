import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import { getConfigDir } from "../../config";
import { globalBase, resolvedDirExists } from "../../fs";
import type { DoctorCheck, DoctorIssue } from "../types";

export async function checkDirs(): Promise<DoctorCheck> {
  const configDir = getConfigDir();
  const issues: DoctorIssue[] = [];

  const required = [
    configDir,
    join(configDir, "cache"),
    join(configDir, "taps"),
    join(globalBase(), ".agents", "skills"),
  ];

  for (const dir of required) {
    if (!(await resolvedDirExists(dir))) {
      issues.push({
        message: `Missing directory: ${dir}`,
        fixable: true,
        fixDescription: `created ${dir}`,
        fix: async () => {
          await mkdir(dir, { recursive: true });
        },
      });
    }
  }

  if (issues.length === 0) {
    return { name: "dirs", status: "pass", detail: configDir };
  }
  return { name: "dirs", status: "warn", issues };
}
