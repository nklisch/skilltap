import { installPlugin } from "../plugin/install";
import type { ResolvedSource } from "../schemas/agent";
import type { PluginManifest } from "../schemas/plugin";
import { loadTaps, tapDir, tapPluginToManifest } from "../taps";
import type { TapEntry } from "../taps";
import type { Result, ScanError, UserError } from "../types";
import { err, ok, UserError as UserErrorClass } from "../types";
import { parseTapPluginRef } from "./resolve";
import type { InstallOptions, InstallResult } from "./types";

/**
 * Resolve a tap-plugin reference (tap-name/plugin-name) and install it.
 * Returns a Result<InstallResult> if a tap-plugin was matched and installed,
 * or null if no match was found (falls through to normal resolution).
 */
export async function resolveTapPluginInstall(
  source: string,
  options: InstallOptions,
  also: string[],
): Promise<Result<InstallResult, UserError | ScanError> | null> {
  const tapPluginRef = parseTapPluginRef(source);
  if (!tapPluginRef) return null;

  const tapsResult = await loadTaps();
  if (!tapsResult.ok) return null;

  const match = tapsResult.value.find(
    (e) =>
      e.tapName === tapPluginRef.tapName &&
      e.tapPlugin?.name === tapPluginRef.pluginName,
  );
  if (!match?.tapPlugin) return null;

  const tapDirPath = tapDir(tapPluginRef.tapName);
  const manifestResult = await tapPluginToManifest(match.tapPlugin, tapDirPath);
  if (!manifestResult.ok) return manifestResult;

  if (options.onPluginDetected) {
    const decision = await options.onPluginDetected(manifestResult.value);
    if (decision === "cancel") return err(new UserErrorClass("Install cancelled."));
    if (decision !== "skills-only") {
      return installTapPluginFromMatch(tapDirPath, manifestResult.value, match, tapPluginRef.tapName, options, also);
    }
    // decision === "skills-only": fall through
    return null;
  }

  // No callback — auto-install as plugin
  return installTapPluginFromMatch(tapDirPath, manifestResult.value, match, tapPluginRef.tapName, options, also);
}

/** Install a detected plugin from the cloned content directory (post-clone plugin detection path). */
export async function installPluginFromContent(
  contentDir: string,
  manifest: PluginManifest,
  options: InstallOptions,
  also: string[],
  cloneUrl: string | undefined,
  resolved: ResolvedSource,
  finalRef: string | undefined,
  sha: string | null,
  effectiveTap: string | null,
): Promise<Result<InstallResult, UserError | ScanError>> {
  const result = await installPlugin(contentDir, manifest, {
    scope: options.scope,
    projectRoot: options.projectRoot,
    also,
    skipScan: options.skipScan,
    onWarnings: options.onWarnings
      ? async (w, n) => options.onWarnings?.(w, "plugin-static", n)
      : undefined,
    onConfirm: options.onConfirmInstall
      ? async (m) => options.onConfirmInstall?.("plugin", m)
      : undefined,
    onCaptureConfirm: options.onPluginCaptureConfirm,
    onCaptureConflict: options.onPluginCaptureConflict,
    skipCapture: options.pluginSkipCapture,
    repo: cloneUrl ?? resolved.url,
    ref: finalRef ?? null,
    sha,
    tap: effectiveTap,
  });
  if (!result.ok) return result;
  return ok({
    records: [],
    warnings: result.value.warnings,
    semanticWarnings: [],
    updates: [],
    pluginRecord: result.value.record,
    captured: result.value.captured,
  });
}

export async function installTapPluginFromMatch(
  tapDirPath: string,
  manifest: PluginManifest,
  match: TapEntry,
  tapName: string,
  options: InstallOptions,
  also: string[],
): Promise<Result<InstallResult, UserError | ScanError>> {
  const result = await installPlugin(tapDirPath, manifest, {
    scope: options.scope,
    projectRoot: options.projectRoot,
    also,
    skipScan: options.skipScan,
    onWarnings: options.onWarnings
      ? async (w, n) => options.onWarnings?.(w, "plugin-static", n)
      : undefined,
    onConfirm: options.onConfirmInstall
      ? async (m) => options.onConfirmInstall?.("plugin", m)
      : undefined,
    onCaptureConfirm: options.onPluginCaptureConfirm,
    onCaptureConflict: options.onPluginCaptureConflict,
    skipCapture: options.pluginSkipCapture,
    repo: match.skill.repo ?? null,
    ref: null,
    sha: null,
    tap: tapName,
  });
  if (!result.ok) return result;
  return ok({
    records: [],
    warnings: result.value.warnings,
    semanticWarnings: [],
    updates: [],
    pluginRecord: result.value.record,
    captured: result.value.captured,
  });
}
