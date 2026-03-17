import { mkdir, realpath } from "node:fs/promises";
import { dirname, join, relative } from "node:path";
import { $ } from "bun";
import { resolveSource } from "./adapters";
import { debug } from "./debug";
import type { AgentAdapter } from "./agents/types";
import { loadInstalled, saveInstalled } from "./config";
import { makeTmpDir, removeTmpDir } from "./fs";
import { checkGitInstalled, clone, revParse } from "./git";
import { downloadAndExtract, parseNpmSource } from "./npm-registry";
import { skillCacheDir, skillInstallDir } from "./paths";
import type { ScannedSkill } from "./scanner";
import { scan } from "./scanner";
import type { ResolvedSource } from "./schemas/agent";
import type { InstalledSkill } from "./schemas/installed";
import type { StaticWarning } from "./security";
import { scanStatic } from "./security";
import type { SemanticWarning } from "./security/semantic";
import { scanSemantic } from "./security/semantic";
import { wrapShell } from "./shell";
import { createAgentSymlinks } from "./symlink";
import type { TapEntry } from "./taps";
import { loadTaps } from "./taps";
import type { TrustInfo } from "./trust";
import { parseGitHubRepo, resolveTrust } from "./trust";
import type { Result } from "./types";
import {
  err,
  GitError,
  NetworkError,
  ok,
  type ScanError,
  UserError,
} from "./types";

export type InstallOptions = {
  scope: "global" | "project";
  projectRoot?: string;
  skillNames?: string[];
  also?: string[];
  ref?: string;
  tap?: string | null;
  /** Default git host for owner/repo shorthand resolution. */
  gitHost?: string;
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
  /** Pre-resolved agent adapter for semantic scanning (or undefined to skip). */
  agent?: AgentAdapter;
  /** Whether to run semantic scan (--semantic flag or config). */
  semantic?: boolean;
  /** Score threshold for semantic warnings (default 5). */
  threshold?: number;
  /** Called when semantic warnings are found. Return true to proceed, false to abort. */
  onSemanticWarnings?: (
    warnings: SemanticWarning[],
    skillName: string,
  ) => Promise<boolean>;
  /** Called after static scan finds warnings — "Run semantic scan?" prompt. */
  onOfferSemantic?: () => Promise<boolean>;
  /** Called when static scan begins for a skill. */
  onStaticScanStart?: (skillName: string) => void;
  /** Called when semantic scan begins for a skill. */
  onSemanticScanStart?: (skillName: string) => void;
  /** Called after each chunk is evaluated during semantic scan. */
  onSemanticProgress?: (completed: number, total: number, score: number, reason: string) => void;
  /** Called after all scans pass cleanly, before placement. Return false to cancel. */
  onConfirmInstall?: (skillNames: string[]) => Promise<boolean>;
  /** Called when a skill is already installed. Return "update" to update it instead, or "abort" to cancel. */
  onAlreadyInstalled?: (name: string) => Promise<"update" | "abort">;
  /** Called when deep scan is triggered (no SKILL.md at standard paths). Return false to cancel. */
  onDeepScan?: (count: number) => Promise<boolean>;
  /** Source metadata for trust-tier override resolution via composePolicyForSource. */
  source?: { tapName?: string; sourceType: "tap" | "git" | "npm" | "local" };
};

export type InstallResult = {
  records: InstalledSkill[];
  warnings: StaticWarning[];
  semanticWarnings: SemanticWarning[];
  /** Names of skills that were already installed and the user chose to update instead. */
  updates: string[];
};

function looksLikeTapName(source: string): boolean {
  if (
    source.startsWith("./") ||
    source.startsWith("/") ||
    source.startsWith("~/")
  )
    return false;
  if (/^(https?:\/\/|git@|ssh:\/\/|github:|npm:)/.test(source)) return false;
  const name = source.includes("@")
    ? source.slice(0, source.lastIndexOf("@"))
    : source;
  if (name.includes("/")) return false;
  return true;
}

type TapResolution = { source: string; tap: string; skillName: string; ref?: string };

/** If source looks like a tap name (or name@ref), resolve it via configured taps. Returns null if not a tap name. */
async function resolveTapName(
  source: string,
  ref: string | undefined,
  onSelectTap?: InstallOptions["onSelectTap"],
): Promise<Result<TapResolution | null, UserError>> {
  if (!looksLikeTapName(source)) return ok(null);

  let tapName = source;
  let effectiveRef = ref;
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
        `No taps configured. Add one with 'skilltap tap add <name> <url>'.`,
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
  } else if (onSelectTap) {
    const selected = await onSelectTap(matches);
    if (!selected) return err(new UserError("Install cancelled."));
    chosen = selected;
  } else {
    // biome-ignore lint/style/noNonNullAssertion: matches.length > 0 guaranteed (checked above)
    chosen = matches[0]!;
  }

  return ok({
    source: chosen.skill.repo,
    tap: chosen.tapName,
    skillName: tapName,
    ref: effectiveRef,
  });
}

async function runSecurityScan(
  selected: ScannedSkill[],
  onWarnings?: InstallOptions["onWarnings"],
  onStaticScanStart?: InstallOptions["onStaticScanStart"],
): Promise<Result<StaticWarning[], ScanError | UserError>> {
  const allWarnings: StaticWarning[] = [];
  for (const skill of selected) {
    onStaticScanStart?.(skill.name);
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

type Placement = {
  skill: ScannedSkill;
  srcPath: string;
  relPath: string | null;
  destDir: string;
  useMove: boolean;
};

async function buildPlacements(params: {
  isStandalone: boolean;
  adapter: string;
  selected: ScannedSkill[];
  contentDir: string;
  resolvedUrl: string;
  scope: "global" | "project";
  projectRoot?: string;
}): Promise<Placement[]> {
  const { isStandalone, adapter, selected, contentDir, resolvedUrl, scope, projectRoot } = params;
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
  } else if (adapter === "npm" || adapter === "http" || adapter === "local") {
    // npm/http/local multi-skill: copy directly from content (no git cache)
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

async function executePlacements(params: {
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
  const { placements, resolved, sha, options, also, now, effectiveTap, finalRef, trust, sourceKey, cloneUrl } = params;
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
    await createAgentSymlinks(skill.name, destDir, also, options.scope, options.projectRoot);
    records.push(
      makeRecord(skill, resolved, sha, relPath, options, also, now, effectiveTap, finalRef, trust, sourceKey, cloneUrl),
    );
  }

  return records;
}

async function resolveInstallTrust(params: {
  tapResult: Result<TapResolution | null, UserError>;
  effectiveTap: string | null;
  effectiveSource: string;
  resolved: ResolvedSource;
  tmpDir: string;
  contentDir: string;
  finalRef: string | undefined;
}): Promise<TrustInfo | undefined> {
  const { tapResult, effectiveTap, effectiveSource, resolved, tmpDir, contentDir, finalRef } = params;

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
    resolved.adapter === "npm"
      ? parseNpmSource(effectiveSource)
      : undefined;
  return resolveTrust({
    adapter: resolved.adapter,
    url: effectiveSource,
    tap: effectiveTap,
    tapSkill: tapSkillEntry?.skill,
    tarballPath:
      resolved.adapter === "npm"
        ? join(tmpDir, "_pkg.tgz")
        : undefined,
    npmPackageName: npmInfo?.name,
    npmVersion: finalRef ?? undefined,
    npmPublisher: resolved.npmPublisher,
    skillDir: resolved.adapter !== "npm" ? contentDir : undefined,
    githubRepo:
      resolved.adapter !== "npm"
        ? parseGitHubRepo(resolved.url)
        : undefined,
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
    repo: resolved.adapter === "local" && !repoUrl ? null : (sourceKey ?? repoUrl ?? resolved.url),
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

export async function installSkill(
  source: string,
  options: InstallOptions,
): Promise<Result<InstallResult, UserError | GitError | ScanError | NetworkError>> {
  debug("installSkill", { source, scope: options.scope });
  const also = options.also ?? [];
  const allWarnings: StaticWarning[] = [];
  const allSemanticWarnings: SemanticWarning[] = [];

  // 1. Check already-installed
  const fileRoot = options.scope === "project" ? options.projectRoot : undefined;
  const installedResult = await loadInstalled(fileRoot);
  if (!installedResult.ok) return installedResult;
  const installed = installedResult.value;

  // 1.5. Tap pre-resolution
  const tapResult = await resolveTapName(
    source,
    options.ref,
    options.onSelectTap,
  );
  if (!tapResult.ok) return tapResult;

  const effectiveSource = tapResult.value?.source ?? source;
  const effectiveTap = tapResult.value?.tap ?? options.tap ?? null;
  const effectiveRef = tapResult.value?.ref ?? options.ref;

  // 2. Resolve source
  const resolvedResult = await resolveSource(effectiveSource, options.gitHost);
  if (!resolvedResult.ok) return resolvedResult;

  // For npm, the adapter resolves the version — use it as the ref if none was specified
  const resolved = resolvedResult.value;
  const finalRef = effectiveRef ?? resolved.ref;

  // 2.5. Check git is installed (skip for local paths, npm, and http tarball downloads)
  if (resolved.adapter !== "local" && resolved.adapter !== "npm" && resolved.adapter !== "http") {
    const gitCheck = await checkGitInstalled();
    if (!gitCheck.ok) return gitCheck;
  }

  // 3. Create temp dir and fetch content
  const tmpResult = await makeTmpDir();
  if (!tmpResult.ok) return tmpResult;
  const tmpDir = tmpResult.value;

  try {
    // contentDir: actual root of skill content (differs for npm due to package/ subdir)
    let contentDir: string;
    let sha: string | null;
    let cloneUrl: string | undefined;

    if (resolved.adapter === "npm" || resolved.adapter === "http") {
      const extractResult = await downloadAndExtract(
        resolved.url,
        tmpDir,
        resolved.integrity,
      );
      if (!extractResult.ok) return extractResult;
      contentDir = extractResult.value;
      sha = null;
    } else if (resolved.adapter === "local") {
      // Local paths: try git clone first (preserves update capability), fall back to cp
      const isGitRepo = await $`git -C ${resolved.url} rev-parse --git-dir`.quiet().then(() => true).catch(() => false);
      if (isGitRepo) {
        const cloneResult = await clone(resolved.url, tmpDir, {
          branch: effectiveRef,
          depth: 1,
        });
        if (!cloneResult.ok) return cloneResult;
        cloneUrl = cloneResult.value.effectiveUrl;
        contentDir = tmpDir;
        const shaResult = await revParse(tmpDir);
        if (!shaResult.ok) return shaResult;
        sha = shaResult.value;
      } else {
        // Non-git local dir: copy directly
        const cpResult = await wrapShell(
          () => $`cp -a ${resolved.url}/. ${tmpDir}`.quiet().then(() => undefined),
          `Failed to copy local skill from "${resolved.url}"`,
          "Check that the path exists and is readable.",
        );
        if (!cpResult.ok) return cpResult;
        contentDir = tmpDir;
        sha = null;
      }
    } else {
      const cloneResult = await clone(resolved.url, tmpDir, {
        branch: effectiveRef,
        depth: 1,
      });
      if (!cloneResult.ok) return cloneResult;
      cloneUrl = cloneResult.value.effectiveUrl;
      contentDir = tmpDir;

      const shaResult = await revParse(tmpDir);
      if (!shaResult.ok) return shaResult;
      sha = shaResult.value;
    }

    // Resolve symlinks so scanner paths and contentDir match
    // (macOS: /tmp → /private/tmp; scanner resolves internally)
    contentDir = await realpath(contentDir).catch(() => contentDir);

    debug("content fetched", { contentDir, sha, adapter: resolved.adapter });

    // 5. Scan for skills
    const scanned = await scan(contentDir, { onDeepScan: options.onDeepScan });
    if (scanned.length === 0) {
      const sourceKind =
        resolved.adapter === "npm"
          ? "npm package"
          : resolved.adapter === "http"
            ? "HTTP registry skill"
            : "repo";
      return err(
        new UserError(
          `No SKILL.md found in "${source}". This ${sourceKind} doesn't contain any skills.`,
        ),
      );
    }

    // 6. Select skills to install
    // When resolved via tap, pre-filter to the requested skill name
    let selectedNames: string[] | undefined = options.skillNames;
    if (!selectedNames && tapResult.ok && tapResult.value) {
      selectedNames = [tapResult.value.skillName];
    }
    if (!selectedNames && options.onSelectSkills) {
      selectedNames = await options.onSelectSkills(scanned);
    }
    let selected: ScannedSkill[] = selectedNames
      ? selectedNames.map((name) => {
          const found = scanned.find((s) => s.name === name);
          if (!found)
            throw new UserError(
              `Skill "${name}" not found in repo. Available: ${scanned.map((s) => s.name).join(", ")}`,
            );
          return found;
        })
      : scanned;

    // 6.1. Check for already-installed conflicts — before running security scans
    const toUpdate: string[] = [];
    const toInstall: ScannedSkill[] = [];
    for (const skill of selected) {
      const conflict = installed.skills.find(
        (s) => s.name === skill.name && s.scope === options.scope,
      );
      if (conflict) {
        if (!options.onAlreadyInstalled) {
          return err(
            new UserError(
              `Skill '${skill.name}' is already installed.`,
              `Use 'skilltap update ${skill.name}' to update, or 'skilltap remove ${skill.name}' first.`,
            ),
          );
        }
        const action = await options.onAlreadyInstalled(skill.name);
        if (action === "abort") {
          return err(
            new UserError(
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
    // If every selected skill is already installed, skip the rest and return update list
    if (toInstall.length === 0) {
      return ok({ records: [], warnings: [], semanticWarnings: [], updates: toUpdate });
    }
    selected = toInstall;

    // 6.5. Security scan (unless skipped)
    if (!options.skipScan) {
      const scanResult = await runSecurityScan(selected, options.onWarnings, options.onStaticScanStart);
      if (!scanResult.ok) return scanResult;
      allWarnings.push(...scanResult.value);
    }

    // 6.6. Semantic scan
    const shouldRunSemantic =
      options.semantic ||
      (allWarnings.length > 0 &&
        options.onOfferSemantic &&
        (await options.onOfferSemantic()));

    if (shouldRunSemantic && !options.skipScan && options.agent) {
      for (const skill of selected) {
        options.onSemanticScanStart?.(skill.name);
        const semResult = await scanSemantic(skill.path, options.agent, {
          threshold: options.threshold,
          onProgress: options.onSemanticProgress,
        });
        if (semResult.ok && semResult.value.length > 0) {
          allSemanticWarnings.push(...semResult.value);
          if (options.onSemanticWarnings) {
            const proceed = await options.onSemanticWarnings(
              semResult.value,
              skill.name,
            );
            if (!proceed) return err(new UserError("Install cancelled."));
          }
        }
      }
    }

    // 6.7. Clean-install confirmation (fires only when no warnings were found and no --yes)
    if (allWarnings.length === 0 && allSemanticWarnings.length === 0 && options.onConfirmInstall) {
      const proceed = await options.onConfirmInstall(selected.map((s) => s.name));
      if (!proceed) return err(new UserError("Install cancelled."));
    }

    // 7.5. Resolve trust (once per source, before placement)
    const trust = await resolveInstallTrust({
      tapResult, effectiveTap, effectiveSource, resolved, tmpDir, contentDir, finalRef,
    });

    // 8. Build and execute placements
    const isStandalone = scanned.length === 1 && scanned[0]?.path === contentDir;
    const sourceKey =
      resolved.adapter === "npm" || resolved.adapter === "http"
        ? effectiveSource
        : undefined;
    const now = new Date().toISOString();

    // For placement strategy: local git repos use the git cache path (not the npm/http copy path)
    const placementAdapter = resolved.adapter === "local" && cloneUrl ? "git" : resolved.adapter;
    const placements = await buildPlacements({
      isStandalone, adapter: placementAdapter, selected, contentDir,
      resolvedUrl: resolved.url, scope: options.scope, projectRoot: options.projectRoot,
    });
    const newRecords = await executePlacements({
      placements, resolved, sha, options, also, now,
      effectiveTap, finalRef, trust, sourceKey, cloneUrl,
    });

    debug("placements complete", { installed: newRecords.map((r) => r.name) });

    // 10. Save installed.json
    installed.skills.push(...newRecords);
    const saveResult = await saveInstalled(installed, fileRoot);
    if (!saveResult.ok) return saveResult;

    return ok({
      records: newRecords,
      warnings: allWarnings,
      semanticWarnings: allSemanticWarnings,
      updates: toUpdate,
    });
  } catch (e) {
    if (e instanceof UserError) return err(e);
    if (e instanceof GitError) return err(e);
    if (e instanceof NetworkError) return err(e);
    return err(
      new UserError(
        `Install failed: ${e instanceof Error ? e.message : String(e)}`,
      ),
    );
  } finally {
    await removeTmpDir(tmpDir);
  }
}
