import { rename } from "node:fs/promises";
import { join } from "node:path";
import { getConfigDir } from "../../dirs";
import { fileExists } from "../../fs";
import type { State } from "../../state/schema";
import type { DoctorCheck, DoctorIssue } from "../types";

/**
 * Phase 31c-c-2d-2 (UX piece): detect orphaned v0.x state files.
 *
 * After Phase 31c-c-2d-1, install/update/remove write only `state.json`.
 * Existing v0.x users who upgrade still have `installed.json`/`plugins.json`
 * on disk; their first install transparently transfers the data into
 * `state.json` (via the read-fallback) but leaves the legacy files orphaned.
 *
 * This check detects that case: state.json populated AND installed.json
 * or plugins.json still on disk = orphan. `--fix` renames each orphan to
 * `<file>.v1.bak` so it's no longer read by the fallback path.
 *
 * Pre-migration users (state.json empty, installed.json populated) are NOT
 * flagged — they need the fallback to keep reading until they install
 * something or run migrate.
 */
export async function checkV1Orphans(
  state: State | null,
  projectRoot?: string,
): Promise<DoctorCheck> {
  // No populated state.json → no orphans to flag (the v0.x file is still
  // the active source via the read-fallback).
  const hasPopulatedState =
    state !== null && (state.skills.length > 0 || state.plugins.length > 0);
  if (!hasPopulatedState) {
    return {
      name: "v0.x file orphans",
      status: "pass",
      detail: "n/a (no populated v2 state)",
    };
  }

  const issues: DoctorIssue[] = [];
  const candidates: { path: string; label: string }[] = [
    {
      path: join(getConfigDir(), "installed.json"),
      label: "global installed.json",
    },
    {
      path: join(getConfigDir(), "plugins.json"),
      label: "global plugins.json",
    },
  ];
  if (projectRoot) {
    candidates.push(
      {
        path: join(projectRoot, ".agents", "installed.json"),
        label: "project installed.json",
      },
      {
        path: join(projectRoot, ".agents", "plugins.json"),
        label: "project plugins.json",
      },
    );
  }

  for (const { path, label } of candidates) {
    if (await fileExists(path)) {
      issues.push({
        message: `${label} is orphaned — your data has been moved to state.json`,
        fixable: true,
        fixDescription: `renamed to ${label}.v1.bak`,
        fix: async () => {
          await rename(path, `${path}.v1.bak`);
        },
      });
    }
  }

  if (issues.length === 0) {
    return {
      name: "v0.x file orphans",
      status: "pass",
      detail: "no orphaned v0.x files",
    };
  }

  return {
    name: "v0.x file orphans",
    status: "warn",
    detail: `${issues.length} orphaned v0.x file${issues.length === 1 ? "" : "s"}`,
    issues,
  };
}
