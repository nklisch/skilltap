import { loadSkillState, saveSkillState } from "../config";
import { debug } from "../debug";
import { addSkillToManifest, manifestExists } from "../manifest";
import type { OnOrphansFound } from "../orphan";
import { purgeOrphansWithCallback } from "../orphan";
import type { InstalledSkill } from "../schemas/installed";
import { resolveTrust } from "../trust";
import type { Result, GitError, NetworkError, ScanError } from "../types";
import { err, ok, UserError } from "../types";
import { updateGitSkill, updateGitSkillGroup } from "./git";
import { updateLocalSkill } from "./local";
import { updateNpmSkill } from "./npm";
import { groupSkillsByRepo, refreshAgentSymlinks } from "./shared";
import type { ResolveTrustFn, UpdateOptions, UpdateResult } from "./types";

async function runUpdatePass(
  skills: InstalledSkill[],
  installed: { skills: InstalledSkill[] },
  options: UpdateOptions,
  result: UpdateResult,
  _resolveTrust: ResolveTrustFn,
): Promise<Result<void, UserError | GitError | ScanError | NetworkError>> {
  const groups = groupSkillsByRepo(skills);

  for (const group of groups) {
    if (group.type === "linked") {
      result.upToDate.push(group.skill.name);
      options.onProgress?.(group.skill.name, "linked");
      continue;
    }

    if (group.type === "local") {
      updateLocalSkill(group.skill, options, result);
      continue;
    }

    if (group.type === "npm") {
      options.onProgress?.(group.skill.name, "checking");
      const r = await updateNpmSkill(
        group.skill,
        installed,
        options,
        result,
        _resolveTrust,
      );
      if (!r.ok) return r;
    } else if (group.type === "git-standalone") {
      options.onProgress?.(group.skill.name, "checking");
      const r = await updateGitSkill(
        group.skill,
        installed,
        options,
        result,
        _resolveTrust,
      );
      if (!r.ok) return r;
    } else {
      const r = await updateGitSkillGroup(
        group.repo,
        group.skills,
        installed,
        options,
        result,
        _resolveTrust,
      );
      if (!r.ok) return r;
    }
  }

  return ok(undefined);
}

export async function updateSkill(
  options: UpdateOptions = {},
  _resolveTrust: ResolveTrustFn = resolveTrust,
): Promise<
  Result<UpdateResult, UserError | GitError | ScanError | NetworkError>
> {
  debug("updateSkill", { name: options.name ?? "all" });

  const globalInstalledResult = await loadSkillState();
  if (!globalInstalledResult.ok) return globalInstalledResult;
  // Wrap in a mutable container so inner functions can patch records in-place.
  const globalInstalled = { skills: globalInstalledResult.value };

  // Optionally load project installed
  let projectInstalled: { skills: InstalledSkill[] } | null = null;
  if (options.projectRoot) {
    const r = await loadSkillState(options.projectRoot);
    if (!r.ok) return r;
    projectInstalled = { skills: r.value };
  }

  // Detect and optionally purge orphan records before updating
  globalInstalled.skills = await purgeOrphansWithCallback(
    globalInstalled.skills,
    undefined,
    undefined,
    options.onOrphansFound,
  );
  if (projectInstalled) {
    projectInstalled.skills = await purgeOrphansWithCallback(
      projectInstalled.skills,
      options.projectRoot,
      options.projectRoot,
      options.onOrphansFound,
    );
  }

  // Filter by name if specified — check both files
  let globalSkills = globalInstalled.skills;
  let projectSkills = projectInstalled?.skills ?? [];

  if (options.name) {
    globalSkills = globalSkills.filter((s) => s.name === options.name);
    projectSkills = projectSkills.filter((s) => s.name === options.name);
    if (globalSkills.length === 0 && projectSkills.length === 0) {
      return err(
        new UserError(
          `Skill '${options.name}' is not installed.`,
          "Run 'skilltap status' to see installed skills.",
        ),
      );
    }
  } else {
    globalSkills = globalSkills.filter((s) => s.active !== false);
    projectSkills = projectSkills.filter((s) => s.active !== false);
  }

  const result: UpdateResult = { updated: [], skipped: [], upToDate: [] };

  // Process global skills
  const globalPass = await runUpdatePass(
    globalSkills,
    globalInstalled,
    options,
    result,
    _resolveTrust,
  );
  if (!globalPass.ok) return globalPass;

  // Process project skills
  if (projectInstalled) {
    const projectPass = await runUpdatePass(
      projectSkills,
      projectInstalled,
      { ...options, projectRoot: options.projectRoot },
      result,
      _resolveTrust,
    );
    if (!projectPass.ok) return projectPass;
  }

  // Save updated state
  const globalSave = await saveSkillState(globalInstalled.skills);
  if (!globalSave.ok) return globalSave;

  if (projectInstalled && options.projectRoot) {
    const projectSave = await saveSkillState(
      projectInstalled.skills,
      options.projectRoot,
    );
    if (!projectSave.ok) return projectSave;
  }

  // Lifecycle drift fix (Unit 3.15): refresh project manifest+lockfile entries
  // for every project-scope skill that was updated. Best-effort — failures are
  // logged and swallowed; the on-disk state has already been updated, so we
  // never roll back over a manifest hiccup.
  if (projectInstalled && options.projectRoot && result.updated.length > 0) {
    const projectRoot = options.projectRoot;
    if (await manifestExists(projectRoot)) {
      for (const name of result.updated) {
        const record = projectInstalled.skills.find((s) => s.name === name);
        if (!record || !record.repo) continue;
        await addSkillToManifest(projectRoot, {
          source: record.repo,
          ref: record.ref,
          sha: record.sha,
        }).catch((e) =>
          debug("update: addSkillToManifest failed", {
            name,
            error: String(e),
          }),
        );
      }
    }
  }

  return ok(result);
}
