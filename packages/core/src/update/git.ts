import { lstat, mkdir } from "node:fs/promises";
import { dirname, join } from "node:path";
import { $ } from "bun";
import {
  diff,
  diffStat,
  fetch,
  pull,
  resetHard,
  revParse,
} from "../git";
import { currentSkillDir, skillCacheDir, skillInstallDir } from "../paths";
import { resolvedDirExists } from "../fs";
import type { InstalledSkill } from "../schemas/installed";
import { scanDiff } from "../security";
import { wrapShell } from "../shell";
import { parseGitHubRepo } from "../trust";
import type { Result, GitError, ScanError, UserError } from "../types";
import { ok } from "../types";
import {
  patchRecord,
  recopyMultiSkill,
  refreshAgentSymlinks,
  runUpdateSemanticScan,
  shouldSkipUpdate,
  skipSkill,
} from "./shared";
import type { ResolveTrustFn, UpdateOptions, UpdateResult } from "./types";
import { removeAgentSymlinks } from "../symlink";

/** Handle updates for standalone git skills (path === null; workDir is the install dir). */
export async function updateGitSkill(
  record: InstalledSkill,
  installed: { skills: InstalledSkill[] },
  options: UpdateOptions,
  result: UpdateResult,
  _resolveTrust: ResolveTrustFn,
): Promise<Result<void, UserError | GitError | ScanError>> {
  const workDir = currentSkillDir(record, options.projectRoot);

  // Before fetch: verify the work directory exists (handles orphan installs)
  if (!(await resolvedDirExists(workDir))) {
    result.skipped.push(record.name);
    options.onProgress?.(record.name, "removed-upstream");
    return ok(undefined);
  }

  const fetchResult = await fetch(workDir);
  if (!fetchResult.ok) {
    // Old local-path installs may have a deleted source. Gracefully skip.
    if (record.repo?.startsWith("/")) {
      result.upToDate.push(record.name);
      options.onProgress?.(record.name, "local");
      return ok(undefined);
    }
    return fetchResult;
  }

  const localShaResult = await revParse(workDir, "HEAD");
  if (!localShaResult.ok) return localShaResult;
  const remoteShaResult = await revParse(workDir, "FETCH_HEAD");
  if (!remoteShaResult.ok) return remoteShaResult;

  const localSha = localShaResult.value;
  const remoteSha = remoteShaResult.value;

  if (localSha === remoteSha && !options.force) {
    await refreshAgentSymlinks(record, options.projectRoot);
    result.upToDate.push(record.name);
    options.onProgress?.(record.name, "upToDate");
    return ok(undefined);
  }

  const diffResult = await diff(workDir, "HEAD", "FETCH_HEAD");
  if (!diffResult.ok) return diffResult;

  const statResult = await diffStat(workDir, "HEAD", "FETCH_HEAD");
  if (!statResult.ok) return statResult;
  const stat = statResult.value;

  options.onDiff?.(record.name, stat, localSha, remoteSha, diffResult.value);

  const warnings = scanDiff(diffResult.value);
  if (await shouldSkipUpdate(warnings, options, record.name)) {
    return skipSkill(result, options, record.name);
  }

  const pullResult = await pull(workDir);
  if (!pullResult.ok) return pullResult;

  if (await runUpdateSemanticScan(workDir, record.name, options)) {
    // Reset to pre-pull state so the next run re-detects the pending update
    await resetHard(workDir, localSha);
    return skipSkill(result, options, record.name);
  }

  const newShaResult = await revParse(workDir, "HEAD");
  if (!newShaResult.ok) return newShaResult;

  const newTrust = await _resolveTrust({
    adapter: "git",
    url: record.repo ?? "",
    tap: record.tap,
    skillDir: workDir,
    githubRepo: record.repo ? parseGitHubRepo(record.repo) : null,
  });

  patchRecord(installed, record, {
    sha: newShaResult.value,
    updatedAt: new Date().toISOString(),
    trust: newTrust,
  });

  await refreshAgentSymlinks(record, options.projectRoot);

  result.updated.push(record.name);
  options.onProgress?.(record.name, "updated");
  return ok(undefined);
}

/** Handle updates for a group of skills sharing the same multi-skill git repo cache. */
export async function updateGitSkillGroup(
  repo: string,
  skills: InstalledSkill[],
  installed: { skills: InstalledSkill[] },
  options: UpdateOptions,
  result: UpdateResult,
  _resolveTrust: ResolveTrustFn,
): Promise<Result<void, UserError | GitError | ScanError>> {
  const workDir = skillCacheDir(repo);

  // Verify cache exists
  const cacheGitExists = await lstat(join(workDir, ".git"))
    .then(() => true)
    .catch(() => false);
  if (!cacheGitExists) {
    for (const skill of skills) {
      result.skipped.push(skill.name);
      options.onProgress?.(skill.name, "skipped");
    }
    return ok(undefined);
  }

  // Fetch once for the whole group
  const fetchResult = await fetch(workDir);
  if (!fetchResult.ok) return fetchResult;

  // Capture SHAs BEFORE any pull
  const localShaResult = await revParse(workDir, "HEAD");
  if (!localShaResult.ok) return localShaResult;
  const remoteShaResult = await revParse(workDir, "FETCH_HEAD");
  if (!remoteShaResult.ok) return remoteShaResult;

  const localSha = localShaResult.value;
  const remoteSha = remoteShaResult.value;

  // If the whole repo is up to date, all skills in the group are too
  if (localSha === remoteSha && !options.force) {
    for (const skill of skills) {
      await refreshAgentSymlinks(skill, options.projectRoot);
      result.upToDate.push(skill.name);
      options.onProgress?.(skill.name, "upToDate");
    }
    return ok(undefined);
  }

  // Per-skill: check path-specific diff, scan, confirm
  const toUpdate: InstalledSkill[] = [];
  for (const skill of skills) {
    options.onProgress?.(skill.name, "checking");

    // biome-ignore lint/style/noNonNullAssertion: multi-skill records always have path
    const pathSpec = skill.path!;

    const statResult = await diffStat(workDir, "HEAD", "FETCH_HEAD", pathSpec);
    if (!statResult.ok) return statResult;
    const stat = statResult.value;

    if (stat.filesChanged === 0 && !options.force) {
      result.upToDate.push(skill.name);
      options.onProgress?.(skill.name, "upToDate");
      continue;
    }

    const diffResult = await diff(workDir, "HEAD", "FETCH_HEAD", pathSpec);
    if (!diffResult.ok) return diffResult;

    options.onDiff?.(skill.name, stat, localSha, remoteSha, diffResult.value);

    const warnings = scanDiff(diffResult.value);
    if (await shouldSkipUpdate(warnings, options, skill.name)) {
      result.skipped.push(skill.name);
      options.onProgress?.(skill.name, "skipped");
      continue;
    }

    toUpdate.push(skill);
  }

  if (toUpdate.length === 0) return ok(undefined);

  // Pull once for the whole group
  const pullResult = await pull(workDir);
  if (!pullResult.ok) return pullResult;

  const newShaResult = await revParse(workDir, "HEAD");
  if (!newShaResult.ok) return newShaResult;
  const newSha = newShaResult.value;

  // Apply update to each confirmed skill
  let anySemanticBlocked = false;
  for (const skill of toUpdate) {
    // biome-ignore lint/style/noNonNullAssertion: multi-skill records always have path
    const skillCacheSubdir = join(workDir, skill.path!);

    // Check if skill still exists in pulled repo
    if (!(await resolvedDirExists(skillCacheSubdir))) {
      if (options.onSkillRemovedUpstream) {
        const action = await options.onSkillRemovedUpstream(skill.name, repo);
        if (action === "remove") {
          const effectiveScope = skill.scope as "global" | "project";
          const installDir = skillInstallDir(
            skill.name,
            effectiveScope,
            options.projectRoot,
          );
          await wrapShell(
            () => $`rm -rf ${installDir}`.quiet().then(() => undefined),
            "",
          );
          await removeAgentSymlinks(
            skill.name,
            skill.also,
            skill.scope,
            options.projectRoot,
          );
          installed.skills = installed.skills.filter((s) => s !== skill);
        }
      }
      result.skipped.push(skill.name);
      options.onProgress?.(skill.name, "removed-upstream");
      continue;
    }

    // Semantic scan on cache subdir BEFORE recopy so we can roll back cleanly on failure
    if (await runUpdateSemanticScan(skillCacheSubdir, skill.name, options)) {
      result.skipped.push(skill.name);
      options.onProgress?.(skill.name, "skipped");
      anySemanticBlocked = true;
      continue;
    }

    const recopyResult = await recopyMultiSkill(
      workDir,
      skill,
      options.projectRoot,
    );
    if (!recopyResult.ok) return recopyResult;

    const installDir = skillInstallDir(
      skill.name,
      skill.scope as "global" | "project",
      options.projectRoot,
    );

    const newTrust = await _resolveTrust({
      adapter: "git",
      url: skill.repo ?? "",
      tap: skill.tap,
      skillDir: installDir,
      githubRepo: skill.repo ? parseGitHubRepo(skill.repo) : null,
    });

    patchRecord(installed, skill, {
      sha: newSha,
      updatedAt: new Date().toISOString(),
      trust: newTrust,
    });

    await refreshAgentSymlinks(skill, options.projectRoot);

    result.updated.push(skill.name);
    options.onProgress?.(skill.name, "updated");
  }

  // If any skills were blocked by semantic scan, reset the cache so the next
  // run re-detects the pending update instead of showing "up to date"
  if (anySemanticBlocked) {
    await resetHard(workDir, localSha);
  }

  return ok(undefined);
}
