import type { Config } from "./schemas/config";
import type { Result } from "./types";
import { err, NetworkError, ok } from "./types";

// ---------------------------------------------------------------------------
// Registry protocol types
// ---------------------------------------------------------------------------

/** A single search result from any skill registry. */
export type RegistrySkill = {
  /** Unique identifier (registry-defined, e.g. "owner/repo/skill-name") */
  id: string;
  /** Display name */
  name: string;
  /** Short description (empty string if not provided) */
  description: string;
  /**
   * Install ref passed to `skilltap install` — must be a valid source format:
   * "owner/repo", full git URL, npm:package, etc.
   */
  source: string;
  /** Total install count (0 if unknown) */
  installs: number;
};

/** Raw API response shape that all registries must return. */
type RegistryApiResponse = {
  skills: Array<{
    id: string;
    name: string;
    description?: string;
    source: string;
    installs: number;
  }>;
};

/** A named registry that can search for skills. */
export type SkillRegistry = {
  name: string;
  search(query: string, limit: number): Promise<RegistrySkill[]>;
};

// ---------------------------------------------------------------------------
// Built-in: skills.sh
// ---------------------------------------------------------------------------

const SKILLS_SH_BASE = "https://skills.sh";

const REGISTRY_FETCH_TIMEOUT_MS = 8000;

async function fetchWithTimeout(url: string): Promise<Response> {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), REGISTRY_FETCH_TIMEOUT_MS);
  try {
    return await fetch(url, { signal: controller.signal });
  } finally {
    clearTimeout(timer);
  }
}

async function searchSkillsSh(query: string, limit: number): Promise<RegistrySkill[]> {
  try {
    const url = `${SKILLS_SH_BASE}/api/search?q=${encodeURIComponent(query)}&limit=${limit}`;
    const res = await fetchWithTimeout(url);
    if (!res.ok) return [];
    const data = (await res.json()) as RegistryApiResponse;
    if (!data.skills || !Array.isArray(data.skills)) return [];
    return data.skills.map((s) => ({
      id: s.id,
      name: s.name,
      description: s.description ?? "",
      source: s.source,
      installs: s.installs ?? 0,
    }));
  } catch {
    return [];
  }
}

export const skillsShRegistry: SkillRegistry = {
  name: "skills.sh",
  search: searchSkillsSh,
};

// ---------------------------------------------------------------------------
// Custom registry factory
// ---------------------------------------------------------------------------

/**
 * Create a registry adapter for any URL implementing the skills.sh search API:
 *   GET {url}/api/search?q={query}&limit={n}
 *   → { skills: [{ id, name, description?, source, installs }] }
 */
export function createCustomRegistry(name: string, baseUrl: string): SkillRegistry {
  const base = baseUrl.replace(/\/$/, "");
  return {
    name,
    async search(query: string, limit: number): Promise<RegistrySkill[]> {
      try {
        const url = `${base}/api/search?q=${encodeURIComponent(query)}&limit=${limit}`;
        const res = await fetchWithTimeout(url);
        if (!res.ok) return [];
        const data = (await res.json()) as RegistryApiResponse;
        if (!data.skills || !Array.isArray(data.skills)) return [];
        return data.skills.map((s) => ({
          id: s.id,
          name: s.name,
          description: s.description ?? "",
          source: s.source,
          installs: s.installs ?? 0,
        }));
      } catch {
        return [];
      }
    },
  };
}

// ---------------------------------------------------------------------------
// Resolver + search
// ---------------------------------------------------------------------------

const BUILTIN_REGISTRIES: Record<string, SkillRegistry> = {
  "skills.sh": skillsShRegistry,
};

/**
 * Returns the list of active `SkillRegistry` instances based on config.
 * Registries are returned in the order specified by `config.registry.enabled`.
 * Unknown names that have no matching `[[registry.sources]]` entry are silently skipped.
 */
export function resolveRegistries(config: Config): SkillRegistry[] {
  const enabled = config.registry?.enabled ?? ["skills.sh"];
  const sources = config.registry?.sources ?? [];

  return enabled.flatMap((name: string) => {
    if (BUILTIN_REGISTRIES[name]) return [BUILTIN_REGISTRIES[name]];
    const customSource = sources.find((s: { name: string; url: string }) => s.name === name);
    if (customSource) return [createCustomRegistry(name, customSource.url)];
    return [];
  });
}

/** A registry search result tagged with which registry it came from. */
export type RegistrySearchResult = RegistrySkill & { registryName: string };

/**
 * Search all provided registries in parallel and concatenate results.
 * Each result is tagged with the registry name. Failures are silently ignored (fail-open).
 */
export async function searchRegistries(
  query: string,
  registries: SkillRegistry[],
  limit = 20,
): Promise<RegistrySearchResult[]> {
  if (registries.length === 0) return [];
  const all = await Promise.all(
    registries.map(async (r) => {
      const results = await r.search(query, limit);
      return results.map((skill) => ({ ...skill, registryName: r.name }));
    }),
  );
  return all.flat();
}

// ---------------------------------------------------------------------------
// Legacy export (backwards compat for any direct callers)
// ---------------------------------------------------------------------------

/** @deprecated Use searchRegistries with resolveRegistries instead */
export type SkillsRegistryResult = RegistrySkill;

/** @deprecated Use searchRegistries with resolveRegistries instead */
export async function searchSkillsRegistry(
  query: string,
  limit = 20,
): Promise<Result<RegistrySkill[], NetworkError>> {
  const results = await searchSkillsSh(query, limit);
  return ok(results);
}
