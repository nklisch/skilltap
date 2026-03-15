import { lstat, mkdir } from "node:fs/promises";
import { dirname, join } from "node:path";
import { $ } from "bun";
import type { AgentAdapter } from "./agents/types";
import { loadInstalled, saveInstalled } from "./config";
import { debug } from "./debug";
import { makeTmpDir, removeTmpDir } from "./fs";
import type { DiffStat } from "./git";
import { diff, diffStat, fetch, pull, resetHard, revParse } from "./git";
import {
  downloadAndExtract,
  fetchPackageMetadata,
  parseNpmSource,
  resolveVersion,
} from "./npm-registry";
import { skillCacheDir, skillDisabledDir, skillInstallDir } from "./paths";
import type { InstalledJson, InstalledSkill } from "./schemas/installed";
import type { StaticWarning } from "./security";
import { scanDiff, scanStatic } from "./security";
import type { SemanticWarning } from "./security/semantic";
import { scanSemantic } from "./security/semantic";
import { wrapShell } from "./shell";
import { createAgentSymlinks, removeAgentSymlinks } from "./symlink";
import { parseGitHubRepo, resolveTrust } from "./trust";

type ResolveTrustFn = typeof resolveTrust;
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
  /** Project root — also processes project-scoped skills from {projectRoot}/.agents/installed.json */
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
    rawDiff: string,
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
  /** Called before starting semantic scan for a skill. */
  onSemanticScanStart?: (skillName: string) => void;
  /** Called with progress during semantic scan. */
  onSemanticProgress?: (completed: number, total: number, score: number, reason: string) => void;
  /** Force re-apply the update even if the skill appears up to date (same SHA / version). */
  force?: boolean;
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
): Promise<Result<void, UserError>> {
  if (record.path === null) return ok(undefined);
  const skillSrc = join(workDir, record.path);
  const destDir = skillInstallDir(
    record.name,
    record.scope as "global" | "project",
    projectRoot,
  );
  const rmResult = await wrapShell(
    () => $`rm -rf ${destDir}`.quiet().then(() => undefined),
    `Failed to remove old skill directory '${record.name}'`,
  );
  if (!rmResult.ok) return rmResult;

  await mkdir(dirname(destDir), { recursive: true });

  return wrapShell(
    () => $`cp -r ${skillSrc} ${destDir}`.quiet().then(() => undefined),
    `Failed to copy updated skill '${record.name}'`,
    "Check disk space and permissions.",
  );
}

/** Remove and re-create agent symlinks for a skill (idempotent). Skips disabled skills. */
async function refreshAgentSymlinks(
  record: InstalledSkill,
  projectRoot?: string,
): Promise<void> {
  if (record.also.length === 0) return;
  if (record.active === false) return;
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

/** Run semantic scan on a skill directory after an update. Returns whether to skip the skill. */
async function runUpdateSemanticScan(
  installDir: string,
  skillName: string,
  options: UpdateOptions,
): Promise<boolean> {
  if (!options.semantic || !options.agent) return false;
  options.onSemanticScanStart?.(skillName);
  const semResult = await scanSemantic(installDir, options.agent, {
    threshold: options.threshold,
    onProgress: options.onSemanticProgress,
  });
  if (semResult.ok && semResult.value.length > 0) {
    options.onSemanticWarnings?.(semResult.value, skillName);
    if (options.strict) return true;
  }
  return false;
}

function skipSkill(
  result: UpdateResult,
  options: UpdateOptions,
  name: string,
): Result<void, never> {
  result.skipped.push(name);
  options.onProgress?.(name, "skipped");
  return ok(undefined);
}

function patchRecord(
  installed: { skills: InstalledSkill[] },
  record: InstalledSkill,
  updates: Partial<InstalledSkill>,
): void {
  const idx = installed.skills.indexOf(record);
  if (idx !== -1) {
    installed.skills[idx] = { ...record, ...updates };
  }
}

/** Handle updates for npm-sourced skills (version comparison instead of git SHA). */
async function updateNpmSkill(
  record: InstalledSkill,
  installed: { skills: InstalledSkill[] },
  options: UpdateOptions,
  result: UpdateResult,
  _resolveTrust: ResolveTrustFn,
): Promise<Result<void, UserError | NetworkError | ScanError>> {
  // biome-ignore lint/style/noNonNullAssertion: caller checks record.repo?.startsWith("npm:")
  const { name: packageName } = parseNpmSource(record.repo!);

  const metaResult = await fetchPackageMetadata(packageName);
  if (!metaResult.ok) {
    // Network failure — skip gracefully rather than hard-failing the whole update
    return skipSkill(result, options, record.name);
  }

  const versionResult = resolveVersion(metaResult.value, "latest");
  if (!versionResult.ok) {
    return skipSkill(result, options, record.name);
  }

  const latestVersion = versionResult.value.version;

  if (record.ref === latestVersion && !options.force) {
    await refreshAgentSymlinks(record, options.projectRoot);
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
      return skipSkill(result, options, record.name);
    }

    const pkgDir = extractResult.value;
    // Standalone: path is null → use the whole package dir
    // Multi-skill: path is relative within the package (e.g. "skills/skill-a")
    const newSkillDir = record.path ? join(pkgDir, record.path) : pkgDir;

    // Static security scan on the new version's content
    const scanResult = await scanStatic(newSkillDir);
    const warnings: StaticWarning[] = scanResult.ok ? scanResult.value : [];

    if (await shouldSkipUpdate(warnings, options, record.name)) {
      return skipSkill(result, options, record.name);
    }

    // Replace the installed skill directory
    const npmEffectiveScope = record.scope as "global" | "project";
    const installDir = record.active === false
      ? skillDisabledDir(record.name, npmEffectiveScope, options.projectRoot)
      : skillInstallDir(record.name, npmEffectiveScope, options.projectRoot);
    const rmResult = await wrapShell(
      () => $`rm -rf ${installDir}`.quiet().then(() => undefined),
      `Failed to remove old skill directory '${record.name}'`,
    );
    if (!rmResult.ok) return rmResult;

    await mkdir(dirname(installDir), { recursive: true });

    const cpResult = await wrapShell(
      () => $`cp -r ${newSkillDir} ${installDir}`.quiet().then(() => undefined),
      `Failed to install updated skill '${record.name}'`,
      "Check disk space and permissions.",
    );
    if (!cpResult.ok) return cpResult;

    // Semantic scan on updated content
    if (await runUpdateSemanticScan(installDir, record.name, options)) {
      return skipSkill(result, options, record.name);
    }

    await refreshAgentSymlinks(record, options.projectRoot);

    // Re-verify trust for the new version
    const newTrust = await _resolveTrust({
      adapter: "npm",
      url: record.repo!,
      tap: record.tap,
      tarballPath: join(tmpDir, "_pkg.tgz"),
      npmPackageName: packageName,
      npmVersion: latestVersion,
      npmPublisher: record.trust?.publisher?.name,
    });

    // Update record in place
    patchRecord(installed, record, {
      ref: latestVersion,
      sha: null,
      updatedAt: new Date().toISOString(),
      trust: newTrust,
    });

    result.updated.push(record.name);
    options.onProgress?.(record.name, "updated");
    return ok(undefined);
  } finally {
    await removeTmpDir(tmpDir);
  }
}

/** Handle updates for standalone git skills (path === null; workDir is the install dir). */
async function updateGitSkill(
  record: InstalledSkill,
  installed: { skills: InstalledSkill[] },
  options: UpdateOptions,
  result: UpdateResult,
  _resolveTrust: ResolveTrustFn,
): Promise<Result<void, UserError | GitError | ScanError>> {
  const effectiveScope = record.scope as "global" | "project";
  const workDir = record.active === false
    ? skillDisabledDir(record.name, effectiveScope, options.projectRoot)
    : skillInstallDir(record.name, effectiveScope, options.projectRoot);

  const fetchResult = await fetch(workDir);
  if (!fetchResult.ok) return fetchResult;

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
async function updateGitSkillGroup(
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

    // Semantic scan on cache subdir BEFORE recopy so we can roll back cleanly on failure
    if (await runUpdateSemanticScan(skillCacheSubdir, skill.name, options)) {
      result.skipped.push(skill.name);
      options.onProgress?.(skill.name, "skipped");
      anySemanticBlocked = true;
      continue;
    }

    const recopyResult = await recopyMultiSkill(workDir, skill, options.projectRoot);
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

type SkillGroup =
  | { type: "linked"; skill: InstalledSkill }
  | { type: "npm"; skill: InstalledSkill }
  | { type: "git-standalone"; skill: InstalledSkill }
  | { type: "git-multi"; repo: string; skills: InstalledSkill[] };

/** Group skills by update strategy. Multi-skill records sharing a repo cache are grouped together. */
function groupSkillsByRepo(skills: InstalledSkill[]): SkillGroup[] {
  const multiGroups = new Map<string, InstalledSkill[]>();
  const solo: SkillGroup[] = [];

  for (const skill of skills) {
    if (skill.scope === "linked") {
      solo.push({ type: "linked", skill });
      continue;
    }
    if (skill.repo?.startsWith("npm:")) {
      solo.push({ type: "npm", skill });
      continue;
    }
    if (skill.path !== null && skill.repo) {
      const existing = multiGroups.get(skill.repo);
      if (existing) {
        existing.push(skill);
      } else {
        multiGroups.set(skill.repo, [skill]);
      }
    } else {
      solo.push({ type: "git-standalone", skill });
    }
  }

  const groups: SkillGroup[] = [...solo];
  for (const [repo, skills] of multiGroups) {
    groups.push({ type: "git-multi", repo, skills });
  }
  return groups;
}

async function runUpdatePass(
  skills: InstalledSkill[],
  installed: InstalledJson,
  options: UpdateOptions,
  result: UpdateResult,
  _resolveTrust: ResolveTrustFn,
): Promise<Result<void, UserError | GitError | ScanError | NetworkError>> {
  const groups = groupSkillsByRepo(skills);

  for (const group of groups) {
    if (group.type === "linked") {
      options.onProgress?.(group.skill.name, "linked");
      continue;
    }

    if (group.type === "npm") {
      options.onProgress?.(group.skill.name, "checking");
      const r = await updateNpmSkill(group.skill, installed, options, result, _resolveTrust);
      if (!r.ok) return r;
    } else if (group.type === "git-standalone") {
      options.onProgress?.(group.skill.name, "checking");
      const r = await updateGitSkill(group.skill, installed, options, result, _resolveTrust);
      if (!r.ok) return r;
    } else {
      const r = await updateGitSkillGroup(group.repo, group.skills, installed, options, result, _resolveTrust);
      if (!r.ok) return r;
    }
  }

  return ok(undefined);
}

export async function updateSkill(
  options: UpdateOptions = {},
  _resolveTrust: ResolveTrustFn = resolveTrust,
): Promise<Result<UpdateResult, UserError | GitError | ScanError | NetworkError>> {
  debug("updateSkill", { name: options.name ?? "all" });

  // Load global installed
  const globalInstalledResult = await loadInstalled();
  if (!globalInstalledResult.ok) return globalInstalledResult;
  const globalInstalled = globalInstalledResult.value;

  // Optionally load project installed
  let projectInstalled: InstalledJson | null = null;
  if (options.projectRoot) {
    const r = await loadInstalled(options.projectRoot);
    if (!r.ok) return r;
    projectInstalled = r.value;
  }

  // Filter by name if specified — check both files
  let globalSkills = globalInstalled.skills;
  let projectSkills = projectInstalled?.skills ?? [];

  if (options.name) {
    globalSkills = globalSkills.filter((s) => s.name === options.name);
    projectSkills = projectSkills.filter((s) => s.name === options.name);
    if (globalSkills.length === 0 && projectSkills.length === 0) {
      return err(
        new UserError(
          `Skill '${options.name}' is not installed.`,
          "Run 'skilltap list' to see installed skills.",
        ),
      );
    }
  } else {
    globalSkills = globalSkills.filter((s) => s.active !== false);
    projectSkills = projectSkills.filter((s) => s.active !== false);
  }

  const result: UpdateResult = { updated: [], skipped: [], upToDate: [] };

  // Process global skills
  const globalPass = await runUpdatePass(globalSkills, globalInstalled, options, result, _resolveTrust);
  if (!globalPass.ok) return globalPass;

  // Process project skills
  if (projectInstalled) {
    const projectPass = await runUpdatePass(projectSkills, projectInstalled, { ...options, projectRoot: options.projectRoot }, result, _resolveTrust);
    if (!projectPass.ok) return projectPass;
  }

  // Save both files
  const globalSave = await saveInstalled(globalInstalled);
  if (!globalSave.ok) return globalSave;

  if (projectInstalled && options.projectRoot) {
    const projectSave = await saveInstalled(projectInstalled, options.projectRoot);
    if (!projectSave.ok) return projectSave;
  }

  return ok(result);
}
