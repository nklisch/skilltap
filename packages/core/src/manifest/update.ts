import { ok, type Result, type UserError } from "../types";
import { loadLockfile, saveLockfile } from "./lockfile";
import { loadManifest, manifestExists } from "./load";
import { saveManifest } from "./save";
import {
  type LockEntry,
  type Lockfile,
  LockfileSchema,
  type ManifestEntry,
  type ProjectManifest,
} from "./schemas";

export interface ManifestUpdateInput {
  /** Source string preferred — falls back to record.repo. */
  source: string;
  /** Branch/tag/ref installed (the reference for the lockfile entry). */
  ref: string | null;
  /** Resolved sha at install time. */
  sha: string | null;
  /** Range to record in the manifest. Defaults to "*". */
  range?: string;
}

// Convert an install record's repo URL into the canonical manifest source key.
// - `https://github.com/owner/repo[.git]` → `github:owner/repo`
// - `git@github.com:owner/repo[.git]` → `github:owner/repo`
// - `npm:@scope/name[@version]` → unchanged
// - Everything else: passthrough.
export function canonicalizeSourceKey(repoOrSource: string): string {
  // SSH form: git@host:owner/repo[.git]
  const sshMatch = /^git@([^:]+):([^/]+)\/([^/.]+?)(?:\.git)?$/.exec(repoOrSource);
  if (sshMatch && sshMatch[1] === "github.com") {
    return `github:${sshMatch[2]}/${sshMatch[3]}`;
  }

  // HTTPS form: https://github.com/owner/repo[.git]
  const httpsMatch = /^https?:\/\/github\.com\/([^/]+)\/([^/]+?)(?:\.git)?$/.exec(repoOrSource);
  if (httpsMatch) {
    return `github:${httpsMatch[1]}/${httpsMatch[2]}`;
  }

  return repoOrSource;
}

// Append (or replace) a skill entry in skilltap.toml + skilltap.lock at the
// project root. No-op if no manifest exists. Manifest-write failures are
// converted to silent ok() — the install/plugin-install caller already
// succeeded; we don't roll back over a manifest hiccup.
export async function addSkillToManifest(
  projectRoot: string,
  input: ManifestUpdateInput,
): Promise<Result<void, UserError>> {
  return updateManifestEntry(projectRoot, input, "skills");
}

export async function addPluginToManifest(
  projectRoot: string,
  input: ManifestUpdateInput,
): Promise<Result<void, UserError>> {
  return updateManifestEntry(projectRoot, input, "plugins");
}

async function updateManifestEntry(
  projectRoot: string,
  input: ManifestUpdateInput,
  kind: "skills" | "plugins",
): Promise<Result<void, UserError>> {
  if (!(await manifestExists(projectRoot))) return ok(undefined);

  const sourceKey = canonicalizeSourceKey(input.source);
  const range = input.range ?? "*";

  const manifestResult = await loadManifest(projectRoot);
  if (!manifestResult.ok) return ok(undefined);
  const manifest = manifestResult.value;
  const updated: ProjectManifest = {
    ...manifest,
    [kind]: {
      ...manifest[kind],
      [sourceKey]: range as ManifestEntry,
    },
  };
  const saveManifestResult = await saveManifest(projectRoot, updated);
  if (!saveManifestResult.ok) return ok(undefined);

  const lockfileResult = await loadLockfile(projectRoot);
  if (!lockfileResult.ok) return ok(undefined);
  const lockfile = lockfileResult.value;

  const targetArray: "skill" | "plugin" = kind === "skills" ? "skill" : "plugin";
  const existing = lockfile[targetArray];
  const existingIdx = existing.findIndex((e) => e.source === sourceKey);
  const newEntry: LockEntry = {
    source: sourceKey,
    ref: input.ref ?? "",
    sha: input.sha ?? undefined,
    range,
  };
  const nextEntries =
    existingIdx === -1
      ? [...existing, newEntry]
      : existing.map((e, i) => (i === existingIdx ? newEntry : e));
  const nextLockfile: Lockfile = LockfileSchema.parse({
    version: 1,
    skill: targetArray === "skill" ? nextEntries : lockfile.skill,
    plugin: targetArray === "plugin" ? nextEntries : lockfile.plugin,
  });

  await saveLockfile(projectRoot, nextLockfile);
  return ok(undefined);
}
