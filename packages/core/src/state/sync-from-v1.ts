import { loadInstalled } from "../config";
import { loadPlugins } from "../plugin/state";
import { ok, type Result, type UserError } from "../types";
import { loadState } from "./load";
import { saveState } from "./save";
import type { State } from "./schema";

/**
 * Phase 31c-c-2a: dual-write helper.
 *
 * After install/update/remove writes installed.json + plugins.json (the v0.x
 * sources of truth), this rebuilds state.json from those files. v0.x doesn't
 * track standalone MCP servers, so any existing `state.mcpServers` (populated
 * by `mcp:` installs in Phase 35b) is preserved.
 *
 * Non-fatal by design: if any v0.x read fails or saveState fails, the caller's
 * v0.x writes already succeeded — we don't want to fail an install just
 * because the state.json shadow couldn't be refreshed. Callers fire-and-forget.
 *
 * Once the v2.1 cutover swaps install/update/remove to read state.json
 * directly, this helper is dropped and state.json becomes the sole writer.
 */
export async function syncV1ToV2State(
  scope: "global" | "project",
  projectRoot?: string,
): Promise<Result<void, UserError>> {
  const root = scope === "project" ? projectRoot : undefined;

  const installedResult = await loadInstalled(root);
  if (!installedResult.ok) return ok(undefined);
  const pluginsResult = await loadPlugins(root);
  if (!pluginsResult.ok) return ok(undefined);

  const stateResult = await loadState(root);
  const existingMcp = stateResult.ok ? stateResult.value.mcpServers : [];

  const newState: State = {
    version: 2,
    skills: installedResult.value.skills,
    plugins: pluginsResult.value.plugins,
    mcpServers: existingMcp,
  };

  return saveState(newState, root);
}
