/**
 * Phase 13 — Trust Signal Integration Tests
 *
 * Two integration scenarios:
 *
 *   A) Real npm provenance verification (network, gated behind SKILLTAP_IT=1)
 *      Fetches a real SLSA attestation from the npm registry, downloads the
 *      tarball, and runs full end-to-end Sigstore + TUF verification.
 *
 *      Requires patches/@tufjs+models@4.1.0.patch — fixes a BoringSSL (Bun)
 *      incompatibility where @tufjs/models called crypto.verify(undefined, ...)
 *      but BoringSSL requires an explicit digest for ECDSA keys.
 *      Tracked upstream: https://github.com/sigstore/sigstore-js/pull/1561
 *
 *   B) Install from a verified tap (local, always runs)
 *      Creates a local tap whose skill entry carries `trust.verified = true`,
 *      installs from it, and asserts the record gets `tier: "curated"`.
 *
 * Run network tests:
 *   SKILLTAP_IT=1 bun test packages/core/src/trust/trust.integration.test.ts
 */

import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { createHash } from "node:crypto";
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

describe.skipIf(SKIP_NETWORK)("npm attestation — real network", () => {
  // sigstore@4.1.0: published via GitHub Actions with npm Trusted Publishing
  // (--provenance flag), known to have SLSA v1 attestations on npm.
  const PACKAGE = "sigstore";
  const VERSION = "4.1.0";

  test(
    "npm attestations endpoint returns SLSA bundle with correct structure",
    async () => {
      const tmpDir = await makeTmpDir();
      try {
        // 1. Probe the attestations endpoint
        let probe: Response;
        try {
          probe = await fetch(
            `https://registry.npmjs.org/-/npm/v1/attestations/${PACKAGE}@${VERSION}`,
            { signal: AbortSignal.timeout(15_000) },
          );
        } catch {
          console.warn("[SKIP] npm registry unreachable");
          return;
        }
        if (probe.status === 404) {
          console.warn(
            `[SKIP] ${PACKAGE}@${VERSION} has no attestations on npm`,
          );
          return;
        }
        expect(probe.ok).toBe(true);

        const data = (await probe.json()) as {
          attestations?: Array<{
            predicateType: string;
            bundle: {
              dsseEnvelope?: {
                payload?: string;
                payloadType?: string;
              };
              verificationMaterial?: {
                tlogEntries?: Array<{ logIndex?: string }>;
              };
            };
          }>;
        };
        const attestations = data.attestations ?? [];
        expect(attestations.length).toBeGreaterThan(0);

        // 2. Find the SLSA v1 attestation
        const slsa = attestations.find(
          (a) => a.predicateType === "https://slsa.dev/provenance/v1",
        );
        expect(slsa).toBeDefined();
        if (!slsa) return;

        // 3. Decode and validate the DSSE payload structure
        const dsse = slsa.bundle.dsseEnvelope;
        expect(dsse?.payload).toBeString();
        expect(dsse?.payloadType).toBe("application/vnd.in-toto+json");

        const stmt = JSON.parse(
          Buffer.from(dsse!.payload!, "base64").toString("utf-8"),
        ) as {
          _type?: string;
          subject?: Array<{ name?: string; digest?: Record<string, string> }>;
          predicateType?: string;
          predicate?: unknown;
        };

        expect(stmt._type).toBe("https://in-toto.io/Statement/v1");
        expect(stmt.subject).toHaveLength(1);
        expect(stmt.subject?.[0]?.name).toContain(PACKAGE);
        // npm SLSA attestations use sha512 digest (not sha256)
        expect(stmt.subject?.[0]?.digest?.sha512).toBeString();

        // 4. Transparency log entry should be present
        const tlog = slsa.bundle.verificationMaterial?.tlogEntries;
        expect(tlog?.length).toBeGreaterThan(0);
        expect(tlog?.[0]?.logIndex).toBeString();

        // 5. Download the tarball and verify the hash matches the attestation
        const metaResp = await fetch(
          `https://registry.npmjs.org/${PACKAGE}/${VERSION}`,
          { signal: AbortSignal.timeout(15_000) },
        );
        expect(metaResp.ok).toBe(true);
        const meta = (await metaResp.json()) as { dist?: { tarball?: string } };
        const tarballUrl = meta.dist?.tarball;
        expect(tarballUrl).toBeString();

        const tarResp = await fetch(tarballUrl as string, {
          signal: AbortSignal.timeout(30_000),
        });
        expect(tarResp.ok).toBe(true);
        const tarball = Buffer.from(await tarResp.arrayBuffer());

        const expectedHash = stmt.subject![0]!.digest!.sha512!;
        const actualHash = createHash("sha512").update(tarball).digest("hex");
        expect(actualHash).toBe(expectedHash);

        // 6. Run full end-to-end provenance verification (Sigstore + TUF)
        const tarballPath = join(tmpDir, "_pkg.tgz");
        await Bun.write(tarballPath, tarball);
        const provResult = await verifyNpmProvenance(PACKAGE, VERSION, tarballPath);

        expect(provResult).not.toBeNull();
        if (!provResult) return;
        expect(provResult.publisher).toBe("sigstore");
        expect(provResult.sourceRepo).toContain("github.com/sigstore/sigstore-js");
        expect(provResult.buildWorkflow).toBeString();
        expect(provResult.transparency).toContain("search.sigstore.dev");
        expect(provResult.verifiedAt).toBeString();
      } finally {
        await removeTmpDir(tmpDir);
      }
    },
    60_000,
  );

  test(
    "returns null gracefully for package with no attestations",
    async () => {
      const tmpDir = await makeTmpDir();
      try {
        const tarballPath = join(tmpDir, "_pkg.tgz");
        await Bun.write(tarballPath, Buffer.alloc(0));
        // left-pad predates npm provenance — 404 on attestations endpoint
        const result = await verifyNpmProvenance("left-pad", "1.3.0", tarballPath);
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
