import { lstat } from "node:fs/promises";
import { join } from "node:path";
import { loadConfig, loadSkillState } from "../config";
import { loadLockfile, loadManifest, manifestExists } from "../manifest";
import { findProjectRoot } from "../paths";
import { loadPlugins } from "../plugin/state";
import { loadState } from "../state/load";
import { getStatePath } from "../state/paths";
import { detectDrift } from "../sync/drift";
import { BUILTIN_TAP } from "../taps";
import type { Result, UserError } from "../types";
import { ok } from "../types";
import type {
  StatusPlugin,
  StatusReport,
  StatusSkill,
  StatusTap,
} from "./types";

export interface GatherStatusOptions {
  /**
   * If provided, this directory is used as the project root; otherwise we
   * walk upward from cwd looking for `.git`. Set to null to skip project
   * detection (force global view).
   */
  projectRootHint?: string | null;
}

// Aggregate everything `skilltap status` needs into a single report.
// Reads from state.json (canonical). Drift only computed when a manifest exists.
export async function gatherStatus(
  options: GatherStatusOptions = {},
): Promise<Result<StatusReport, UserError>> {
  const projectRoot =
    options.projectRootHint === null
      ? null
      : (options.projectRootHint ?? (await tryProjectRoot()));

  const configResult = await loadConfig();
  if (!configResult.ok) return configResult;
  const config = configResult.value;

  const inProject = projectRoot !== null;

  // ── Manifest (project) ───────────────────────────────────────────────────
  const hasManifest = inProject ? await manifestExists(projectRoot) : false;

  // ── Scope inference ──────────────────────────────────────────────────────
  // Smart default: in a project root → project, else global. Config defaults.scope
  // overrides only if explicit; the smart-default lands in 33c (cutover).
  const scope: "global" | "project" =
    config.defaults.scope === "project"
      ? "project"
      : config.defaults.scope === "global"
        ? "global"
        : inProject
          ? "project"
          : "global";

  // ── also (from manifest if present, else config) ────────────────────────
  const also = hasManifest
    ? await manifestAlso(projectRoot, config.defaults.also)
    : config.defaults.also;

  // ── Skills + plugins (v2 state if available, v1 fallback otherwise) ─────
  const v2Path = getStatePath(
    scope === "project" ? (projectRoot ?? undefined) : undefined,
  );
  const v2Exists = await Bun.file(v2Path).exists();

  let skills: StatusSkill[];
  let plugins: StatusPlugin[];
  if (v2Exists) {
    const stateResult = await loadState(
      scope === "project" ? (projectRoot ?? undefined) : undefined,
    );
    if (!stateResult.ok) return stateResult;
    skills = stateResult.value.skills.map(skillToStatus);
    plugins = stateResult.value.plugins.map(pluginToStatus);
  } else {
    const installedResult = await loadSkillState(
      scope === "project" ? (projectRoot ?? undefined) : undefined,
    );
    const pluginsResult = await loadPlugins(
      scope === "project" ? (projectRoot ?? undefined) : undefined,
    );
    if (!installedResult.ok) return installedResult;
    if (!pluginsResult.ok) return pluginsResult;
    skills = installedResult.value.skills.map(skillToStatus);
    plugins = pluginsResult.value.plugins.map(pluginToStatus);
  }

  // ── Taps ─────────────────────────────────────────────────────────────────
  const taps: StatusTap[] = [];
  if (config.builtin_tap !== false) {
    taps.push({
      name: BUILTIN_TAP.name,
      url: BUILTIN_TAP.url,
      builtin: true,
      type: "builtin",
    });
  }
  for (const tap of config.taps) {
    taps.push({
      name: tap.name,
      url: tap.url,
      builtin: false,
      type: tap.type,
    });
  }

  // ── Drift (only if manifest exists) ──────────────────────────────────────
  let drift: StatusReport["drift"] = null;
  if (hasManifest && projectRoot) {
    const manifestResult = await loadManifest(projectRoot);
    const lockfileResult = await loadLockfile(projectRoot);
    const stateResult = await loadState(projectRoot);
    if (manifestResult.ok && lockfileResult.ok && stateResult.ok) {
      drift = detectDrift(
        manifestResult.value,
        lockfileResult.value,
        stateResult.value,
      );
    }
  }

  return ok({
    projectRoot,
    hasManifest,
    scope,
    also,
    skills,
    plugins,
    fromV2State: v2Exists,
    taps,
    drift,
  });
}

async function tryProjectRoot(): Promise<string | null> {
  const fromCwd = await findProjectRoot();
  // findProjectRoot returns cwd as fallback when no .git found; verify
  // there's actually a .git there. Use lstat — Bun.file().exists() returns
  // false for directories.
  const stat = await lstat(join(fromCwd, ".git")).catch(() => null);
  return stat ? fromCwd : null;
}

async function manifestAlso(
  projectRoot: string | null,
  fallback: string[],
): Promise<string[]> {
  if (!projectRoot) return fallback;
  const result = await loadManifest(projectRoot);
  if (!result.ok) return fallback;
  return result.value.targets.also.length > 0
    ? result.value.targets.also
    : fallback;
}

// Used by status renderers — these helpers shape installed records into
// the status-friendly types.

function skillToStatus(skill: {
  name: string;
  repo: string | null;
  ref: string | null;
  scope: "global" | "project" | "linked";
  also: string[];
  active: boolean;
}): StatusSkill {
  return {
    name: skill.name,
    scope: skill.scope,
    source: skill.repo,
    ref: skill.ref,
    also: skill.also,
    active: skill.active,
  };
}

function pluginToStatus(plugin: {
  name: string;
  repo: string | null;
  ref: string | null;
  scope: "global" | "project";
  active: boolean;
  components: { type: "skill" | "mcp" | "agent"; active: boolean }[];
}): StatusPlugin {
  const skillCount = plugin.components.filter((c) => c.type === "skill").length;
  const mcpCount = plugin.components.filter((c) => c.type === "mcp").length;
  const agentCount = plugin.components.filter((c) => c.type === "agent").length;
  const parts: string[] = [];
  if (skillCount > 0)
    parts.push(`${skillCount} skill${skillCount === 1 ? "" : "s"}`);
  if (mcpCount > 0) parts.push(`${mcpCount} MCP${mcpCount === 1 ? "" : "s"}`);
  if (agentCount > 0)
    parts.push(`${agentCount} agent${agentCount === 1 ? "" : "s"}`);
  const summary = parts.length === 0 ? "(empty)" : parts.join(", ");

  return {
    name: plugin.name,
    scope: plugin.scope,
    source: plugin.repo,
    ref: plugin.ref,
    componentCount: plugin.components.length,
    componentSummary: summary,
    active: plugin.active,
  };
}
