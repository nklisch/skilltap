import { lstat } from "node:fs/promises";
import { join } from "node:path";
import { getConfigDir, loadInstalled } from "./config";
import { fetch as gitFetch, revParse } from "./git";
import { fetchPackageMetadata, parseNpmSource, resolveVersion } from "./npm-registry";
import { skillCacheDir } from "./paths";
import type { InstalledSkill } from "./schemas/installed";

type GitFetchFn = typeof gitFetch;
type RevParseFn = typeof revParse;
type FetchPackageMetadataFn = typeof fetchPackageMetadata;

interface SkillUpdateCache {
  checkedAt: string;
  updatesAvailable: string[];
  projectRoot: string | null;
}

const SKILL_CHECK_CACHE_FILE = "skills-update-check.json";

async function readSkillCheckCache(
  configDir: string,
): Promise<SkillUpdateCache | null> {
  const f = Bun.file(join(configDir, SKILL_CHECK_CACHE_FILE));
  if (!(await f.exists())) return null;
  try {
    return (await f.json()) as SkillUpdateCache;
  } catch {
    return null;
  }
}

export async function writeSkillUpdateCache(
  updatesAvailable: string[],
  projectRoot: string | null,
): Promise<void> {
  const configDir = getConfigDir();
  const file = join(configDir, SKILL_CHECK_CACHE_FILE);
  try {
    await Bun.write(
      file,
      JSON.stringify({
        checkedAt: new Date().toISOString(),
        updatesAvailable,
        projectRoot,
      }),
    );
  } catch {
    // Non-critical — ignore write failures
  }
}

/**
 * Perform a full remote check for all installed skills.
 * Groups git skills by cache dir to avoid redundant fetches.
 * Returns names of skills that have updates available.
 */
export async function fetchSkillUpdateStatus(
  projectRoot: string | null,
  _gitFetch: GitFetchFn = gitFetch,
  _revParse: RevParseFn = revParse,
  _fetchPackageMetadata: FetchPackageMetadataFn = fetchPackageMetadata,
): Promise<string[]> {
  const globalResult = await loadInstalled();
  const globalSkills = globalResult.ok ? globalResult.value.skills : [];

  const projectSkills: InstalledSkill[] = [];
  if (projectRoot) {
    const projectResult = await loadInstalled(projectRoot);
    if (projectResult.ok) projectSkills.push(...projectResult.value.skills);
  }

  const allSkills = [...globalSkills, ...projectSkills];
  const updatesAvailable: string[] = [];

  // Group git skills by cache dir (same repo = same cache dir = fetch once)
  const gitGroups = new Map<string, InstalledSkill[]>();
  const npmSkills: InstalledSkill[] = [];

  for (const skill of allSkills) {
    if (skill.scope === "linked" || !skill.repo) continue;
    if (skill.repo.startsWith("npm:")) {
      npmSkills.push(skill);
    } else {
      const cacheDir = skillCacheDir(skill.repo);
      const group = gitGroups.get(cacheDir) ?? [];
      group.push(skill);
      gitGroups.set(cacheDir, group);
    }
  }

  // Check git groups — fetch once per unique repo
  for (const [cacheDir, skills] of gitGroups) {
    const cacheGitExists = await lstat(join(cacheDir, ".git"))
      .then(() => true)
      .catch(() => false);
    if (!cacheGitExists) continue;

    const fetchResult = await _gitFetch(cacheDir);
    if (!fetchResult.ok) continue; // network error — skip gracefully

    const localResult = await _revParse(cacheDir, "HEAD");
    const remoteResult = await _revParse(cacheDir, "FETCH_HEAD");
    if (!localResult.ok || !remoteResult.ok) continue;

    if (localResult.value !== remoteResult.value) {
      for (const skill of skills) {
        updatesAvailable.push(skill.name);
      }
    }
  }

  // Check npm skills
  for (const skill of npmSkills) {
    if (!skill.sha) continue;
    const { name } = parseNpmSource(skill.repo!);
    const metaResult = await _fetchPackageMetadata(name);
    if (!metaResult.ok) continue;
    const versionResult = resolveVersion(metaResult.value, "latest");
    if (!versionResult.ok) continue;
    if (versionResult.value.version !== skill.sha) {
      updatesAvailable.push(skill.name);
    }
  }

  return updatesAvailable;
}

/**
 * Read cached skill update check result. Kicks off a background refresh if stale.
 * Returns the list of skill names with updates, or null if cache is empty / no updates.
 */
export async function checkForSkillUpdates(
  intervalHours: number,
  projectRoot: string | null,
): Promise<string[] | null> {
  const configDir = getConfigDir();
  const cache = await readSkillCheckCache(configDir);

  const isStale =
    !cache ||
    Date.now() - new Date(cache.checkedAt).getTime() >
      intervalHours * 3_600_000 ||
    cache.projectRoot !== projectRoot;

  if (isStale) {
    // Fire-and-forget — do not block the CLI
    fetchSkillUpdateStatus(projectRoot).then((updates) => {
      writeSkillUpdateCache(updates, projectRoot);
    });
  }

  if (!cache?.updatesAvailable?.length) return null;
  return cache.updatesAvailable;
}
