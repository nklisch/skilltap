import { resolveSource } from "../adapters";
import { loadSkillState, saveSkillState } from "../config";
import { debug } from "../debug";
import { makeTmpDir, removeTmpDir } from "../fs";
import { checkGitInstalled } from "../git";
import { addSkillToManifest } from "../manifest/update";
import { purgeOrphansWithCallback } from "../orphan";
import { detectPlugin } from "../plugin/detect";
import { scan } from "../scanner";
import type { Result, ScanError } from "../types";
import { err, ok, UserError, GitError, NetworkError } from "../types";
import { fetchContent } from "./fetch";
import { buildPlacements, executePlacements, resolveInstallTrust } from "./place";
import { resolveTapName } from "./resolve";
import { selectAndScan } from "./scan";
import { installPluginFromContent, resolveTapPluginInstall } from "./tap-plugin";
import type { InstallOptions, InstallResult } from "./types";

export async function installSkill(
  source: string,
  options: InstallOptions,
): Promise<
  Result<InstallResult, UserError | GitError | ScanError | NetworkError>
> {
  debug("installSkill", { source, scope: options.scope });
  const also = options.also ?? [];

  // 1. Check already-installed (reads state.json)
  const fileRoot =
    options.scope === "project" ? options.projectRoot : undefined;
  const installedResult = await loadSkillState(fileRoot);
  if (!installedResult.ok) return installedResult;
  const installed = installedResult.value;

  // 1.1. Detect and optionally purge orphan records before installing
  const purged = await purgeOrphansWithCallback(
    installed,
    fileRoot,
    options.projectRoot,
    options.onOrphansFound,
  );
  installed.splice(0, installed.length, ...purged);

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

  // 1.6. Tap plugin resolution (tap-name/plugin-name)
  if (!tapResult.value) {
    const tapPluginInstallResult = await resolveTapPluginInstall(source, options, also);
    if (tapPluginInstallResult !== null) return tapPluginInstallResult;
  }

  // 2. Resolve source
  const resolvedResult = await resolveSource(effectiveSource, options.gitHost);
  if (!resolvedResult.ok) return resolvedResult;

  // For npm, the adapter resolves the version — use it as the ref if none was specified
  const resolved = resolvedResult.value;
  const finalRef = effectiveRef ?? resolved.ref;

  // 2.5. Check git is installed (skip for local paths and npm)
  if (resolved.adapter !== "local" && resolved.adapter !== "npm") {
    const gitCheck = await checkGitInstalled();
    if (!gitCheck.ok) return gitCheck;
  }

  // 3. Create temp dir and fetch content
  const tmpResult = await makeTmpDir();
  if (!tmpResult.ok) return tmpResult;
  const tmpDir = tmpResult.value;

  try {
    // Fetch content into tmpDir (clone / npm extract / local copy)
    const fetchResult = await fetchContent(resolved, tmpDir, effectiveRef);
    if (!fetchResult.ok) return fetchResult;
    const { contentDir, sha, cloneUrl } = fetchResult.value;

    debug("content fetched", { contentDir, sha, adapter: resolved.adapter });

    // 4. Plugin detection — before skill scanning
    const selector = options.selectName ?? resolved.pluginSelector;
    const pluginResult = await detectPlugin(contentDir, {
      selectName: selector === "*" ? undefined : selector,
    });
    if (!pluginResult.ok) return pluginResult;

    if (pluginResult.value && options.onPluginDetected) {
      const decision = await options.onPluginDetected(pluginResult.value);
      if (decision === "cancel")
        return err(new UserError("Install cancelled."));
      if (decision === "plugin") {
        return await installPluginFromContent(
          contentDir,
          pluginResult.value,
          options,
          also,
          cloneUrl,
          resolved,
          finalRef,
          sha,
          effectiveTap,
        );
      }
      // decision === "skills-only" → fall through to normal skill scanning
    }

    // 5. Scan for skills
    const scanned = await scan(contentDir, { onDeepScan: options.onDeepScan });
    if (scanned.length === 0) {
      const sourceKind = resolved.adapter === "npm" ? "npm package" : "repo";
      return err(
        new UserError(
          `No SKILL.md found in "${source}". This ${sourceKind} doesn't contain any skills.`,
        ),
      );
    }

    // 6. Select, conflict-check, scan, and confirm
    const tapSkillName = tapResult.ok && tapResult.value ? tapResult.value.skillName : undefined;
    const selectResult = await selectAndScan(scanned, installed, options, tapSkillName);
    if (!selectResult.ok) return selectResult;

    const { selected, toUpdate, allWarnings, allSemanticWarnings, allAlreadyInstalled } = selectResult.value;

    if (allAlreadyInstalled) {
      return ok({ records: [], warnings: [], semanticWarnings: [], updates: toUpdate });
    }

    // 7.5. Resolve trust (once per source, before placement)
    const trust = await resolveInstallTrust({
      tapResult,
      effectiveTap,
      effectiveSource,
      resolved,
      tmpDir,
      contentDir,
      finalRef,
    });

    // 8. Build and execute placements
    const isStandalone =
      scanned.length === 1 && scanned[0]?.path === contentDir;
    const sourceKey = resolved.adapter === "npm" ? effectiveSource : undefined;
    const now = new Date().toISOString();

    // For placement strategy: local git repos use the git cache path (not the npm copy path)
    const placementAdapter =
      resolved.adapter === "local" && cloneUrl ? "git" : resolved.adapter;
    const placements = await buildPlacements({
      isStandalone,
      adapter: placementAdapter,
      selected,
      contentDir,
      resolvedUrl: resolved.url,
      scope: options.scope,
      projectRoot: options.projectRoot,
    });
    const newRecords = await executePlacements({
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
    });

    debug("placements complete", { installed: newRecords.map((r) => r.name) });

    // 10. Save state — writes to state.json.
    installed.push(...newRecords);
    const saveResult = await saveSkillState(installed, fileRoot);
    if (!saveResult.ok) return saveResult;

    // 11. v2 manifest update — no-op without skilltap.toml.
    // Only fires for project-scope installs in a project root that has a
    // manifest. Failures are non-fatal — the skill is already installed.
    if (options.scope === "project" && options.projectRoot) {
      for (const record of newRecords) {
        if (!record.repo) continue;
        await addSkillToManifest(options.projectRoot, {
          source: record.repo,
          ref: record.ref,
          sha: record.sha,
        }).catch((e) => {
          debug("manifest update failed (non-fatal)", {
            name: record.name,
            error: String(e),
          });
        });
      }
    }

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
