import type { Output } from "../output/types";
import { resolvedDirExists } from "../fs";
import { currentSkillDir } from "../paths";
import type { ScannedSkill } from "../scanner";
import type { InstalledSkill } from "../schemas/installed";
import type { StaticWarning } from "../security";
import { scanStatic } from "../security";
import type { SemanticWarning } from "../security/semantic";
import { scanSemantic } from "../security/semantic";
import { removeAgentSymlinks } from "../symlink";
import type { Result, ScanError, UserError } from "../types";
import { err, ok, UserError as UserErrorClass } from "../types";
import type { InstallOptions } from "./types";

export type ConflictCheckResult = {
  toUpdate: string[];
  toInstall: ScannedSkill[];
};

/** Check for already-installed conflicts; removes phantom records and returns filtered install set. */
export async function checkConflicts(
  selected: ScannedSkill[],
  installed: InstalledSkill[],
  options: InstallOptions,
): Promise<Result<ConflictCheckResult, UserError>> {
  const toUpdate: string[] = [];
  const toInstall: ScannedSkill[] = [];
  const projectRoot =
    options.scope === "project" ? options.projectRoot : undefined;

  for (const skill of selected) {
    const conflict = installed.find(
      (s) => s.name === skill.name && s.scope === options.scope,
    );
    if (conflict) {
      const conflictDir = currentSkillDir(conflict, projectRoot);

      if (!(await resolvedDirExists(conflictDir))) {
        installed.splice(installed.indexOf(conflict), 1);
        await removeAgentSymlinks(
          conflict.name,
          conflict.also,
          conflict.scope,
          projectRoot,
        );
        toInstall.push(skill);
        continue;
      }

      if (!options.onAlreadyInstalled) {
        return err(
          new UserErrorClass(
            `Skill '${skill.name}' is already installed.`,
            `Use 'skilltap update ${skill.name}' to update, or 'skilltap remove ${skill.name}' first.`,
          ),
        );
      }
      const action = await options.onAlreadyInstalled(skill.name);
      if (action === "abort") {
        return err(
          new UserErrorClass(
            `Skill '${skill.name}' is already installed.`,
            `Use 'skilltap update ${skill.name}' to update, or 'skilltap remove ${skill.name}' first.`,
          ),
        );
      }
      toUpdate.push(skill.name);
    } else {
      toInstall.push(skill);
    }
  }

  return ok({ toUpdate, toInstall });
}

export type SelectAndScanResult = {
  selected: ScannedSkill[];
  toUpdate: string[];
  allWarnings: StaticWarning[];
  allSemanticWarnings: SemanticWarning[];
  /** True when all selected skills were already installed; no placement needed. */
  allAlreadyInstalled: boolean;
};

/**
 * Select which skills to install (filter by name, apply tap pre-filter),
 * check conflicts, run security scans, and confirm installation.
 * Returns the final list of skills to place plus accumulated warnings.
 */
export async function selectAndScan(
  scanned: ScannedSkill[],
  installed: InstalledSkill[],
  options: InstallOptions,
  tapSkillName: string | undefined,
): Promise<Result<SelectAndScanResult, UserError | ScanError>> {
  // Select skills to install
  let selectedNames: string[] | undefined = options.skillNames;
  if (!selectedNames && tapSkillName) {
    selectedNames = [tapSkillName];
  }
  if (!selectedNames && options.onSelectSkills) {
    selectedNames = await options.onSelectSkills(scanned);
  }
  let selected: ScannedSkill[];
  try {
    selected = selectedNames
      ? selectedNames.map((name) => {
          const found = scanned.find((s) => s.name === name);
          if (!found)
            throw new UserErrorClass(
              `Skill "${name}" not found in repo. Available: ${scanned.map((s) => s.name).join(", ")}`,
            );
          return found;
        })
      : scanned;
  } catch (e) {
    if (e instanceof UserErrorClass) return err(e);
    throw e;
  }

  // Check conflicts
  const conflictResult = await checkConflicts(selected, installed, options);
  if (!conflictResult.ok) return conflictResult;
  const { toUpdate, toInstall } = conflictResult.value;

  // If every selected skill is already installed, return early flag
  if (toInstall.length === 0) {
    return ok({ selected: [], toUpdate, allWarnings: [], allSemanticWarnings: [], allAlreadyInstalled: true });
  }
  selected = toInstall;

  const allWarnings: StaticWarning[] = [];
  const allSemanticWarnings: SemanticWarning[] = [];

  // Security scan (unless skipped)
  if (!options.skipScan) {
    const scanResult = await runSecurityScan(selected, options.out, options.onWarnings);
    if (!scanResult.ok) return scanResult;
    allWarnings.push(...scanResult.value);
  }

  // Semantic scan
  const semResult = await runSemanticScan(selected, options);
  if (!semResult.ok) return semResult;
  allSemanticWarnings.push(...semResult.value.allSemanticWarnings);

  // Clean-install confirmation (fires only when no warnings and no --yes)
  if (
    allWarnings.length === 0 &&
    allSemanticWarnings.length === 0 &&
    options.onConfirmInstall
  ) {
    const proceed = await options.onConfirmInstall("skill", selected.map((s) => s.name));
    if (!proceed) return err(new UserErrorClass("Install cancelled."));
  }

  return ok({ selected, toUpdate, allWarnings, allSemanticWarnings, allAlreadyInstalled: false });
}

export async function runSecurityScan(
  selected: ScannedSkill[],
  out: Output | undefined,
  onWarnings?: InstallOptions["onWarnings"],
): Promise<Result<StaticWarning[], ScanError | UserError>> {
  const allWarnings: StaticWarning[] = [];
  for (const skill of selected) {
    const p = out?.progress(`Scanning ${skill.name}`);
    const scanResult = await scanStatic(skill.path);
    if (!scanResult.ok) {
      p?.fail();
      return scanResult;
    }
    if (scanResult.value.length > 0) {
      p?.succeed();
      allWarnings.push(...scanResult.value);
      if (onWarnings) {
        const proceed = await onWarnings(
          scanResult.value,
          "skill-static",
          skill.name,
        );
        if (!proceed) return err(new UserErrorClass("Install cancelled."));
      }
    } else {
      p?.succeed();
    }
  }
  return ok(allWarnings);
}

export async function runSemanticScan(
  selected: ScannedSkill[],
  options: InstallOptions,
): Promise<Result<{ allSemanticWarnings: SemanticWarning[] }, UserError>> {
  const allSemanticWarnings: SemanticWarning[] = [];

  if (!options.semantic || !options.agent || options.skipScan) {
    return ok({ allSemanticWarnings });
  }

  for (const skill of selected) {
    const semP = options.out?.progress(`Semantic scan: ${skill.name}`);
    const semResult = await scanSemantic(skill.path, options.agent, {
      threshold: options.threshold,
      onProgress: (completed, total, score, reason) => {
        const threshold = options.threshold ?? 5;
        const flag = score >= threshold ? ` — ${reason.slice(0, 60)}` : "";
        semP?.update(`Chunk ${completed}/${total}${flag}`);
      },
    });
    semP?.succeed();
    if (semResult.ok && semResult.value.length > 0) {
      allSemanticWarnings.push(...semResult.value);
      if (options.onWarnings) {
        const proceed = await options.onWarnings(
          semResult.value,
          "skill-semantic",
          skill.name,
        );
        if (!proceed) return err(new UserErrorClass("Install cancelled."));
      }
    }
  }

  return ok({ allSemanticWarnings });
}
