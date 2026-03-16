import { $ } from "bun";
import type { DoctorCheck } from "../types";

export async function checkGit(): Promise<DoctorCheck> {
  let gitPath: string;
  try {
    gitPath = await $`which git`.quiet().then((r) => r.stdout.toString().trim());
  } catch {
    return {
      name: "git",
      status: "fail",
      issues: [
        {
          message: "git not found on PATH. Install git: https://git-scm.com",
          fixable: false,
        },
      ],
    };
  }

  let versionStr: string;
  try {
    versionStr = await $`git --version`
      .quiet()
      .then((r) => r.stdout.toString().trim());
  } catch {
    return {
      name: "git",
      status: "fail",
      issues: [
        {
          message: "git not found on PATH. Install git: https://git-scm.com",
          fixable: false,
        },
      ],
    };
  }

  const match = versionStr.match(/(\d+)\.(\d+)\.(\d+)/);
  const major = match ? parseInt(match[1]!, 10) : 0;
  const minor = match ? parseInt(match[2]!, 10) : 0;
  const patch = match ? parseInt(match[3]!, 10) : 0;
  const versionTag = match ? `${major}.${minor}.${patch}` : versionStr;
  const detail = `${gitPath} (${versionTag})`;

  if (major < 2 || (major === 2 && minor < 25)) {
    return {
      name: "git",
      status: "warn",
      detail,
      issues: [
        {
          message: `git 2.25+ recommended (found ${versionTag}). Shallow clone --filter may not work.`,
          fixable: false,
        },
      ],
    };
  }

  return { name: "git", status: "pass", detail };
}
