import { lstat } from "node:fs/promises";
import { join } from "node:path";
import { saveSkillState } from "./config";
import { resolvedDirExists } from "./fs";
import { skillCacheDir, skillInstallDir } from "./paths";
import type { InstalledSkill } from "./schemas/installed";
import { removeAgentSymlinks } from "./symlink";
import type { Result } from "./types";
import { ok, type UserError } from "./types";

export type OrphanRecord = {
  record: InstalledSkill;
  reason:
    | "directory-missing"
    | "cache-missing"
    | "cache-subdir-missing"
    | "link-target-missing";
};

/** Called when orphan records are detected before the main operation.
 *  Return the names of records to purge. Return [] to skip cleanup. */
export type OnOrphansFound = (orphans: OrphanRecord[]) => Promise<string[]>;

export function formatOrphanReason(reason: OrphanRecord["reason"]): string {
  switch (reason) {
    case "directory-missing":
      return "install directory missing from disk";
    case "cache-missing":
      return "git cache directory missing";
    case "cache-subdir-missing":
      return "skill subdirectory removed from upstream repo";
    case "link-target-missing":
      return "symlink target no longer exists";
  }
}

/** Scan state for records whose corresponding filesystem state is missing.
 *  Pure verification — does not modify anything. */
export async function findOrphanRecords(
  skills: InstalledSkill[],
  projectRoot?: string,
): Promise<OrphanRecord[]> {
  const orphans: OrphanRecord[] = [];

  for (const record of skills) {
    // Disabled skills are expected to have missing install dirs/symlinks — skip them
    if (record.active === false) continue;

    // Linked skills: check record.path exists
    if (record.scope === "linked") {
      if (record.path && !(await resolvedDirExists(record.path))) {
        orphans.push({ record, reason: "link-target-missing" });
      }
      continue;
    }

    // npm skills
    if (record.repo?.startsWith("npm:")) {
      const installDir = skillInstallDir(
        record.name,
        record.scope as "global" | "project",
        projectRoot,
      );
      if (!(await resolvedDirExists(installDir))) {
        orphans.push({ record, reason: "directory-missing" });
      }
      continue;
    }

    // Multi-skill git skills (path !== null, repo not npm)
    if (record.path !== null && record.repo) {
      const cacheDir = skillCacheDir(record.repo);
      const cacheGitExists = await lstat(join(cacheDir, ".git"))
        .then(() => true)
        .catch(() => false);

      if (!cacheGitExists) {
        orphans.push({ record, reason: "cache-missing" });
        continue;
      }

      // Cache exists — check subdir
      const subdirExists = await resolvedDirExists(join(cacheDir, record.path));
      if (!subdirExists) {
        orphans.push({ record, reason: "cache-subdir-missing" });
        continue;
      }

      // Check install dir
      const installDir = skillInstallDir(
        record.name,
        record.scope as "global" | "project",
        projectRoot,
      );
      if (!(await resolvedDirExists(installDir))) {
        orphans.push({ record, reason: "directory-missing" });
      }
      continue;
    }

    // Standalone git, local, or anything else: check install dir
    const effectiveScope = record.scope as "global" | "project";
    const installDir = skillInstallDir(
      record.name,
      effectiveScope,
      projectRoot,
    );

    if (!(await resolvedDirExists(installDir))) {
      orphans.push({ record, reason: "directory-missing" });
    }
  }

  return orphans;
}

/** Remove orphan records from the skills list and save.
 *  Returns the names of removed records. */
export async function purgeOrphanRecords(
  orphans: OrphanRecord[],
  skills: InstalledSkill[],
  fileRoot?: string,
): Promise<Result<string[], UserError>> {
  if (orphans.length === 0) return ok([]);

  const namesToPurge = new Set(orphans.map((o) => o.record.name));

  for (const orphan of orphans) {
    await removeAgentSymlinks(
      orphan.record.name,
      orphan.record.also,
      orphan.record.scope,
      fileRoot,
    );
  }

  const remaining = skills.filter((s) => !namesToPurge.has(s.name));

  const saveResult = await saveSkillState(remaining, fileRoot);
  if (!saveResult.ok) return saveResult;

  return ok([...namesToPurge]);
}
