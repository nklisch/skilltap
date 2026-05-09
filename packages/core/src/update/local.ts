import type { InstalledSkill } from "../schemas/installed";
import type { Result } from "../types";
import { ok } from "../types";
import type { UpdateOptions, UpdateResult } from "./types";

/** Handle updates for local path skills (no remote; always "local" status). */
export function updateLocalSkill(
  record: InstalledSkill,
  options: UpdateOptions,
  result: UpdateResult,
): Result<void, never> {
  result.upToDate.push(record.name);
  options.onProgress?.(record.name, "local");
  return ok(undefined);
}
