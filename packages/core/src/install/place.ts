import { mkdir } from "node:fs/promises";
import { dirname, join, relative } from "node:path";
import { $ } from "bun";
import { skillCacheDir, skillInstallDir } from "../paths";
import { parseNpmSource } from "../npm-registry";
import type { ResolvedSource } from "../schemas/agent";
import type { InstalledSkill } from "../schemas/installed";
import type { ScannedSkill } from "../scanner";
import { wrapShell } from "../shell";
import { createAgentSymlinks } from "../symlink";
import { loadTaps } from "../taps";
import type { TapEntry } from "../taps";
import { parseGitHubRepo, resolveTrust } from "../trust";
import type { TrustInfo } from "../trust";
import type { Result, UserError } from "../types";
import { ok } from "../types";
import type { TapResolution } from "./resolve";
import type { InstallOptions } from "./types";

export type Placement = {
  skill: ScannedSkill;
  srcPath: string;
  relPath: string | null;
  destDir: string;
  useMove: boolean;
};

export async function buildPlacements(params: {
  isStandalone: boolean;
  adapter: string;
  selected: ScannedSkill[];
  contentDir: string;
  resolvedUrl: string;
  scope: "global" | "project";
  projectRoot?: string;
}): Promise<Placement[]> {
  const {
    isStandalone,
    adapter,
    selected,
    contentDir,
    resolvedUrl,
    scope,
    projectRoot,
  } = params;
  const placements: Placement[] = [];

  if (isStandalone) {
    // biome-ignore lint/style/noNonNullAssertion: isStandalone guarantees exactly one selected skill
    const skill = selected[0]!;
    placements.push({
      skill,
      srcPath: contentDir,
      relPath: null,
      destDir: skillInstallDir(skill.name, scope, projectRoot),
      useMove: true,
    });
  } else if (adapter === "npm" || adapter === "local") {
    // npm/local multi-skill: copy directly from content (no git cache)
    for (const skill of selected) {
      placements.push({
        skill,
        srcPath: skill.path,
        relPath: relative(contentDir, skill.path),
        destDir: skillInstallDir(skill.name, scope, projectRoot),
        useMove: false,
      });
    }
  } else {
    // git multi-skill: move clone to cache first, then copy selected skills
    const cacheRoot = skillCacheDir(resolvedUrl);
    await mkdir(dirname(cacheRoot), { recursive: true });
    const mvResult = await wrapShell(
      () =>
        $`rm -rf ${cacheRoot} && cp -a ${contentDir} ${cacheRoot} && rm -rf ${contentDir}`
          .quiet()
          .then(() => undefined),
      "Failed to move clone to cache",
      "Check disk space and permissions.",
    );
    if (!mvResult.ok) throw mvResult.error;
    for (const skill of selected) {
      const skillSrcInCache = skill.path.replace(contentDir, cacheRoot);
      placements.push({
        skill,
        srcPath: skillSrcInCache,
        relPath: relative(cacheRoot, skillSrcInCache),
        destDir: skillInstallDir(skill.name, scope, projectRoot),
        useMove: false,
      });
    }
  }

  return placements;
}

export async function executePlacements(params: {
  placements: Placement[];
  resolved: ResolvedSource;
  sha: string | null;
  options: InstallOptions;
  also: string[];
  now: string;
  effectiveTap: string | null;
  finalRef: string | undefined;
  trust: TrustInfo | undefined;
  sourceKey: string | undefined;
  cloneUrl?: string;
}): Promise<InstalledSkill[]> {
  const {
    placements,
    resolved,
    sha,
    options,
    also,
    now,
    effectiveTap,
    finalRef,
    trust,
    sourceKey,
    cloneUrl,
  } = params;
  const records: InstalledSkill[] = [];

  for (const { skill, srcPath, relPath, destDir, useMove } of placements) {
    await mkdir(dirname(destDir), { recursive: true });
    const op = useMove ? "move" : "copy";
    const shellResult = await wrapShell(
      () =>
        useMove
          ? $`cp -a ${srcPath} ${destDir} && rm -rf ${srcPath}`
              .quiet()
              .then(() => undefined)
          : $`cp -r ${srcPath} ${destDir}`.quiet().then(() => undefined),
      `Failed to ${op} skill '${skill.name}' to ${destDir}`,
      "Check disk space and permissions.",
    );
    if (!shellResult.ok) throw shellResult.error;
    await createAgentSymlinks(
      skill.name,
      destDir,
      also,
      options.scope,
      options.projectRoot,
    );
    records.push(
      makeRecord(
        skill,
        resolved,
        sha,
        relPath,
        options,
        also,
        now,
        effectiveTap,
        finalRef,
        trust,
        sourceKey,
        cloneUrl,
      ),
    );
  }

  return records;
}

export async function resolveInstallTrust(params: {
  tapResult: Result<TapResolution | null, UserError>;
  effectiveTap: string | null;
  effectiveSource: string;
  resolved: ResolvedSource;
  tmpDir: string;
  contentDir: string;
  finalRef: string | undefined;
}): Promise<TrustInfo | undefined> {
  const {
    tapResult,
    effectiveTap,
    effectiveSource,
    resolved,
    tmpDir,
    contentDir,
    finalRef,
  } = params;

  let tapSkillEntry: TapEntry | undefined;
  if (tapResult.ok && tapResult.value) {
    const tapsResult = await loadTaps();
    if (tapsResult.ok) {
      tapSkillEntry = tapsResult.value.find(
        (e) => e.tapName === effectiveTap && e.skill.repo === effectiveSource,
      );
    }
  }
  const npmInfo =
    resolved.adapter === "npm" ? parseNpmSource(effectiveSource) : undefined;
  return resolveTrust({
    adapter: resolved.adapter,
    url: effectiveSource,
    tap: effectiveTap,
    tapSkill: tapSkillEntry?.skill,
    tarballPath:
      resolved.adapter === "npm" ? join(tmpDir, "_pkg.tgz") : undefined,
    npmPackageName: npmInfo?.name,
    npmVersion: finalRef ?? undefined,
    npmPublisher: resolved.npmPublisher,
    skillDir: resolved.adapter !== "npm" ? contentDir : undefined,
    githubRepo:
      resolved.adapter !== "npm" ? parseGitHubRepo(resolved.url) : undefined,
  });
}

function makeRecord(
  skill: ScannedSkill,
  resolved: ResolvedSource,
  sha: string | null,
  path: string | null,
  options: InstallOptions,
  also: string[],
  now: string,
  effectiveTap: string | null,
  effectiveRef: string | undefined,
  trust: TrustInfo | undefined,
  sourceKey?: string,
  repoUrl?: string,
): InstalledSkill {
  return {
    name: skill.name,
    description: skill.description,
    repo:
      resolved.adapter === "local" && !repoUrl
        ? null
        : (sourceKey ?? repoUrl ?? resolved.url),
    ref: effectiveRef ?? null,
    sha,
    scope: options.scope,
    path,
    tap: effectiveTap,
    also,
    installedAt: now,
    updatedAt: now,
    trust,
  };
}
