import { loadSkillState, saveSkillState } from "../config";
import type { InstalledSkill } from "../schemas/installed";
import type { Result, UserError } from "../types";
import { ok } from "../types";

export interface ApplyChangeOptions {
  projectRoot?: string;
  scope: "global" | "project";
  /** Returns the new array. Return null to abort without saving. */
  mutate: (current: InstalledSkill[]) => InstalledSkill[] | null;
  manifestSync?: {
    onAdded?: (record: InstalledSkill, projectRoot: string) => Promise<void>;
    onRemoved?: (record: InstalledSkill, projectRoot: string) => Promise<void>;
  };
}

/**
 * Atomic load → mutate → save lifecycle for skill state.
 * Diffs before/after by name to drive manifest sync hooks.
 * Returns null-mutate as ok(before) — callers treat abort as a no-op.
 */
export async function applySkillStateChange(
  opts: ApplyChangeOptions,
): Promise<Result<InstalledSkill[], UserError>> {
  const fileRoot = opts.scope === "project" ? opts.projectRoot : undefined;

  const loadResult = await loadSkillState(fileRoot);
  if (!loadResult.ok) return loadResult;
  const before = loadResult.value;

  const after = opts.mutate(before);
  if (after === null) return ok(before);

  const saveResult = await saveSkillState(after, fileRoot);
  if (!saveResult.ok) return saveResult;

  if (opts.manifestSync && opts.projectRoot) {
    const projectRoot = opts.projectRoot;
    const beforeNames = new Set(before.map((r) => r.name));
    const afterNames = new Set(after.map((r) => r.name));
    const added = after.filter((r) => !beforeNames.has(r.name));
    const removed = before.filter((r) => !afterNames.has(r.name));
    for (const r of added) {
      await opts.manifestSync.onAdded?.(r, projectRoot).catch(() => {});
    }
    for (const r of removed) {
      await opts.manifestSync.onRemoved?.(r, projectRoot).catch(() => {});
    }
  }

  return ok(after);
}
