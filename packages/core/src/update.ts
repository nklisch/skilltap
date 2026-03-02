import { lstat, mkdir } from "node:fs/promises";
import { dirname, join } from "node:path";
import { $ } from "bun";
import type { AgentAdapter } from "./agents/types";
import { loadInstalled, saveInstalled } from "./config";
import { makeTmpDir, removeTmpDir } from "./fs";
import type { DiffStat } from "./git";
import { diff, diffStat, fetch, pull, revParse } from "./git";
import {
  downloadAndExtract,
  fetchPackageMetadata,
  parseNpmSource,
  resolveVersion,
} from "./npm-registry";
import { skillCacheDir, skillInstallDir } from "./paths";
import type { InstalledSkill } from "./schemas/installed";
import type { StaticWarning } from "./security";
import { scanDiff, scanStatic } from "./security";
import type { SemanticWarning } from "./security/semantic";
import { scanSemantic } from "./security/semantic";
import { createAgentSymlinks, removeAgentSymlinks } from "./symlink";
import { parseGitHubRepo, resolveTrust } from "./trust";
import type { Result } from "./types";
import {
  err,
  type GitError,
  NetworkError,
  ok,
  type ScanError,
  UserError,
} from "./types";

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

/** Handle updates for npm-sourced skills (version comparison instead of git SHA). */
async function updateNpmSkill(
  record: InstalledSkill,
  installed: { skills: InstalledSkill[] },
  options: UpdateOptions,
  result: UpdateResult,
): Promise<Result<void, UserError | NetworkError | ScanError>> {
  // biome-ignore lint/style/noNonNullAssertion: caller checks record.repo?.startsWith("npm:")
  const { name: packageName } = parseNpmSource(record.repo!);

  const metaResult = await fetchPackageMetadata(packageName);
  if (!metaResult.ok) {
    // Network failure — skip gracefully rather than hard-failing the whole update
    result.skipped.push(record.name);
    options.onProgress?.(record.name, "skipped");
    return ok(undefined);
  }

  const versionResult = resolveVersion(metaResult.value, "latest");
  if (!versionResult.ok) {
    result.skipped.push(record.name);
    options.onProgress?.(record.name, "skipped");
    return ok(undefined);
  }

  const latestVersion = versionResult.value.version;

  if (record.ref === latestVersion) {
    result.upToDate.push(record.name);
    options.onProgress?.(record.name, "upToDate");
    return ok(undefined);
  }

  const tmpResult = await makeTmpDir();
  if (!tmpResult.ok) return tmpResult;
  const tmpDir = tmpResult.value;

  try {
    const info = versionResult.value;
    const extractResult = await downloadAndExtract(
      info.dist.tarball,
      tmpDir,
      info.dist.integrity,
    );
    if (!extractResult.ok) {
      result.skipped.push(record.name);
      options.onProgress?.(record.name, "skipped");
      return ok(undefined);
    }

    const pkgDir = extractResult.value;
    // Standalone: path is null → use the whole package dir
    // Multi-skill: path is relative within the package (e.g. "skills/skill-a")
    const newSkillDir = record.path ? join(pkgDir, record.path) : pkgDir;

    // Static security scan on the new version's content
    const scanResult = await scanStatic(newSkillDir);
    const warnings: StaticWarning[] = scanResult.ok ? scanResult.value : [];

    if (await shouldSkipUpdate(warnings, options, record.name)) {
      result.skipped.push(record.name);
      options.onProgress?.(record.name, "skipped");
      return ok(undefined);
    }

    // Replace the installed skill directory
    const installDir = skillInstallDir(
      record.name,
      record.scope as "global" | "project",
      options.projectRoot,
    );
    await $`rm -rf ${installDir}`.quiet();
    await mkdir(dirname(installDir), { recursive: true });
    await $`cp -r ${newSkillDir} ${installDir}`.quiet();

    // Semantic scan on updated content
    if (options.semantic && options.agent) {
      const semResult = await scanSemantic(installDir, options.agent, {
        threshold: options.threshold,
        onProgress: options.onSemanticProgress,
      });
      if (semResult.ok && semResult.value.length > 0) {
        options.onSemanticWarnings?.(semResult.value, record.name);
        if (options.strict) {
          result.skipped.push(record.name);
          options.onProgress?.(record.name, "skipped");
          return ok(undefined);
        }
      }
    }

    await refreshAgentSymlinks(record, options.projectRoot);

    // Re-verify trust for the new version
    const { name: packageName } = parseNpmSource(record.repo!);
    const newTrust = await resolveTrust({
      adapter: "npm",
      url: record.repo!,
      tap: record.tap,
      tarballPath: join(tmpDir, "_pkg.tgz"),
      npmPackageName: packageName,
      npmVersion: latestVersion,
      npmPublisher: record.trust?.publisher?.name,
    });

    // Update record in place
    const idx = installed.skills.indexOf(record);
    if (idx !== -1) {
      installed.skills[idx] = {
        ...record,
        ref: latestVersion,
        sha: null,
        updatedAt: new Date().toISOString(),
        trust: newTrust,
      };
    }

    result.updated.push(record.name);
    options.onProgress?.(record.name, "updated");
    return ok(undefined);
  } finally {
    await removeTmpDir(tmpDir);
  }
}

export async function updateSkill(
  options: UpdateOptions = {},
): Promise<Result<UpdateResult, UserError | GitError | ScanError | NetworkError>> {
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

    // npm-sourced skills use version comparison instead of git SHA comparison
    if (record.repo?.startsWith("npm:")) {
      const npmResult = await updateNpmSkill(record, installed, options, result);
      if (!npmResult.ok) return npmResult;
      continue;
    }

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
      const cacheGitExists = await lstat(join(workDir, ".git"))
        .then(() => true)
        .catch(() => false);
      if (!cacheGitExists) {
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

    // Re-verify trust for the updated skill
    const installDir = skillInstallDir(
      record.name,
      record.scope as "global" | "project",
      options.projectRoot,
    );
    const newTrust = await resolveTrust({
      adapter: "git",
      url: record.repo ?? "",
      tap: record.tap,
      skillDir: installDir,
      githubRepo: record.repo ? parseGitHubRepo(record.repo) : null,
    });

    // Update the record in place
    const idx = installed.skills.indexOf(record);
    if (idx !== -1) {
      installed.skills[idx] = {
        ...record,
        sha: newShaResult.value,
        updatedAt: new Date().toISOString(),
        trust: newTrust,
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
