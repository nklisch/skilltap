import { mkdir } from "node:fs/promises";
import { dirname, join, relative } from "node:path";
import { $ } from "bun";
import { resolveSource } from "./adapters";
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
  /** Called with progress during semantic scan. */
  onSemanticProgress?: (completed: number, total: number) => void;
};

export type InstallResult = {
  records: InstalledSkill[];
  warnings: StaticWarning[];
  semanticWarnings: SemanticWarning[];
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

type TapResolution = { source: string; tap: string; ref?: string };

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
    ref: effectiveRef,
  });
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
  sha: string | null,
  path: string | null,
  options: InstallOptions,
  also: string[],
  now: string,
  effectiveTap: string | null,
  effectiveRef: string | undefined,
  trust: TrustInfo | undefined,
  sourceKey?: string,
): InstalledSkill {
  return {
    name: skill.name,
    description: skill.description,
    repo: sourceKey ?? resolved.url,
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
  const also = options.also ?? [];
  const allWarnings: StaticWarning[] = [];
  const allSemanticWarnings: SemanticWarning[] = [];

  // 1. Check already-installed
  const installedResult = await loadInstalled();
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
  const resolvedResult = await resolveSource(effectiveSource);
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

    if (resolved.adapter === "npm" || resolved.adapter === "http") {
      const extractResult = await downloadAndExtract(
        resolved.url,
        tmpDir,
        resolved.integrity,
      );
      if (!extractResult.ok) return extractResult;
      contentDir = extractResult.value;
      sha = null;
    } else {
      const cloneResult = await clone(resolved.url, tmpDir, {
        branch: effectiveRef,
        depth: 1,
      });
      if (!cloneResult.ok) return cloneResult;
      contentDir = tmpDir;

      const shaResult = await revParse(tmpDir);
      if (!shaResult.ok) return shaResult;
      sha = shaResult.value;
    }

    // 5. Scan for skills
    const scanned = await scan(contentDir);
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

    // 6.6. Semantic scan
    const shouldRunSemantic =
      options.semantic ||
      (allWarnings.length > 0 &&
        options.onOfferSemantic &&
        (await options.onOfferSemantic()));

    if (shouldRunSemantic && !options.skipScan && options.agent) {
      for (const skill of selected) {
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

    // 7.5. Resolve trust (once per source, before placement)
    let trust: TrustInfo | undefined;
    {
      const tapSkillEntry = tapResult.value
        ? (await loadTaps()).ok
          ? (await loadTaps()).value?.find(
              (e) =>
                e.tapName === effectiveTap && e.skill.repo === effectiveSource,
            )
          : undefined
        : undefined;
      const npmInfo =
        resolved.adapter === "npm"
          ? parseNpmSource(effectiveSource)
          : undefined;
      trust = await resolveTrust({
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

    // 8. Determine standalone vs multi-skill
    // Standalone: single skill at content root (skill.path === contentDir)
    const isStandalone = scanned.length === 1 && scanned[0]?.path === contentDir;

    // sourceKey: identifier stored as `repo` in the installed record.
    // For npm and http, store the original source string (not the tarball URL).
    const sourceKey =
      resolved.adapter === "npm" || resolved.adapter === "http"
        ? effectiveSource
        : undefined;

    // 9. Place skills
    const now = new Date().toISOString();
    const newRecords: InstalledSkill[] = [];

    // Pre-compute placements: each entry captures the source path, destination, and copy strategy.
    // git multi-skill: move the clone to cache before building placements.
    const placements: Array<{
      skill: ScannedSkill;
      srcPath: string;
      relPath: string | null;
      destDir: string;
      useMove: boolean;
    }> = [];

    if (isStandalone) {
      // biome-ignore lint/style/noNonNullAssertion: isStandalone guarantees exactly one selected skill
      const skill = selected[0]!;
      placements.push({
        skill,
        srcPath: contentDir,
        relPath: null,
        destDir: skillInstallDir(skill.name, options.scope, options.projectRoot),
        useMove: true,
      });
    } else if (resolved.adapter === "npm" || resolved.adapter === "http") {
      // npm/http multi-skill: copy directly from extracted package (no git cache)
      for (const skill of selected) {
        placements.push({
          skill,
          srcPath: skill.path,
          relPath: relative(contentDir, skill.path),
          destDir: skillInstallDir(skill.name, options.scope, options.projectRoot),
          useMove: false,
        });
      }
    } else {
      // git multi-skill: move clone to cache first, then copy selected skills
      const cacheRoot = skillCacheDir(resolved.url);
      await mkdir(dirname(cacheRoot), { recursive: true });
      await $`mv ${contentDir} ${cacheRoot}`.quiet();
      for (const skill of selected) {
        const skillSrcInCache = skill.path.replace(contentDir, cacheRoot);
        placements.push({
          skill,
          srcPath: skillSrcInCache,
          relPath: relative(cacheRoot, skillSrcInCache),
          destDir: skillInstallDir(skill.name, options.scope, options.projectRoot),
          useMove: false,
        });
      }
    }

    for (const { skill, srcPath, relPath, destDir, useMove } of placements) {
      await mkdir(dirname(destDir), { recursive: true });
      if (useMove) {
        await $`mv ${srcPath} ${destDir}`.quiet();
      } else {
        await $`cp -r ${srcPath} ${destDir}`.quiet();
      }
      await createAgentSymlinks(skill.name, destDir, also, options.scope, options.projectRoot);
      newRecords.push(
        makeRecord(skill, resolved, sha, relPath, options, also, now, effectiveTap, finalRef, trust, sourceKey),
      );
    }

    // 10. Save installed.json
    installed.skills.push(...newRecords);
    const saveResult = await saveInstalled(installed);
    if (!saveResult.ok) return saveResult;

    return ok({
      records: newRecords,
      warnings: allWarnings,
      semanticWarnings: allSemanticWarnings,
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
