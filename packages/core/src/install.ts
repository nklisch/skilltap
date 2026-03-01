import { lstat, mkdir, symlink } from "node:fs/promises";
import { dirname, join, relative } from "node:path";
import { $ } from "bun";
import { resolveSource } from "./adapters";
import { getConfigDir, loadInstalled, saveInstalled } from "./config";
import { globalBase, makeTmpDir, removeTmpDir } from "./fs";
import { clone, diff, diffStat, fetch, pull, revParse } from "./git";
import type { DiffStat } from "./git";
import type { ScannedSkill } from "./scanner";
import { scan } from "./scanner";
import type { ResolvedSource } from "./schemas/agent";
import type { InstalledSkill } from "./schemas/installed";
import type { StaticWarning } from "./security";
import { scanDiff, scanStatic } from "./security";
import { createAgentSymlinks, removeAgentSymlinks } from "./symlink";
import type { TapEntry } from "./taps";
import { loadTaps } from "./taps";
import type { Result } from "./types";
import { err, GitError, ok, type ScanError, UserError } from "./types";

export type InstallOptions = {
  scope: "global" | "project";
  projectRoot?: string;
  skillNames?: string[];
  also?: string[];
  ref?: string;
  tap?: string | null;
  skipScan?: boolean;
  /** Called before placement if warnings are found. Return true to proceed, false to abort. */
  onWarnings?: (
    warnings: StaticWarning[],
    skillName: string,
  ) => Promise<boolean>;
  /** Called after scan, before placement. Returns skill names to install. If omitted, installs all. */
  onSelectSkills?: (skills: ScannedSkill[]) => Promise<string[]>;
  /** Called when source resolves to multiple taps. Return chosen entry or null to cancel. */
  onSelectTap?: (matches: TapEntry[]) => Promise<TapEntry | null>;
};

export type LinkOptions = {
  scope: "global" | "project";
  projectRoot?: string;
  also?: string[];
};

export type InstallResult = {
  records: InstalledSkill[];
  warnings: StaticWarning[];
};

export type RemoveOptions = {
  scope?: "global" | "project" | "linked";
  projectRoot?: string;
};

function looksLikeTapName(source: string): boolean {
  if (
    source.startsWith("./") ||
    source.startsWith("/") ||
    source.startsWith("~/")
  )
    return false;
  if (/^(https?:\/\/|git@|ssh:\/\/|github:)/.test(source)) return false;
  // github shorthand: owner/repo — has a slash, handled by github adapter
  // name@ref: may contain @, handled here; owner/repo@ref: has slash so not a tap name
  const name = source.includes("@")
    ? source.slice(0, source.lastIndexOf("@"))
    : source;
  if (name.includes("/")) return false;
  return true;
}

export async function findProjectRoot(startDir?: string): Promise<string> {
  let dir = startDir ?? process.cwd();
  while (true) {
    const stat = await lstat(join(dir, ".git")).catch(() => null);
    if (stat) return dir;
    const parent = dirname(dir);
    if (parent === dir) return startDir ?? process.cwd();
    dir = parent;
  }
}

function skillInstallDir(
  name: string,
  scope: "global" | "project",
  projectRoot?: string,
): string {
  const base =
    scope === "global" ? globalBase() : (projectRoot ?? process.cwd());
  return join(base, ".agents", "skills", name);
}

function skillCacheDir(repoUrl: string): string {
  const hash = Bun.hash(repoUrl).toString(16);
  return join(getConfigDir(), "cache", hash);
}

async function runSecurityScan(
  selected: ScannedSkill[],
  onWarnings?: InstallOptions["onWarnings"],
): Promise<Result<StaticWarning[], ScanError | UserError>> {
  const allWarnings: StaticWarning[] = [];
  for (const skill of selected) {
    const scanResult = await scanStatic(skill.path);
    if (!scanResult.ok) return scanResult;
    if (scanResult.value.length > 0) {
      allWarnings.push(...scanResult.value);
      if (onWarnings) {
        const proceed = await onWarnings(scanResult.value, skill.name);
        if (!proceed) return err(new UserError("Install cancelled."));
      }
    }
  }
  return ok(allWarnings);
}

function makeRecord(
  skill: ScannedSkill,
  resolved: ResolvedSource,
  sha: string,
  path: string | null,
  options: InstallOptions,
  also: string[],
  now: string,
  effectiveTap: string | null,
  effectiveRef: string | undefined,
): InstalledSkill {
  return {
    name: skill.name,
    description: skill.description,
    repo: resolved.url,
    ref: effectiveRef ?? null,
    sha,
    scope: options.scope,
    path,
    tap: effectiveTap,
    also,
    installedAt: now,
    updatedAt: now,
  };
}

export async function installSkill(
  source: string,
  options: InstallOptions,
): Promise<Result<InstallResult, UserError | GitError | ScanError>> {
  const also = options.also ?? [];
  const allWarnings: StaticWarning[] = [];

  // 1. Check already-installed
  const installedResult = await loadInstalled();
  if (!installedResult.ok) return installedResult;
  const installed = installedResult.value;

  // 1.5. Tap pre-resolution: if source looks like a tap name (or name@ref), resolve via taps
  let effectiveSource = source;
  let effectiveTap = options.tap ?? null;
  let effectiveRef = options.ref;

  if (looksLikeTapName(source)) {
    let tapName = source;
    if (source.includes("@")) {
      const atIdx = source.lastIndexOf("@");
      tapName = source.slice(0, atIdx);
      if (!effectiveRef) effectiveRef = source.slice(atIdx + 1);
    }

    const tapsResult = await loadTaps();
    if (!tapsResult.ok) return tapsResult;
    const allSkills = tapsResult.value;

    if (allSkills.length === 0) {
      return err(
        new UserError(
          `Skill '${tapName}' not found — no taps configured.`,
          `Add a tap with 'skilltap tap add <name> <url>'`,
        ),
      );
    }

    const matches = allSkills.filter((e) => e.skill.name === tapName);
    if (matches.length === 0) {
      return err(
        new UserError(
          `Skill '${tapName}' not found in any configured tap.`,
          `Run 'skilltap find ${tapName}' to search, or check tap names with 'skilltap tap list'`,
        ),
      );
    }

    let chosen: TapEntry;
    if (matches.length === 1) {
      // biome-ignore lint/style/noNonNullAssertion: matches.length === 1 guarantees index 0 exists
      chosen = matches[0]!;
    } else if (options.onSelectTap) {
      const selected = await options.onSelectTap(matches);
      if (!selected) return err(new UserError("Install cancelled."));
      chosen = selected;
    } else {
      // biome-ignore lint/style/noNonNullAssertion: matches.length > 0 guaranteed (checked above)
      chosen = matches[0]!;
    }

    effectiveSource = chosen.skill.repo;
    effectiveTap = chosen.tapName;
  }

  // 2. Resolve source
  const resolvedResult = await resolveSource(effectiveSource);
  if (!resolvedResult.ok) return resolvedResult;
  const resolved = resolvedResult.value;

  // 3. Create temp dir and clone
  const tmpResult = await makeTmpDir();
  if (!tmpResult.ok) return tmpResult;
  const tmpDir = tmpResult.value;

  try {
    const cloneResult = await clone(resolved.url, tmpDir, {
      branch: effectiveRef,
      depth: 1,
    });
    if (!cloneResult.ok) return cloneResult;

    // 4. Get SHA
    const shaResult = await revParse(tmpDir);
    if (!shaResult.ok) return shaResult;
    const sha = shaResult.value;

    // 5. Scan for skills
    const scanned = await scan(tmpDir);
    if (scanned.length === 0) {
      return err(
        new UserError(
          `No SKILL.md found in "${source}". This repo doesn't contain any skills.`,
        ),
      );
    }

    // 6. Select skills to install
    let selectedNames: string[] | undefined = options.skillNames;
    if (!selectedNames && options.onSelectSkills) {
      selectedNames = await options.onSelectSkills(scanned);
    }
    const selected: ScannedSkill[] = selectedNames
      ? selectedNames.map((name) => {
          const found = scanned.find((s) => s.name === name);
          if (!found)
            throw new UserError(
              `Skill "${name}" not found in repo. Available: ${scanned.map((s) => s.name).join(", ")}`,
            );
          return found;
        })
      : scanned;

    // 6.5. Security scan (unless skipped)
    if (!options.skipScan) {
      const scanResult = await runSecurityScan(selected, options.onWarnings);
      if (!scanResult.ok) return scanResult;
      allWarnings.push(...scanResult.value);
    }

    // 7. Check for already-installed conflicts
    for (const skill of selected) {
      const conflict = installed.skills.find(
        (s) => s.name === skill.name && s.scope === options.scope,
      );
      if (conflict) {
        return err(
          new UserError(
            `Skill '${skill.name}' is already installed.`,
            `Use 'skilltap update ${skill.name}' to update, or 'skilltap remove ${skill.name}' first.`,
          ),
        );
      }
    }

    // 8. Determine standalone vs multi-skill
    // Standalone: single skill at repo root (skill.path === tmpDir)
    const isStandalone = scanned.length === 1 && scanned[0]?.path === tmpDir;

    // 9. Place skills
    const now = new Date().toISOString();
    const newRecords: InstalledSkill[] = [];

    if (isStandalone) {
      // biome-ignore lint/style/noNonNullAssertion: isStandalone guarantees exactly one selected skill
      const skill = selected[0]!;
      const destDir = skillInstallDir(
        skill.name,
        options.scope,
        options.projectRoot,
      );
      await mkdir(dirname(destDir), { recursive: true });
      await $`mv ${tmpDir} ${destDir}`.quiet();

      await createAgentSymlinks(
        skill.name,
        destDir,
        also,
        options.scope,
        options.projectRoot,
      );
      newRecords.push(
        makeRecord(
          skill,
          resolved,
          sha,
          null,
          options,
          also,
          now,
          effectiveTap,
          effectiveRef,
        ),
      );
    } else {
      // Multi-skill: move clone to cache, copy selected skills to install dirs
      const cacheRoot = skillCacheDir(resolved.url);
      await mkdir(dirname(cacheRoot), { recursive: true });
      await $`mv ${tmpDir} ${cacheRoot}`.quiet();

      for (const skill of selected) {
        const relPath = relative(
          cacheRoot,
          skill.path.replace(tmpDir, cacheRoot),
        );
        const skillSrcInCache = skill.path.replace(tmpDir, cacheRoot);
        const destDir = skillInstallDir(
          skill.name,
          options.scope,
          options.projectRoot,
        );
        await mkdir(dirname(destDir), { recursive: true });
        await $`cp -r ${skillSrcInCache} ${destDir}`.quiet();

        await createAgentSymlinks(
          skill.name,
          destDir,
          also,
          options.scope,
          options.projectRoot,
        );
        newRecords.push(
          makeRecord(
            skill,
            resolved,
            sha,
            relPath,
            options,
            also,
            now,
            effectiveTap,
            effectiveRef,
          ),
        );
      }
    }

    // 10. Save installed.json
    installed.skills.push(...newRecords);
    const saveResult = await saveInstalled(installed);
    if (!saveResult.ok) return saveResult;

    return ok({ records: newRecords, warnings: allWarnings });
  } catch (e) {
    if (e instanceof UserError) return err(e);
    if (e instanceof GitError) return err(e);
    return err(
      new UserError(
        `Install failed: ${e instanceof Error ? e.message : String(e)}`,
      ),
    );
  } finally {
    await removeTmpDir(tmpDir);
  }
}

export async function removeSkill(
  name: string,
  options: RemoveOptions = {},
): Promise<Result<void, UserError>> {
  const installedResult = await loadInstalled();
  if (!installedResult.ok) return installedResult;
  const installed = installedResult.value;

  const idx = installed.skills.findIndex(
    (s) => s.name === name && (!options.scope || s.scope === options.scope),
  );

  if (idx === -1) {
    return err(
      new UserError(
        `Skill '${name}' is not installed.`,
        `Run 'skilltap list' to see installed skills.`,
      ),
    );
  }

  // biome-ignore lint/style/noNonNullAssertion: idx was found via findIndex, guaranteed in range
  const record = installed.skills[idx]!;

  // Remove agent symlinks
  await removeAgentSymlinks(
    record.name,
    record.also,
    record.scope,
    options.projectRoot,
  );

  // Remove skill directory (for linked skills, record.path is the symlink location)
  const installPath =
    record.scope === "linked" && record.path !== null
      ? record.path
      : skillInstallDir(
          record.name,
          record.scope === "linked" ? "global" : record.scope,
          options.projectRoot,
        );
  await $`rm -rf ${installPath}`.quiet();

  // Remove cache if this was the last skill from the repo
  if (record.path !== null && record.repo) {
    const remainingFromSameRepo = installed.skills.filter(
      (s, i) => i !== idx && s.repo === record.repo,
    );
    if (remainingFromSameRepo.length === 0) {
      const cacheRoot = skillCacheDir(record.repo);
      await $`rm -rf ${cacheRoot}`.quiet();
    }
  }

  installed.skills.splice(idx, 1);
  const saveResult = await saveInstalled(installed);
  if (!saveResult.ok) return saveResult;

  return ok(undefined);
}

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
};

export type UpdateResult = {
  updated: string[];
  skipped: string[];
  upToDate: string[];
};

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
        // Cache missing — skip with a skipped status (user should reinstall)
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

    // Security scan on diff
    const warnings = scanDiff(diffOutput);

    if (warnings.length > 0) {
      options.onShowWarnings?.(warnings, record.name);
      if (options.strict) {
        result.skipped.push(record.name);
        options.onProgress?.(record.name, "skipped");
        continue;
      }
      const confirmed = await options.onConfirm?.(record.name, true);
      if (confirmed === false) {
        result.skipped.push(record.name);
        options.onProgress?.(record.name, "skipped");
        continue;
      }
    } else if (!options.yes) {
      const confirmed = await options.onConfirm?.(record.name, false);
      if (confirmed === false) {
        result.skipped.push(record.name);
        options.onProgress?.(record.name, "skipped");
        continue;
      }
    }

    // Apply update
    const pullResult = await pull(workDir);
    if (!pullResult.ok) return pullResult;

    // For multi-skill: re-copy skill dir to install path
    if (isMulti && record.path !== null) {
      const skillSrc = join(workDir, record.path);
      const destDir = skillInstallDir(
        record.name,
        record.scope as "global" | "project",
        options.projectRoot,
      );
      await $`rm -rf ${destDir}`.quiet();
      await mkdir(dirname(destDir), { recursive: true });
      await $`cp -r ${skillSrc} ${destDir}`.quiet();
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

    // Re-create agent symlinks (remove old then recreate — idempotent)
    if (record.also.length > 0) {
      await removeAgentSymlinks(
        record.name,
        record.also,
        record.scope as "global" | "project",
        options.projectRoot,
      );
      const installDir = skillInstallDir(
        record.name,
        record.scope as "global" | "project",
        options.projectRoot,
      );
      await createAgentSymlinks(
        record.name,
        installDir,
        record.also,
        record.scope as "global" | "project",
        options.projectRoot,
      );
    }

    result.updated.push(record.name);
    options.onProgress?.(record.name, "updated");
  }

  const saveResult = await saveInstalled(installed);
  if (!saveResult.ok) return saveResult;

  return ok(result);
}

export async function linkSkill(
  localPath: string,
  options: LinkOptions,
): Promise<Result<InstalledSkill, UserError>> {
  // 1. Validate localPath has SKILL.md
  const skillMdFile = Bun.file(join(localPath, "SKILL.md"));
  if (!(await skillMdFile.exists())) {
    return err(
      new UserError(
        `"${localPath}" does not contain SKILL.md`,
        "The path must be a valid skill directory.",
      ),
    );
  }

  // 2. Get skill name via scan
  const scanned = await scan(localPath);
  if (scanned.length === 0) {
    return err(new UserError(`No skill found in "${localPath}"`));
  }
  // biome-ignore lint/style/noNonNullAssertion: scanned.length > 0
  const skill = scanned[0]!;

  // 3. Load installed to check for conflicts
  const installedResult = await loadInstalled();
  if (!installedResult.ok) return installedResult;
  const installed = installedResult.value;

  // 4. Check already-installed
  const conflict = installed.skills.find((s) => s.name === skill.name);
  if (conflict) {
    return err(
      new UserError(
        `Skill '${skill.name}' is already installed.`,
        `Run 'skilltap remove ${skill.name}' first.`,
      ),
    );
  }

  // 5. Compute install path and create symlink
  const installPath = skillInstallDir(
    skill.name,
    options.scope,
    options.projectRoot,
  );
  await mkdir(dirname(installPath), { recursive: true });

  try {
    await symlink(localPath, installPath, "dir");
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    return err(new UserError(`Failed to create symlink: ${msg}`));
  }

  // 6. Create agent symlinks if requested
  const also = options.also ?? [];
  if (also.length > 0) {
    const symlinkResult = await createAgentSymlinks(
      skill.name,
      installPath,
      also,
      options.scope,
      options.projectRoot,
    );
    if (!symlinkResult.ok) return symlinkResult;
  }

  // 7. Build record (path = installPath = the symlink location)
  const now = new Date().toISOString();
  const record: InstalledSkill = {
    name: skill.name,
    description: skill.description,
    repo: null,
    ref: null,
    sha: null,
    scope: "linked",
    path: installPath,
    tap: null,
    also,
    installedAt: now,
    updatedAt: now,
  };

  // 8. Save installed.json
  installed.skills.push(record);
  const saveResult = await saveInstalled(installed);
  if (!saveResult.ok) return saveResult;

  return ok(record);
}
