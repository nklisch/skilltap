import { mkdir } from "node:fs/promises";
import { dirname } from "node:path";
import { $ } from "bun";
import { loadInstalled, saveInstalled } from "./config";
import { skillInstallDir } from "./paths";
import type { InstalledSkill } from "./schemas/installed";
import { createAgentSymlinks, removeAgentSymlinks } from "./symlink";
import type { Result } from "./types";
import { err, ok, UserError } from "./types";

export type MoveTarget =
  | { scope: "global" }
  | { scope: "project"; projectRoot: string };

export type MoveOptions = {
  to: MoveTarget;
  /** The project root of the source skill, if known. Required when moving from project scope to global. */
  fromProjectRoot?: string;
  also?: string[];
};

export type MoveResult = {
  record: InstalledSkill;
  from: string;
  to: string;
};

export async function moveSkill(
  name: string,
  options: MoveOptions,
): Promise<Result<MoveResult, UserError>> {
  const targetScope = options.to.scope;
  const targetProjectRoot =
    options.to.scope === "project" ? options.to.projectRoot : undefined;

  // Look up the skill in global and project installed.json
  const globalInstalledResult = await loadInstalled();
  if (!globalInstalledResult.ok) return globalInstalledResult;
  const globalInstalled = globalInstalledResult.value;

  let record: InstalledSkill | undefined;
  let sourceProjectRoot: string | undefined;

  const globalRecord = globalInstalled.skills.find(
    (s) => s.name === name && (s.scope === "global" || s.scope === "linked"),
  );

  if (globalRecord) {
    record = globalRecord;
    sourceProjectRoot = undefined;
  } else {
    // Look for a project-scoped record. Try fromProjectRoot first (explicit source),
    // then targetProjectRoot (when moving between projects), then cwd-based detection.
    const candidateRoots: string[] = [];
    if (options.fromProjectRoot) candidateRoots.push(options.fromProjectRoot);
    if (targetProjectRoot && targetProjectRoot !== options.fromProjectRoot) {
      candidateRoots.push(targetProjectRoot);
    }
    if (candidateRoots.length === 0) {
      const { findProjectRoot } = await import("./paths");
      const detectedRoot = await findProjectRoot();
      candidateRoots.push(detectedRoot);
    }

    for (const root of candidateRoots) {
      const projectInstalledResult = await loadInstalled(root);
      if (!projectInstalledResult.ok) continue;
      const projectRecord = projectInstalledResult.value.skills.find(
        (s) => s.name === name && s.scope === "project",
      );
      if (projectRecord) {
        record = projectRecord;
        sourceProjectRoot = root;
        break;
      }
    }
  }

  if (!record) {
    return err(
      new UserError(
        `Skill '${name}' is not installed.`,
        `Run 'skilltap list' to see installed skills.`,
      ),
    );
  }

  // Check if already in target scope
  const effectiveSourceScope =
    record.scope === "linked" ? "linked" : record.scope;
  const effectiveTargetScope = targetScope;

  if (record.scope === "global" && effectiveTargetScope === "global") {
    return err(new UserError(`Skill '${name}' is already in global scope.`));
  }
  if (record.scope === "project" && effectiveTargetScope === "project") {
    return err(new UserError(`Skill '${name}' is already in project scope.`));
  }

  // Compute source path
  const sourcePath =
    effectiveSourceScope === "linked" && record.path !== null
      ? record.path
      : skillInstallDir(
          name,
          effectiveSourceScope === "linked" ? "global" : (record.scope as "global" | "project"),
          sourceProjectRoot,
        );

  // Compute destination path
  const destPath = skillInstallDir(name, effectiveTargetScope, targetProjectRoot);

  // Ensure parent dir exists
  try {
    await mkdir(dirname(destPath), { recursive: true });
  } catch (e) {
    return err(new UserError(`Failed to create target directory: ${e}`));
  }

  // Remove old agent symlinks
  await removeAgentSymlinks(name, record.also, record.scope, sourceProjectRoot);

  // Move directory
  try {
    await $`mv ${sourcePath} ${destPath}`.quiet();
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    return err(new UserError(`Failed to move skill directory: ${msg}`));
  }

  // Merge also lists
  const mergedAlso = Array.from(
    new Set([...(record.also ?? []), ...(options.also ?? [])]),
  );

  // Create new agent symlinks
  const symlinkResult = await createAgentSymlinks(
    name,
    destPath,
    mergedAlso,
    effectiveTargetScope,
    targetProjectRoot,
  );
  if (!symlinkResult.ok) return symlinkResult;

  // Update records: remove from source, add to target
  const now = new Date().toISOString();
  const newRecord: InstalledSkill = {
    ...record,
    scope: effectiveTargetScope,
    path: null, // After move, path is always null (uses skillInstallDir)
    also: mergedAlso,
    updatedAt: now,
  };

  // Remove from source installed.json
  const sourceFileRoot = record.scope === "project" ? sourceProjectRoot : undefined;
  const sourceInstalledResult = await loadInstalled(sourceFileRoot);
  if (!sourceInstalledResult.ok) return sourceInstalledResult;
  const sourceInstalled = sourceInstalledResult.value;
  const sourceIdx = sourceInstalled.skills.findIndex(
    (s) => s.name === name && s.scope === record!.scope,
  );
  if (sourceIdx !== -1) {
    sourceInstalled.skills.splice(sourceIdx, 1);
  }
  const saveSourceResult = await saveInstalled(sourceInstalled, sourceFileRoot);
  if (!saveSourceResult.ok) return saveSourceResult;

  // Add to target installed.json
  const targetFileRoot = effectiveTargetScope === "project" ? targetProjectRoot : undefined;
  const targetInstalledResult = await loadInstalled(targetFileRoot);
  if (!targetInstalledResult.ok) return targetInstalledResult;
  const targetInstalled = targetInstalledResult.value;
  targetInstalled.skills.push(newRecord);
  const saveTargetResult = await saveInstalled(targetInstalled, targetFileRoot);
  if (!saveTargetResult.ok) return saveTargetResult;

  return ok({ record: newRecord, from: sourcePath, to: destPath });
}
