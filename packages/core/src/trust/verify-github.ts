import { join } from "node:path";
import { $ } from "bun";

export interface GitHubTrustData {
  owner: string;
  repo: string;
  workflow?: string;
  verifiedAt: string;
}

interface GhAttestationResult {
  verificationResult?: {
    statement?: {
      predicate?: {
        buildDefinition?: {
          externalParameters?: {
            workflow?: { path?: string };
          };
        };
      };
    };
  };
}

/**
 * Verify GitHub attestation for a skill directory using the `gh` CLI.
 * Returns null if `gh` is not on PATH, no attestation exists, or verification fails.
 */
export async function verifyGitHubAttestation(
  skillDir: string,
  githubRepo: string,
): Promise<GitHubTrustData | null> {
  const [owner, repo] = githubRepo.split("/");
  if (!owner || !repo) return null;

  try {
    // Check if gh CLI is available
    let ghPath: string;
    try {
      const result = await $`which gh`.quiet();
      ghPath = result.stdout.toString().trim();
      if (!ghPath) return null;
    } catch {
      return null;
    }

    // Find first SKILL.md in the skill directory
    const skillMd = join(skillDir, "SKILL.md");
    const skillMdExists = await Bun.file(skillMd).exists();
    if (!skillMdExists) return null;

    // Run gh attestation verify
    let raw: string;
    try {
      const result =
        await $`${ghPath} attestation verify ${skillMd} --repo ${owner}/${repo} --format json`.quiet();
      raw = result.stdout.toString().trim();
    } catch {
      return null;
    }

    let results: GhAttestationResult[];
    try {
      results = JSON.parse(raw) as GhAttestationResult[];
    } catch {
      return null;
    }

    if (!Array.isArray(results) || results.length === 0) return null;

    const first = results[0];
    const workflow =
      first?.verificationResult?.statement?.predicate?.buildDefinition
        ?.externalParameters?.workflow?.path;

    return {
      owner,
      repo,
      workflow,
      verifiedAt: new Date().toISOString(),
    };
  } catch {
    return null;
  }
}

/** Parse a GitHub URL into { owner, repo } or null. */
export function parseGitHubRepo(url: string): string | null {
  // Handles: https://github.com/owner/repo, https://github.com/owner/repo.git
  // github:owner/repo, git@github.com:owner/repo.git
  let match = url.match(/github\.com[/:]([\w.-]+)\/([\w.-]+?)(?:\.git)?(?:\/.*)?$/);
  if (!match) {
    // github:owner/repo shorthand
    match = url.match(/^github:([\w.-]+)\/([\w.-]+?)(?:\.git)?(?:\/.*)?$/);
  }
  if (!match) return null;
  return `${match[1]}/${match[2]}`;
}
