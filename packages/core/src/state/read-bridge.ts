import { loadInstalled } from "../config";
import { loadPlugins } from "../plugin/state";
import type { InstalledJson } from "../schemas/installed";
import type { PluginsJson } from "../schemas/plugins";
import { ok, type Result, type UserError } from "../types";
import { loadState } from "./load";

/**
 * Phase 31c-c-2b: read-side cutover bridge.
 *
 * Returns the v0.x-shaped `InstalledJson` so callers don't need to change.
 * Read priority:
 * 1. state.json (v2 source of truth — populated by Phase 31c-c-2a's
 *    dual-write on every install/update/remove)
 * 2. Fallback to installed.json for unmigrated v0.x users whose state.json
 *    is still empty
 *
 * After the next dual-write, state.json catches up and the fallback path
 * stops firing — making this an auto-healing read source.
 */
export async function loadActiveInstalled(
  scope: "global" | "project",
  projectRoot?: string,
): Promise<Result<InstalledJson, UserError>> {
  const root = scope === "project" ? projectRoot : undefined;
  const stateResult = await loadState(root);
  if (!stateResult.ok) return stateResult;
  if (stateResult.value.skills.length > 0) {
    return ok({ version: 1 as const, skills: stateResult.value.skills });
  }
  return loadInstalled(root);
}

/**
 * Same shape as loadActiveInstalled, but for the `plugins.json`/`state.plugins`
 * surface used by plugin install + lifecycle code.
 */
export async function loadActivePlugins(
  scope: "global" | "project",
  projectRoot?: string,
): Promise<Result<PluginsJson, UserError>> {
  const root = scope === "project" ? projectRoot : undefined;
  const stateResult = await loadState(root);
  if (!stateResult.ok) return stateResult;
  if (stateResult.value.plugins.length > 0) {
    return ok({ version: 1 as const, plugins: stateResult.value.plugins });
  }
  return loadPlugins(root);
}
