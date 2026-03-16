import { join } from "node:path";
import { z } from "zod/v4";
import { getConfigDir } from "../../config";
import { fileExists, resolvedDirExists } from "../../fs";
import { clone } from "../../git";
import type { Config } from "../../schemas/config";
import { TapSchema } from "../../schemas/tap";
import { BUILTIN_TAP } from "../../taps";
import type { DoctorCheck, DoctorIssue } from "../types";

export async function checkTaps(config: Config): Promise<DoctorCheck> {
  const issues: DoctorIssue[] = [];
  const info: string[] = [];
  let validCount = 0;

  const hasBuiltin = config.builtin_tap !== false;
  const allTaps: Array<{ name: string; url: string; type: "git" | "http" | "builtin" }> = [];

  if (hasBuiltin) {
    allTaps.push({ name: BUILTIN_TAP.name, url: BUILTIN_TAP.url, type: "builtin" });
  }
  for (const tap of config.taps) {
    allTaps.push({ name: tap.name, url: tap.url, type: tap.type });
  }

  if (allTaps.length === 0) {
    return { name: "taps", status: "pass", detail: "0 configured" };
  }

  for (const tap of allTaps) {
    if (tap.type === "http") {
      validCount++;
      info.push(`${tap.name} (http): ok`);
      continue;
    }

    const dir = join(getConfigDir(), "taps", tap.name);
    const label = tap.type === "builtin" ? `${tap.name} (built-in)` : tap.name;

    if (!(await resolvedDirExists(dir))) {
      const tapUrl = tap.url;
      issues.push({
        message: `tap '${tap.name}': directory missing. Run 'skilltap tap update ${tap.name}' to re-clone.`,
        fixable: true,
        fixDescription: "re-cloned tap",
        fix: async () => {
          await clone(tapUrl, dir, { depth: 1 });
        },
      });
      continue;
    }

    const tapJsonFile = join(dir, "tap.json");
    if (!(await fileExists(tapJsonFile))) {
      issues.push({
        message: `tap '${tap.name}': tap.json is missing`,
        fixable: false,
      });
      continue;
    }

    let tapRaw: unknown;
    try {
      tapRaw = await Bun.file(tapJsonFile).json();
    } catch (e) {
      issues.push({
        message: `tap '${tap.name}': tap.json is invalid JSON: ${e}`,
        fixable: false,
      });
      continue;
    }

    const tapResult = TapSchema.safeParse(tapRaw);
    if (!tapResult.success) {
      issues.push({
        message: `tap '${tap.name}': tap.json is invalid: ${z.prettifyError(tapResult.error)}`,
        fixable: false,
      });
      continue;
    }

    const gitDir = join(dir, ".git");
    if (!(await resolvedDirExists(gitDir))) {
      issues.push({
        message: `tap '${tap.name}': .git directory missing (not a git repo)`,
        fixable: false,
      });
      continue;
    }

    validCount++;
    info.push(`${label}: ok (${tapResult.data.skills.length} skills)`);
  }

  const total = allTaps.length;

  if (issues.length === 0) {
    return {
      name: "taps",
      status: "pass",
      detail: `${total} configured, ${validCount} valid`,
      info,
    };
  }
  return {
    name: "taps",
    status: "warn",
    detail: `${total} configured, ${validCount} valid`,
    issues,
    info,
  };
}
