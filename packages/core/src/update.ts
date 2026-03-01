import { mkdir } from "node:fs/promises";
import { dirname, join } from "node:path";
import { $ } from "bun";
import type { AgentAdapter } from "./agents/types";
import { loadInstalled, saveInstalled } from "./config";
import type { DiffStat } from "./git";
import { diff, diffStat, fetch, pull, revParse } from "./git";
import { skillCacheDir, skillInstallDir } from "./paths";
import type { InstalledSkill } from "./schemas/installed";
import type { StaticWarning } from "./security";
import { scanDiff } from "./security";
import type { SemanticWarning } from "./security/semantic";
import { scanSemantic } from "./security/semantic";
import { createAgentSymlinks, removeAgentSymlinks } from "./symlink";
import type { Result } from "./types";
import { err, type GitError, ok, type ScanError, UserError } from "./types";

export type UpdateOptions = {
  /** Specific skill to update; undefined = update all */
  name?: string;
  /** Auto-accept clean updates without prompting */
  yes?: boolean;
  /** Skip skills that have security warnings in their diff */
  strict?: boolean;
  /** Project root for project-scoped symlink re-creation */
  projectRoot?: string;
  onProgress?: (
    skillName: string,
    status: "checking" | "upToDate" | "updated" | "skipped" | "linked",
  ) => void;
  onDiff?: (
    skillName: string,
    stat: DiffStat,
    fromSha: string,
    toSha: string,
  ) => void;
  /** Called when warnings are found. Return value only matters in non-strict mode: true = proceed. */
  onShowWarnings?: (warnings: StaticWarning[], skillName: string) => void;
  /** Called when user confirmation is needed. true = apply. */
  onConfirm?: (skillName: string, hasWarnings: boolean) => Promise<boolean>;
  /** Pre-resolved agent adapter for semantic scanning. */
  agent?: AgentAdapter;
  /** Whether to run semantic scan. */
  semantic?: boolean;
  /** Score threshold for semantic warnings (default 5). */
  threshold?: number;
  /** Called when semantic warnings are found. */
  onSemanticWarnings?: (warnings: SemanticWarning[], skillName: string) => void;
  /** Called with progress during semantic scan. */
  onSemanticProgress?: (completed: number, total: number) => void;
};

export type UpdateResult = {
  updated: string[];
  skipped: string[];
  upToDate: string[];
};

/** Decide whether the user wants to skip this update based on warnings and confirmation. */
async function shouldSkipUpdate(
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
async function recopyMultiSkill(
  workDir: string,
  record: InstalledSkill,
  projectRoot?: string,
): Promise<void> {
  if (record.path === null) return;
  const skillSrc = join(workDir, record.path);
  const destDir = skillInstallDir(
    record.name,
    record.scope as "global" | "project",
    projectRoot,
  );
  await $`rm -rf ${destDir}`.quiet();
  await mkdir(dirname(destDir), { recursive: true });
  await $`cp -r ${skillSrc} ${destDir}`.quiet();
}

/** Remove and re-create agent symlinks for a skill (idempotent). */
async function refreshAgentSymlinks(
  record: InstalledSkill,
  projectRoot?: string,
): Promise<void> {
  if (record.also.length === 0) return;
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

export async function updateSkill(
  options: UpdateOptions = {},
): Promise<Result<UpdateResult, UserError | GitError | ScanError>> {
  const installedResult = await loadInstalled();
  if (!installedResult.ok) return installedResult;
  const installed = installedResult.value;

  let skills = installed.skills;
  if (options.name) {
    const found = skills.filter((s) => s.name === options.name);
    if (found.length === 0) {
      return err(
        new UserError(
          `Skill '${options.name}' is not installed.`,
          "Run 'skilltap list' to see installed skills.",
        ),
      );
    }
    skills = found;
  }

  const result: UpdateResult = { updated: [], skipped: [], upToDate: [] };

  for (const record of skills) {
    if (record.scope === "linked") {
      options.onProgress?.(record.name, "linked");
      continue;
    }

    options.onProgress?.(record.name, "checking");

    // Standalone: work dir is the install path. Multi-skill: work dir is the cache.
    const isMulti = record.path !== null;
    const workDir = isMulti
      ? skillCacheDir(record.repo!)
      : skillInstallDir(
          record.name,
          record.scope as "global" | "project",
          options.projectRoot,
        );

    // For multi-skill, verify cache exists
    if (isMulti) {
      const cacheGitDir = Bun.file(join(workDir, ".git"));
      if (!(await cacheGitDir.exists())) {
        result.skipped.push(record.name);
        options.onProgress?.(record.name, "skipped");
        continue;
      }
    }

    const fetchResult = await fetch(workDir);
    if (!fetchResult.ok) return fetchResult;

    const localShaResult = await revParse(workDir, "HEAD");
    if (!localShaResult.ok) return localShaResult;
    const remoteShaResult = await revParse(workDir, "FETCH_HEAD");
    if (!remoteShaResult.ok) return remoteShaResult;

    const localSha = localShaResult.value;
    const remoteSha = remoteShaResult.value;

    if (localSha === remoteSha) {
      result.upToDate.push(record.name);
      options.onProgress?.(record.name, "upToDate");
      continue;
    }

    // Get diff (path-filtered for multi-skill)
    const pathSpec = record.path ?? undefined;
    const diffResult = await diff(workDir, "HEAD", "FETCH_HEAD", pathSpec);
    if (!diffResult.ok) return diffResult;
    const diffOutput = diffResult.value;

    const statResult = await diffStat(workDir, "HEAD", "FETCH_HEAD", pathSpec);
    if (!statResult.ok) return statResult;
    const stat = statResult.value;

    // If skill-specific path has no changes, mark as up to date
    if (stat.filesChanged === 0) {
      result.upToDate.push(record.name);
      options.onProgress?.(record.name, "upToDate");
      continue;
    }

    options.onDiff?.(record.name, stat, localSha, remoteSha);

    // Security scan on diff + confirmation
    const warnings = scanDiff(diffOutput);
    if (await shouldSkipUpdate(warnings, options, record.name)) {
      result.skipped.push(record.name);
      options.onProgress?.(record.name, "skipped");
      continue;
    }

    // Apply update
    const pullResult = await pull(workDir);
    if (!pullResult.ok) return pullResult;

    if (isMulti) await recopyMultiSkill(workDir, record, options.projectRoot);

    // Semantic scan on updated skill directory
    if (options.semantic && options.agent) {
      const installDir = skillInstallDir(
        record.name,
        record.scope as "global" | "project",
        options.projectRoot,
      );
      const semResult = await scanSemantic(installDir, options.agent, {
        threshold: options.threshold,
        onProgress: options.onSemanticProgress,
      });
      if (semResult.ok && semResult.value.length > 0) {
        options.onSemanticWarnings?.(semResult.value, record.name);
        if (options.strict) {
          result.skipped.push(record.name);
          options.onProgress?.(record.name, "skipped");
          continue;
        }
      }
    }

    // Get new SHA
    const newShaResult = await revParse(workDir, "HEAD");
    if (!newShaResult.ok) return newShaResult;

    // Update the record in place
    const idx = installed.skills.indexOf(record);
    if (idx !== -1) {
      installed.skills[idx] = {
        ...record,
        sha: newShaResult.value,
        updatedAt: new Date().toISOString(),
      };
    }

    await refreshAgentSymlinks(record, options.projectRoot);

    result.updated.push(record.name);
    options.onProgress?.(record.name, "updated");
  }

  const saveResult = await saveInstalled(installed);
  if (!saveResult.ok) return saveResult;

  return ok(result);
}
