import { $ } from "bun";
import type { InstalledJson } from "../../schemas/installed";
import type { DoctorCheck } from "../types";

export async function checkNpm(installed: InstalledJson): Promise<DoctorCheck | null> {
  const hasNpmSkills = installed.skills.some((s) => s.repo?.startsWith("npm:"));
  if (!hasNpmSkills) return null;

  let npmPath: string;
  try {
    npmPath = await $`which npm`
      .quiet()
      .then((r) => r.stdout.toString().trim());
  } catch {
    return {
      name: "npm",
      status: "warn",
      issues: [
        {
          message: "npm not found. Install Node.js for npm skill support.",
          fixable: false,
        },
      ],
    };
  }

  let version: string;
  try {
    version = await $`npm --version`
      .quiet()
      .then((r) => r.stdout.toString().trim());
  } catch {
    return {
      name: "npm",
      status: "warn",
      issues: [{ message: "npm --version failed", fixable: false }],
    };
  }

  const issues = [];

  try {
    await $`npm ping`.quiet();
  } catch {
    issues.push({
      message:
        "npm registry is not reachable. Check your network or registry config.",
      fixable: false,
    });
  }

  let whoami: string | null = null;
  try {
    whoami = await $`npm whoami`
      .quiet()
      .then((r) => r.stdout.toString().trim());
  } catch {
    issues.push({
      message: "Not logged in to npm. Run 'npm login' if you need to publish.",
      fixable: false,
    });
  }

  const detail = whoami
    ? `${npmPath} (${version}) — logged in as ${whoami}`
    : `${npmPath} (${version})`;

  if (issues.length === 0) {
    return { name: "npm", status: "pass", detail };
  }
  return { name: "npm", status: "warn", detail, issues };
}
