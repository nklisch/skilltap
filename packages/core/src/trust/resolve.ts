import type { TapSkill } from "../schemas/tap";
import type { TrustInfo } from "./types";
import { verifyGitHubAttestation, parseGitHubRepo } from "./verify-github";
import { verifyNpmProvenance } from "./verify-npm";

export type ResolveTrustParams = {
  /** Adapter that resolved the source: "npm", "github", "git", "local" */
  adapter: string;
  /** Original URL or "npm:..." source string */
  url: string;
  /** Tap name if installed from a tap */
  tap: string | null;
  /** Tap skill entry — provides trust.verified flag */
  tapSkill?: TapSkill;
  // npm-specific
  /** Path to the downloaded tarball ({tmpDir}/_pkg.tgz) */
  tarballPath?: string;
  /** npm package name (e.g. "@scope/name") */
  npmPackageName?: string;
  /** Resolved npm version (e.g. "1.2.0") */
  npmVersion?: string;
  /** npm publisher username (from registry metadata) */
  npmPublisher?: string;
  // git-specific
  /** Directory containing SKILL.md for GitHub attestation */
  skillDir?: string;
  /** "owner/repo" extracted from the source URL, or null if not GitHub */
  githubRepo?: string | null;
};

// Injectable for testing
type VerifyNpmFn = typeof verifyNpmProvenance;
type VerifyGitHubFn = typeof verifyGitHubAttestation;

/**
 * Resolve the trust tier for a skill source.
 * Never throws — returns unverified on any unexpected error.
 * Tier priority: provenance > publisher > curated > unverified
 */
export async function resolveTrust(
  params: ResolveTrustParams,
  _verifyNpm: VerifyNpmFn = verifyNpmProvenance,
  _verifyGitHub: VerifyGitHubFn = verifyGitHubAttestation,
): Promise<TrustInfo> {
  try {
    const tapStr = params.tap ?? undefined;

    // Try provenance verification first (highest tier)
    if (
      params.adapter === "npm" &&
      params.tarballPath &&
      params.npmPackageName &&
      params.npmVersion
    ) {
      const npmTrust = await _verifyNpm(
        params.npmPackageName,
        params.npmVersion,
        params.tarballPath,
      );
      if (npmTrust) {
        return {
          tier: "provenance",
          npm: npmTrust,
          publisher: {
            name: npmTrust.publisher,
            platform: "npm",
          },
          tap: tapStr,
        };
      }
    }

    if (
      params.adapter !== "npm" &&
      params.adapter !== "local" &&
      params.skillDir &&
      params.githubRepo
    ) {
      const ghTrust = await _verifyGitHub(params.skillDir, params.githubRepo);
      if (ghTrust) {
        return {
          tier: "provenance",
          github: ghTrust,
          publisher: {
            name: ghTrust.owner,
            platform: "github",
          },
          tap: tapStr,
        };
      }
    }

    // Publisher tier: identity known but not cryptographically verified
    if (params.adapter === "npm") {
      const publisherName =
        params.npmPublisher ||
        (params.npmPackageName?.startsWith("@")
          ? params.npmPackageName.split("/")[0]?.slice(1)
          : params.npmPackageName) ||
        "unknown";
      return {
        tier: "publisher",
        publisher: { name: publisherName, platform: "npm" },
        tap: tapStr,
      };
    }

    if (params.adapter !== "local") {
      // Check if it's a GitHub URL — extract owner for publisher identity
      const ghRepo =
        params.githubRepo ?? parseGitHubRepo(params.url);
      if (ghRepo) {
        const owner = ghRepo.split("/")[0] ?? "unknown";
        return {
          tier: "publisher",
          publisher: { name: owner, platform: "github" },
          tap: tapStr,
        };
      }
    }

    // Curated tier: from a tap (human-curated index)
    if (params.tap) {
      return {
        tier: "curated",
        tap: tapStr,
      };
    }

    // Unverified
    return { tier: "unverified" };
  } catch {
    return { tier: "unverified" };
  }
}
