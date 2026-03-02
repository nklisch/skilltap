import { describe, expect, test } from "bun:test";
import type { TapSkill } from "../schemas/tap";
import type { GitHubTrustData } from "./verify-github";
import type { NpmTrustData } from "./verify-npm";
import { resolveTrust } from "./resolve";

// Mock verify functions
const npmSuccess: NpmTrustData = {
  publisher: "acme",
  sourceRepo: "https://github.com/acme/code-review",
  buildWorkflow: ".github/workflows/publish.yml",
  transparency: "https://search.sigstore.dev/?logIndex=99",
  verifiedAt: "2026-01-01T00:00:00.000Z",
};

const ghSuccess: GitHubTrustData = {
  owner: "acme",
  repo: "skill-repo",
  workflow: ".github/workflows/attest.yml",
  verifiedAt: "2026-01-01T00:00:00.000Z",
};

const verifyNpmOk = async () => npmSuccess;
const verifyNpmFail = async () => null;
const verifyGitHubOk = async () => ghSuccess;
const verifyGitHubFail = async () => null;

describe("resolveTrust — npm source", () => {
  test("provenance when npm attestation verifies", async () => {
    const trust = await resolveTrust(
      {
        adapter: "npm",
        url: "npm:@acme/code-review",
        tap: null,
        tarballPath: "/tmp/pkg.tgz",
        npmPackageName: "@acme/code-review",
        npmVersion: "1.2.0",
        npmPublisher: "acme-user",
      },
      verifyNpmOk,
      verifyGitHubFail,
    );
    expect(trust.tier).toBe("provenance");
    expect(trust.npm?.sourceRepo).toBe("https://github.com/acme/code-review");
    expect(trust.npm?.publisher).toBe("acme");
    expect(trust.publisher?.name).toBe("acme");
    expect(trust.publisher?.platform).toBe("npm");
    expect(trust.tap).toBeUndefined();
  });

  test("provenance includes tap when from a tap", async () => {
    const trust = await resolveTrust(
      {
        adapter: "npm",
        url: "npm:@acme/code-review",
        tap: "home",
        tarballPath: "/tmp/pkg.tgz",
        npmPackageName: "@acme/code-review",
        npmVersion: "1.2.0",
      },
      verifyNpmOk,
      verifyGitHubFail,
    );
    expect(trust.tier).toBe("provenance");
    expect(trust.tap).toBe("home");
  });

  test("publisher when npm attestation returns 404 (null)", async () => {
    const trust = await resolveTrust(
      {
        adapter: "npm",
        url: "npm:@acme/code-review",
        tap: null,
        tarballPath: "/tmp/pkg.tgz",
        npmPackageName: "@acme/code-review",
        npmVersion: "1.2.0",
        npmPublisher: "acme-user",
      },
      verifyNpmFail,
      verifyGitHubFail,
    );
    expect(trust.tier).toBe("publisher");
    expect(trust.publisher?.name).toBe("acme-user");
    expect(trust.publisher?.platform).toBe("npm");
  });

  test("publisher uses package scope when no npmPublisher", async () => {
    const trust = await resolveTrust(
      {
        adapter: "npm",
        url: "npm:@acme/code-review",
        tap: null,
        npmPackageName: "@acme/code-review",
        npmVersion: "1.2.0",
      },
      verifyNpmFail,
      verifyGitHubFail,
    );
    expect(trust.tier).toBe("publisher");
    expect(trust.publisher?.name).toBe("acme");
  });

  test("publisher falls back to unscoped package name", async () => {
    const trust = await resolveTrust(
      {
        adapter: "npm",
        url: "npm:my-skill",
        tap: null,
        npmPackageName: "my-skill",
        npmVersion: "1.0.0",
      },
      verifyNpmFail,
      verifyGitHubFail,
    );
    expect(trust.tier).toBe("publisher");
    expect(trust.publisher?.name).toBe("my-skill");
  });

  test("publisher when missing tarball/version params (no verification attempted)", async () => {
    const trust = await resolveTrust(
      {
        adapter: "npm",
        url: "npm:@acme/skill",
        tap: null,
        npmPublisher: "acme",
        // no tarballPath, no npmVersion
      },
      verifyNpmOk, // wouldn't be called
      verifyGitHubFail,
    );
    expect(trust.tier).toBe("publisher");
  });
});

describe("resolveTrust — git/github source", () => {
  test("provenance when GitHub attestation verifies", async () => {
    const trust = await resolveTrust(
      {
        adapter: "github",
        url: "https://github.com/acme/skill-repo",
        tap: null,
        skillDir: "/tmp/skill-dir",
        githubRepo: "acme/skill-repo",
      },
      verifyNpmFail,
      verifyGitHubOk,
    );
    expect(trust.tier).toBe("provenance");
    expect(trust.github?.owner).toBe("acme");
    expect(trust.github?.repo).toBe("skill-repo");
    expect(trust.publisher?.name).toBe("acme");
    expect(trust.publisher?.platform).toBe("github");
  });

  test("publisher when gh not on PATH (null result)", async () => {
    const trust = await resolveTrust(
      {
        adapter: "github",
        url: "https://github.com/acme/skill-repo",
        tap: null,
        skillDir: "/tmp/skill-dir",
        githubRepo: "acme/skill-repo",
      },
      verifyNpmFail,
      verifyGitHubFail,
    );
    expect(trust.tier).toBe("publisher");
    expect(trust.publisher?.name).toBe("acme");
    expect(trust.publisher?.platform).toBe("github");
  });

  test("publisher extracted from GitHub URL when githubRepo not provided", async () => {
    const trust = await resolveTrust(
      {
        adapter: "git",
        url: "https://github.com/owner/my-skill.git",
        tap: null,
        skillDir: "/tmp/skill-dir",
        // no explicit githubRepo — resolved from URL
      },
      verifyNpmFail,
      verifyGitHubFail,
    );
    expect(trust.tier).toBe("publisher");
    expect(trust.publisher?.name).toBe("owner");
  });

  test("curated when non-github git URL and from tap", async () => {
    const trust = await resolveTrust(
      {
        adapter: "git",
        url: "https://gitlab.com/user/skill.git",
        tap: "home",
        skillDir: "/tmp/skill-dir",
      },
      verifyNpmFail,
      verifyGitHubFail,
    );
    expect(trust.tier).toBe("curated");
    expect(trust.tap).toBe("home");
  });

  test("unverified when non-github git URL and no tap", async () => {
    const trust = await resolveTrust(
      {
        adapter: "git",
        url: "https://gitlab.com/user/skill.git",
        tap: null,
        skillDir: "/tmp/skill-dir",
      },
      verifyNpmFail,
      verifyGitHubFail,
    );
    expect(trust.tier).toBe("unverified");
  });
});

describe("resolveTrust — tap signals", () => {
  test("curated tier when from tap", async () => {
    const trust = await resolveTrust(
      {
        adapter: "git",
        url: "https://bitbucket.org/user/skill",
        tap: "community",
      },
      verifyNpmFail,
      verifyGitHubFail,
    );
    expect(trust.tier).toBe("curated");
    expect(trust.tap).toBe("community");
  });

  test("curated tap populates tap string even with higher tier", async () => {
    // GitHub URL from a tap — publisher tier but tap is also present
    const trust = await resolveTrust(
      {
        adapter: "github",
        url: "https://github.com/acme/skill",
        tap: "home",
        githubRepo: "acme/skill",
      },
      verifyNpmFail,
      verifyGitHubFail,
    );
    expect(trust.tier).toBe("publisher");
    expect(trust.tap).toBe("home");
  });

  test("tap skill with trust.verified provides curated", async () => {
    const tapSkill: TapSkill = {
      name: "cool-skill",
      description: "A cool skill",
      repo: "https://bitbucket.org/user/skill",
      tags: [],
      trust: { verified: true, verifiedBy: "maintainer", verifiedAt: "2026-01-01" },
    };
    const trust = await resolveTrust(
      {
        adapter: "git",
        url: "https://bitbucket.org/user/skill",
        tap: "home",
        tapSkill,
      },
      verifyNpmFail,
      verifyGitHubFail,
    );
    // Tap curated — non-GitHub URL so no publisher
    expect(trust.tier).toBe("curated");
    expect(trust.tap).toBe("home");
  });
});

describe("resolveTrust — local source", () => {
  test("unverified for local paths", async () => {
    const trust = await resolveTrust(
      {
        adapter: "local",
        url: "/home/user/my-skill",
        tap: null,
      },
      verifyNpmFail,
      verifyGitHubFail,
    );
    expect(trust.tier).toBe("unverified");
    expect(trust.publisher).toBeUndefined();
  });

  test("curated for local path from tap", async () => {
    const trust = await resolveTrust(
      {
        adapter: "local",
        url: "/home/user/my-skill",
        tap: "home",
      },
      verifyNpmFail,
      verifyGitHubFail,
    );
    expect(trust.tier).toBe("curated");
    expect(trust.tap).toBe("home");
  });
});

describe("resolveTrust — parseGitHubRepo", () => {
  test("parses https github URL", async () => {
    const { parseGitHubRepo } = await import("./verify-github");
    expect(parseGitHubRepo("https://github.com/owner/repo")).toBe("owner/repo");
    expect(parseGitHubRepo("https://github.com/owner/repo.git")).toBe("owner/repo");
    expect(parseGitHubRepo("https://github.com/owner/repo/tree/main")).toBe("owner/repo");
  });

  test("parses github: shorthand", async () => {
    const { parseGitHubRepo } = await import("./verify-github");
    expect(parseGitHubRepo("github:owner/repo")).toBe("owner/repo");
  });

  test("parses git@github.com SSH URL", async () => {
    const { parseGitHubRepo } = await import("./verify-github");
    expect(parseGitHubRepo("git@github.com:owner/repo.git")).toBe("owner/repo");
  });

  test("returns null for non-github URLs", async () => {
    const { parseGitHubRepo } = await import("./verify-github");
    expect(parseGitHubRepo("https://gitlab.com/owner/repo")).toBeNull();
    expect(parseGitHubRepo("https://bitbucket.org/owner/repo")).toBeNull();
    expect(parseGitHubRepo("/local/path")).toBeNull();
  });
});
