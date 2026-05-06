import { realpath } from "node:fs/promises";
import { resolveSource } from "./adapters";
import { debug } from "./debug";
import { makeTmpDir, removeTmpDir } from "./fs";
import { clone, type GitError, revParse } from "./git";
import { detectPlugin } from "./plugin/detect";
import { type ScannedSkill, scan } from "./scanner";
import type { ResolvedSource } from "./schemas/agent";
import type { PluginManifest } from "./schemas/plugin";
import { scanStatic, type StaticWarning } from "./security";
import { err, ok, type Result, type ScanError, UserError } from "./types";

export interface TryReport {
  source: string;
  resolved: ResolvedSource;
  /** Cloned commit SHA truncated to 12 chars; null for local sources or when revParse fails. */
  sha: string | null;
  /** Detected plugin manifest, or null if the source is a skill repo. */
  plugin: PluginManifest | null;
  /** Skills found via scanner. Populated for both skill and plugin sources. */
  skills: ScannedSkill[];
  /** Warnings from static scan run on the cloned content. Empty if scan was skipped. */
  warnings: StaticWarning[];
  /** False when --skip-scan was passed. */
  scanned: boolean;
}

export interface TryOptions {
  /** Default git host for owner/repo shorthand. */
  gitHost?: string;
  /** Skip the static security scan. */
  skipScan?: boolean;
}

// Read-only preview of a source: clone (or use in-place for local), parse
// manifests, scan, format. Never writes to install paths or state.
export async function tryPreview(
  source: string,
  options: TryOptions = {},
): Promise<Result<TryReport, UserError | GitError | ScanError>> {
  const resolveResult = await resolveSource(source, options.gitHost);
  if (!resolveResult.ok) return resolveResult;
  const resolved = resolveResult.value;

  // Local sources: use the path directly — no clone, no cleanup needed.
  if (resolved.adapter === "local") {
    const contentDir = await realpath(resolved.url).catch(() => resolved.url);
    return finalize(source, resolved, contentDir, null, options);
  }

  // Remote sources: clone to a temp dir, finalize, clean up.
  const tmpResult = await makeTmpDir();
  if (!tmpResult.ok) return tmpResult;
  const tmp = tmpResult.value;

  let cleaned = false;
  const cleanup = async () => {
    if (cleaned) return;
    cleaned = true;
    try {
      await removeTmpDir(tmp);
    } catch (e) {
      debug("try: cleanup failed", { tmp, error: String(e) });
    }
  };

  try {
    const cloneResult = await clone(resolved.url, tmp, {
      branch: resolved.ref,
      depth: 1,
    });
    if (!cloneResult.ok) {
      await cleanup();
      return cloneResult;
    }

    const contentDir = await realpath(tmp).catch(() => tmp);

    const shaResult = await revParse(contentDir);
    const sha = shaResult.ok ? shaResult.value.slice(0, 12) : null;

    const result = await finalize(source, resolved, contentDir, sha, options);
    await cleanup();
    return result;
  } catch (e) {
    await cleanup();
    return err(new UserError(`try preview failed: ${e}`));
  }
}

async function finalize(
  source: string,
  resolved: ResolvedSource,
  contentDir: string,
  sha: string | null,
  options: TryOptions,
): Promise<Result<TryReport, UserError | ScanError>> {
  const pluginResult = await detectPlugin(contentDir);
  if (!pluginResult.ok) return pluginResult;

  const skills = await scan(contentDir);

  let warnings: StaticWarning[] = [];
  if (!options.skipScan) {
    const scanResult = await scanStatic(contentDir);
    if (!scanResult.ok) return scanResult;
    warnings = scanResult.value;
  }

  return ok({
    source,
    resolved,
    sha,
    plugin: pluginResult.value,
    skills,
    warnings,
    scanned: !options.skipScan,
  });
}
