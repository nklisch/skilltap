import { installSkill } from "../install";
import { canonicalizeSourceKey } from "../manifest/update";
import { installPlugin } from "../plugin/install";
import { removeInstalledPlugin } from "../plugin/lifecycle";
import { removeSkill } from "../remove";
import type { State } from "../state/schema";
import { ok, type Result, type UserError } from "../types";
import type { DriftItem, SyncPlan } from "./types";

export type ApplyStatus = "ok" | "skipped" | "fail";

export interface ApplyItemResult {
  item: DriftItem;
  status: ApplyStatus;
  error?: string;
}

export interface SyncApplyResult {
  results: ApplyItemResult[];
  applied: number;
  skipped: number;
  failed: number;
}

export interface SyncApplyOptions {
  projectRoot: string;
  /** State used to resolve a name from a source string for remove items. */
  state: State;
  /** Stop on first failure. Default false. */
  strict?: boolean;
  /** Per-item progress callback. */
  onProgress?: (item: DriftItem, status: ApplyStatus, error?: string) => void;

  // Injectable dependencies — default to real implementations.
  // Used by tests to avoid network/git operations.
  installFn?: typeof installSkill;
  removeSkillFn?: typeof removeSkill;
  installPluginFn?: typeof installPlugin;
  removeInstalledPluginFn?: typeof removeInstalledPlugin;
}

interface ApplyFns {
  installFn: typeof installSkill;
  removeSkillFn: typeof removeSkill;
  installPluginFn: typeof installPlugin;
  removeInstalledPluginFn: typeof removeInstalledPlugin;
}

export async function applySync(
  plan: SyncPlan,
  options: SyncApplyOptions,
): Promise<Result<SyncApplyResult, UserError>> {
  const fns: ApplyFns = {
    installFn: options.installFn ?? installSkill,
    removeSkillFn: options.removeSkillFn ?? removeSkill,
    installPluginFn: options.installPluginFn ?? installPlugin,
    removeInstalledPluginFn:
      options.removeInstalledPluginFn ?? removeInstalledPlugin,
  };

  const results: ApplyItemResult[] = [];
  let applied = 0;
  let skipped = 0;
  let failed = 0;

  for (const item of plan.ordered) {
    const outcome = await applyItem(item, options, fns);
    results.push({ item, status: outcome.status, error: outcome.error });
    options.onProgress?.(item, outcome.status, outcome.error);

    if (outcome.status === "ok") applied++;
    else if (outcome.status === "skipped") skipped++;
    else failed++;

    if (outcome.status === "fail" && options.strict) {
      return ok({ results, applied, skipped, failed });
    }
  }

  return ok({ results, applied, skipped, failed });
}

async function applyItem(
  item: DriftItem,
  options: SyncApplyOptions,
  fns: ApplyFns,
): Promise<{ status: ApplyStatus; error?: string }> {
  if (
    item.kind === "lock-missing" ||
    item.kind === "lock-stale" ||
    item.kind === "lock-orphan"
  ) {
    return { status: "skipped" };
  }

  if (item.target === "skill") {
    if (item.kind === "remove") return applyRemoveSkill(item, options, fns);
    if (item.kind === "add" || item.kind === "ref-mismatch") {
      return applyAddSkill(item, options, fns);
    }
  }

  if (item.target === "plugin") {
    if (item.kind === "remove") return applyRemovePlugin(item, options, fns);
    if (item.kind === "add" || item.kind === "ref-mismatch") {
      // Plugin install runs through installSkill (which auto-detects plugin).
      return applyAddSkill(item, options, fns);
    }
  }

  return { status: "skipped" };
}

async function applyAddSkill(
  item: DriftItem,
  options: SyncApplyOptions,
  fns: ApplyFns,
): Promise<{ status: ApplyStatus; error?: string }> {
  const ref = item.declared?.ref;
  const result = await fns.installFn(item.source, {
    scope: "project",
    projectRoot: options.projectRoot,
    ref,
    onSelectSkills: async (skills) => skills.map((s) => s.name),
    onWarnings: async () => !options.strict,
    onSemanticWarnings: async () => !options.strict,
    onConfirmInstall: async () => true,
    onAlreadyInstalled: async () => "update",
    onDeepScan: async () => true,
    onPluginDetected: async () => "plugin",
    onPluginWarnings: async () => !options.strict,
    onPluginConfirm: async () => true,
    // Capture: same-source captures auto-confirm (matches the plugin's pre-stated
    // intent in the manifest); cross-source conflicts hard-fail. Sync is the
    // path most vulnerable to silent substitution — a teammate cloning a repo
    // shouldn't have skills they trust replaced because the manifest declared
    // a plugin from a different author. The drift item bubbles up with
    // status: "fail" carrying the cross-source resolution hint.
    onPluginCaptureConfirm: async () => true,
    onPluginCaptureConflict: async () => "abort",
  });
  if (!result.ok) return { status: "fail", error: result.error.message };
  return { status: "ok" };
}

async function applyRemoveSkill(
  item: DriftItem,
  options: SyncApplyOptions,
  fns: ApplyFns,
): Promise<{ status: ApplyStatus; error?: string }> {
  const name = findSkillNameBySource(options.state, item.source);
  if (!name) {
    return {
      status: "fail",
      error: `state has no skill matching source ${item.source}`,
    };
  }
  const result = await fns.removeSkillFn(name, {
    scope: "project",
    projectRoot: options.projectRoot,
  });
  if (!result.ok) return { status: "fail", error: result.error.message };
  return { status: "ok" };
}

async function applyRemovePlugin(
  item: DriftItem,
  options: SyncApplyOptions,
  fns: ApplyFns,
): Promise<{ status: ApplyStatus; error?: string }> {
  const name = findPluginNameBySource(options.state, item.source);
  if (!name) {
    return {
      status: "fail",
      error: `state has no plugin matching source ${item.source}`,
    };
  }
  const result = await fns.removeInstalledPluginFn(name, {
    scope: "project",
    projectRoot: options.projectRoot,
  });
  if (!result.ok) return { status: "fail", error: result.error.message };
  return { status: "ok" };
}

function findSkillNameBySource(state: State, source: string): string | null {
  for (const s of state.skills) {
    if (s.repo && canonicalizeSourceKey(s.repo) === source) return s.name;
  }
  return null;
}

function findPluginNameBySource(state: State, source: string): string | null {
  for (const p of state.plugins) {
    if (p.repo && canonicalizeSourceKey(p.repo) === source) return p.name;
  }
  return null;
}
