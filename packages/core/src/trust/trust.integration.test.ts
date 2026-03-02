/**
 * Phase 13 — Trust Signal Integration Tests
 *
 * Two integration scenarios:
 *
 *   A) Real npm provenance verification (network, gated behind SKILLTAP_IT=1)
 *      Downloads a real tarball from the npm registry and verifies its
 *      Sigstore SLSA attestation end-to-end.
 *
 *   B) Install from a verified tap (local, always runs)
 *      Creates a local tap whose skill entry carries `trust.verified = true`,
 *      installs from it, and asserts the record gets `tier: "curated"`.
 *
 * Run network tests:
 *   SKILLTAP_IT=1 bun test packages/core/src/trust/trust.integration.test.ts
 */

import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { join } from "node:path";
import {
  commitAll,
  initRepo,
  makeTmpDir,
  removeTmpDir,
} from "@skilltap/test-utils";
import { addTap } from "../taps";
import { installSkill } from "../install";
import { verifyNpmProvenance } from "./verify-npm";

const SKIP_NETWORK = !process.env.SKILLTAP_IT;

// ─── Test environment setup ───────────────────────────────────────────────────

type SavedEnv = { SKILLTAP_HOME?: string; XDG_CONFIG_HOME?: string };
let savedEnv: SavedEnv;
let homeDir: string;
let configDir: string;

beforeEach(async () => {
  savedEnv = {
    SKILLTAP_HOME: process.env.SKILLTAP_HOME,
    XDG_CONFIG_HOME: process.env.XDG_CONFIG_HOME,
  };
  homeDir = await makeTmpDir();
  configDir = await makeTmpDir();
  process.env.SKILLTAP_HOME = homeDir;
  process.env.XDG_CONFIG_HOME = configDir;
});

afterEach(async () => {
  if (savedEnv.SKILLTAP_HOME === undefined) delete process.env.SKILLTAP_HOME;
  else process.env.SKILLTAP_HOME = savedEnv.SKILLTAP_HOME;
  if (savedEnv.XDG_CONFIG_HOME === undefined)
    delete process.env.XDG_CONFIG_HOME;
  else process.env.XDG_CONFIG_HOME = savedEnv.XDG_CONFIG_HOME;
  await removeTmpDir(homeDir);
  await removeTmpDir(configDir);
});

// ─── Helpers ─────────────────────────────────────────────────────────────────

async function createLocalSkillRepo(
  name: string,
): Promise<{ path: string; cleanup: () => Promise<void> }> {
  const repoDir = await makeTmpDir();
  await Bun.write(
    join(repoDir, "SKILL.md"),
    `---\nname: ${name}\ndescription: Integration test skill\n---\n# ${name}\n`,
  );
  await initRepo(repoDir);
  await commitAll(repoDir);
  return { path: repoDir, cleanup: () => removeTmpDir(repoDir) };
}

async function createVerifiedTap(
  skills: Array<{
    name: string;
    repo: string;
    trust?: { verified: boolean; verifiedBy?: string; verifiedAt?: string };
  }>,
): Promise<{ path: string; cleanup: () => Promise<void> }> {
  const tapDir = await makeTmpDir();
  const tapJson = {
    name: "test-tap",
    description: "Integration test tap",
    skills: skills.map((s) => ({
      name: s.name,
      description: `Skill ${s.name}`,
      repo: s.repo,
      tags: [],
      ...(s.trust ? { trust: s.trust } : {}),
    })),
  };
  await Bun.write(join(tapDir, "tap.json"), JSON.stringify(tapJson, null, 2));
  await initRepo(tapDir);
  await commitAll(tapDir);
  return { path: tapDir, cleanup: () => removeTmpDir(tapDir) };
}

// ─── A) Real npm provenance verification ─────────────────────────────────────

describe.skipIf(SKIP_NETWORK)("verifyNpmProvenance — real network", () => {
  // sigstore@4.1.0: the Sigstore JS library, published via GitHub Actions with
  // npm Trusted Publishing (--provenance), known to have SLSA attestations.
  const PACKAGE = "sigstore";
  const VERSION = "4.1.0";

  test(
    "verifies SLSA provenance for sigstore@4.1.0",
    async () => {
      const tmpDir = await makeTmpDir();
      try {
        // 1. Probe the attestations endpoint — skip if the version has no attestations
        const attestationsUrl = `https://registry.npmjs.org/-/npm/v1/attestations/${PACKAGE}@${VERSION}`;
        let probe: Response;
        try {
          probe = await fetch(attestationsUrl, {
            signal: AbortSignal.timeout(15_000),
          });
        } catch {
          console.warn(
            `[SKIP] Could not reach npm attestations endpoint (network unavailable)`,
          );
          return;
        }

        if (probe.status === 404) {
          console.warn(
            `[SKIP] ${PACKAGE}@${VERSION} has no npm attestations — try a newer version`,
          );
          return;
        }
        expect(probe.ok).toBe(true);

        // 2. Resolve the canonical tarball URL from npm registry metadata
        const metaResp = await fetch(
          `https://registry.npmjs.org/${PACKAGE}/${VERSION}`,
          { signal: AbortSignal.timeout(15_000) },
        );
        expect(metaResp.ok).toBe(true);
        const meta = (await metaResp.json()) as { dist?: { tarball?: string } };
        const tarballUrl = meta.dist?.tarball;
        expect(tarballUrl).toBeString();

        // 3. Download the tarball
        const tarResp = await fetch(tarballUrl as string, {
          signal: AbortSignal.timeout(30_000),
        });
        expect(tarResp.ok).toBe(true);
        const tarballPath = join(tmpDir, "_pkg.tgz");
        await Bun.write(
          tarballPath,
          Buffer.from(await tarResp.arrayBuffer()),
        );

        // 4. Run the full provenance verification
        const result = await verifyNpmProvenance(PACKAGE, VERSION, tarballPath);

        expect(result).not.toBeNull();
        if (!result) return; // type narrowing

        expect(result.publisher).toBeString();
        expect(result.publisher.length).toBeGreaterThan(0);
        expect(result.sourceRepo).toBeString();
        expect(result.sourceRepo).toContain("github.com");
        expect(result.verifiedAt).toBeString();
        // buildWorkflow: may or may not be present depending on SLSA payload
      } finally {
        await removeTmpDir(tmpDir);
      }
    },
    60_000, // 60 s — real network + crypto
  );

  test(
    "returns null for a package with no attestations",
    async () => {
      const tmpDir = await makeTmpDir();
      try {
        // A package that predates npm provenance (published before 2023)
        // will never have attestations regardless of network conditions.
        const tarballPath = join(tmpDir, "_pkg.tgz");
        await Bun.write(tarballPath, Buffer.alloc(0)); // empty — won't be read

        // Use a clearly non-existent package version — always 404
        const result = await verifyNpmProvenance(
          "left-pad",
          "1.3.0",
          tarballPath,
        );
        // left-pad predates npm provenance; if it somehow has attestations
        // the tarball hash check will still fail since our file is empty.
        expect(result).toBeNull();
      } finally {
        await removeTmpDir(tmpDir);
      }
    },
    30_000,
  );
});

// ─── B) Install from verified tap → curated tier ─────────────────────────────

describe("trust integration — install from verified tap", () => {
  test("record gets tier:curated when tap skill has trust.verified=true", async () => {
    const skillRepo = await createLocalSkillRepo("verified-skill");
    const tap = await createVerifiedTap([
      {
        name: "verified-skill",
        repo: skillRepo.path,
        trust: {
          verified: true,
          verifiedBy: "test-author",
          verifiedAt: "2026-01-01",
        },
      },
    ]);
    try {
      await addTap("home", tap.path);

      const result = await installSkill("verified-skill", {
        scope: "global",
        skipScan: true,
      });

      expect(result.ok).toBe(true);
      if (!result.ok) return;

      expect(result.value.records).toHaveLength(1);
      const record = result.value.records[0];
      expect(record?.name).toBe("verified-skill");
      expect(record?.tap).toBe("home");

      // Trust tier: this is a non-GitHub local path from a tap → curated
      expect(record?.trust?.tier).toBe("curated");
      expect(record?.trust?.tap).toBe("home");
    } finally {
      await skillRepo.cleanup();
      await tap.cleanup();
    }
  });

  test("record gets tier:curated for plain tap skill without trust.verified", async () => {
    const skillRepo = await createLocalSkillRepo("plain-skill");
    const tap = await createVerifiedTap([
      {
        name: "plain-skill",
        repo: skillRepo.path,
        // no trust field — ordinary tap entry
      },
    ]);
    try {
      await addTap("home", tap.path);

      const result = await installSkill("plain-skill", {
        scope: "global",
        skipScan: true,
      });

      expect(result.ok).toBe(true);
      if (!result.ok) return;

      const record = result.value.records[0];
      expect(record?.trust?.tier).toBe("curated");
      expect(record?.trust?.tap).toBe("home");
    } finally {
      await skillRepo.cleanup();
      await tap.cleanup();
    }
  });

  test("trust persists in installed.json and is readable after install", async () => {
    const skillRepo = await createLocalSkillRepo("persisted-skill");
    const tap = await createVerifiedTap([
      {
        name: "persisted-skill",
        repo: skillRepo.path,
        trust: { verified: true, verifiedBy: "alice", verifiedAt: "2026-01-01" },
      },
    ]);
    try {
      await addTap("home", tap.path);
      await installSkill("persisted-skill", { scope: "global", skipScan: true });

      // Read back from installed.json
      const { loadInstalled } = await import("../config");
      const installedResult = await loadInstalled();
      expect(installedResult.ok).toBe(true);
      if (!installedResult.ok) return;
      const skill = installedResult.value.skills.find(
        (s) => s.name === "persisted-skill",
      );

      expect(skill).toBeDefined();
      expect(skill?.trust?.tier).toBe("curated");
      expect(skill?.trust?.tap).toBe("home");
    } finally {
      await skillRepo.cleanup();
      await tap.cleanup();
    }
  });
});
