import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { lstat } from "node:fs/promises";
import { dirname, join } from "node:path";
import { createTestEnv, makeTmpDir, removeTmpDir, type TestEnv } from "@skilltap/test-utils";
import { $ } from "bun";
import { loadInstalled } from "./config";
import { installSkill } from "./install";
import { updateSkill } from "./update";

// --- Helpers ---

type FileMap = Record<string, string>;

type MockVersion = {
  version: string;
  tgzPath: string;
  integrity: string;
};

type MockPackageEntry = {
  name: string;
  description?: string;
  versions: MockVersion[];
};

/**
 * Build an npm-style tarball where all files live under a `package/` subdirectory.
 * Returns the path to the .tgz and its sha512 SRI integrity string.
 */
async function buildTarball(
  dir: string,
  files: FileMap,
): Promise<{ tgzPath: string; integrity: string }> {
  const packageDir = join(dir, "package");
  await $`mkdir -p ${packageDir}`.quiet();

  for (const [relPath, content] of Object.entries(files)) {
    const dest = join(packageDir, relPath);
    await $`mkdir -p ${dirname(dest)}`.quiet();
    await Bun.write(dest, content);
  }

  const tgzPath = join(dir, "pkg.tgz");
  await $`tar -czf ${tgzPath} -C ${dir} package`.quiet();

  const bytes = await Bun.file(tgzPath).arrayBuffer();
  const hasher = new Bun.CryptoHasher("sha512");
  hasher.update(new Uint8Array(bytes));
  const integrity = `sha512-${hasher.digest("base64")}`;

  return { tgzPath, integrity };
}

/**
 * Start a minimal npm registry mock server.
 * Handles GET /<pkgname> (metadata) and GET /dl/<pkgname>/<version>.tgz (tarball).
 * The `packages` array is read by reference so callers can mutate it to add versions.
 */
function startMockRegistry(packages: MockPackageEntry[]): {
  baseUrl: string;
  stop: () => void;
} {
  let baseUrl = "";

  const server = Bun.serve({
    port: 0,
    fetch(req) {
      const url = new URL(req.url);
      const path = url.pathname;

      // GET /dl/<name>/<version>.tgz → serve tarball bytes
      if (path.startsWith("/dl/")) {
        const rest = path.slice("/dl/".length);
        const lastSlash = rest.lastIndexOf("/");
        if (lastSlash === -1) return new Response("Not found", { status: 404 });
        const pkgName = rest.slice(0, lastSlash);
        const filename = rest.slice(lastSlash + 1);
        const version = filename.endsWith(".tgz") ? filename.slice(0, -4) : filename;

        const pkg = packages.find((p) => p.name === pkgName);
        const ver = pkg?.versions.find((v) => v.version === version);
        if (!ver) return new Response("Not found", { status: 404 });

        return new Response(Bun.file(ver.tgzPath));
      }

      // GET /<name> → package metadata JSON
      const pkgName = decodeURIComponent(path.slice(1));
      const pkg = packages.find((p) => p.name === pkgName);
      if (!pkg) return new Response("Not found", { status: 404 });

      const latestVersion = pkg.versions.at(-1)?.version ?? "";
      const versions: Record<string, unknown> = {};
      for (const ver of pkg.versions) {
        versions[ver.version] = {
          version: ver.version,
          dist: {
            tarball: `${baseUrl}dl/${pkg.name}/${ver.version}.tgz`,
            integrity: ver.integrity,
          },
        };
      }

      return new Response(
        JSON.stringify({
          name: pkg.name,
          description: pkg.description ?? "",
          "dist-tags": { latest: latestVersion },
          versions,
        }),
        { headers: { "Content-Type": "application/json" } },
      );
    },
  });

  baseUrl = `http://127.0.0.1:${server.port}/`;
  return { baseUrl, stop: () => server.stop(true) };
}

// --- Env setup ---

let env: TestEnv;
let homeDir: string;
let configDir: string;
let savedNpmRegistry: string | undefined;

beforeEach(async () => {
  env = await createTestEnv();
  homeDir = env.homeDir;
  configDir = env.configDir;
  savedNpmRegistry = process.env.NPM_CONFIG_REGISTRY;
});

afterEach(async () => {
  if (savedNpmRegistry === undefined) delete process.env.NPM_CONFIG_REGISTRY;
  else process.env.NPM_CONFIG_REGISTRY = savedNpmRegistry;
  await env.cleanup();
});

// --- Tests ---

describe("installSkill — npm standalone", () => {
  test("installs to global scope and places skill directory", async () => {
    const tgzDir = await makeTmpDir();
    try {
      const { tgzPath, integrity } = await buildTarball(tgzDir, {
        "SKILL.md":
          "---\nname: npm-skill\ndescription: A skill from npm\n---\n# npm-skill\nInstructions.",
      });
      const packages: MockPackageEntry[] = [
        { name: "npm-skill", versions: [{ version: "1.0.0", tgzPath, integrity }] },
      ];
      const registry = startMockRegistry(packages);
      process.env.NPM_CONFIG_REGISTRY = registry.baseUrl;

      try {
        const result = await installSkill("npm:npm-skill", {
          scope: "global",
          skipScan: true,
        });
        expect(result.ok).toBe(true);
        if (!result.ok) return;

        expect(result.value.records).toHaveLength(1);
        // biome-ignore lint/style/noNonNullAssertion: checked length above
        const record = result.value.records[0]!;
        expect(record.name).toBe("npm-skill");
        expect(record.repo).toBe("npm:npm-skill");
        expect(record.ref).toBe("1.0.0");
        expect(record.sha).toBeNull();
        expect(record.path).toBeNull();
        expect(record.scope).toBe("global");

        const skillDir = join(homeDir, ".agents", "skills", "npm-skill");
        expect(await lstat(skillDir).then((s) => s.isDirectory())).toBe(true);
        expect(await Bun.file(join(skillDir, "SKILL.md")).exists()).toBe(true);
      } finally {
        registry.stop();
      }
    } finally {
      await removeTmpDir(tgzDir);
    }
  });

  test("stores correct repo, ref, sha, and path in installed.json", async () => {
    const tgzDir = await makeTmpDir();
    try {
      const { tgzPath, integrity } = await buildTarball(tgzDir, {
        "SKILL.md":
          "---\nname: npm-skill\ndescription: A skill from npm\n---\n# npm-skill\nInstructions.",
      });
      const registry = startMockRegistry([
        { name: "npm-skill", versions: [{ version: "2.3.1", tgzPath, integrity }] },
      ]);
      process.env.NPM_CONFIG_REGISTRY = registry.baseUrl;

      try {
        await installSkill("npm:npm-skill", { scope: "global", skipScan: true });

        const installedResult = await loadInstalled();
        expect(installedResult.ok).toBe(true);
        if (!installedResult.ok) return;

        const { skills } = installedResult.value;
        expect(skills).toHaveLength(1);
        // biome-ignore lint/style/noNonNullAssertion: checked length above
        const record = skills[0]!;
        expect(record.repo).toBe("npm:npm-skill");
        expect(record.ref).toBe("2.3.1");
        expect(record.sha).toBeNull();
        expect(record.path).toBeNull();
      } finally {
        registry.stop();
      }
    } finally {
      await removeTmpDir(tgzDir);
    }
  });

  test("installs a specific version with npm:pkg@version syntax", async () => {
    const tgzDir1 = await makeTmpDir();
    const tgzDir2 = await makeTmpDir();
    try {
      const v1 = await buildTarball(tgzDir1, {
        "SKILL.md": "---\nname: npm-skill\ndescription: Version one\n---\n# v1",
      });
      const v2 = await buildTarball(tgzDir2, {
        "SKILL.md": "---\nname: npm-skill\ndescription: Version two\n---\n# v2",
      });
      const registry = startMockRegistry([
        {
          name: "npm-skill",
          versions: [
            { version: "1.0.0", tgzPath: v1.tgzPath, integrity: v1.integrity },
            { version: "2.0.0", tgzPath: v2.tgzPath, integrity: v2.integrity },
          ],
        },
      ]);
      process.env.NPM_CONFIG_REGISTRY = registry.baseUrl;

      try {
        const result = await installSkill("npm:npm-skill@1.0.0", {
          scope: "global",
          skipScan: true,
        });
        expect(result.ok).toBe(true);
        if (!result.ok) return;

        // biome-ignore lint/style/noNonNullAssertion: install succeeded
        expect(result.value.records[0]!.ref).toBe("1.0.0");
      } finally {
        registry.stop();
      }
    } finally {
      await removeTmpDir(tgzDir1);
      await removeTmpDir(tgzDir2);
    }
  });

  test("errors when package is not found", async () => {
    const registry = startMockRegistry([]);
    process.env.NPM_CONFIG_REGISTRY = registry.baseUrl;

    try {
      const result = await installSkill("npm:does-not-exist", {
        scope: "global",
        skipScan: true,
      });
      expect(result.ok).toBe(false);
    } finally {
      registry.stop();
    }
  });
});

describe("installSkill — npm multi-skill (skills/* convention)", () => {
  test("installs all skills from a package with skills/ layout", async () => {
    const tgzDir = await makeTmpDir();
    try {
      const { tgzPath, integrity } = await buildTarball(tgzDir, {
        "skills/skill-alpha/SKILL.md":
          "---\nname: skill-alpha\ndescription: Alpha skill\n---\n# skill-alpha",
        "skills/skill-beta/SKILL.md":
          "---\nname: skill-beta\ndescription: Beta skill\n---\n# skill-beta",
      });
      const registry = startMockRegistry([
        { name: "multi-skill-pkg", versions: [{ version: "1.0.0", tgzPath, integrity }] },
      ]);
      process.env.NPM_CONFIG_REGISTRY = registry.baseUrl;

      try {
        const result = await installSkill("npm:multi-skill-pkg", {
          scope: "global",
          skipScan: true,
        });
        expect(result.ok).toBe(true);
        if (!result.ok) return;

        expect(result.value.records).toHaveLength(2);
        const names = result.value.records.map((r) => r.name).sort();
        expect(names).toEqual(["skill-alpha", "skill-beta"]);

        for (const record of result.value.records) {
          expect(record.repo).toBe("npm:multi-skill-pkg");
          expect(record.ref).toBe("1.0.0");
          expect(record.sha).toBeNull();
          expect(record.path).not.toBeNull();

          const skillDir = join(homeDir, ".agents", "skills", record.name);
          expect(await lstat(skillDir).then((s) => s.isDirectory())).toBe(true);
          expect(
            await Bun.file(join(skillDir, "SKILL.md")).exists(),
          ).toBe(true);
        }
      } finally {
        registry.stop();
      }
    } finally {
      await removeTmpDir(tgzDir);
    }
  });

  test("installs only the selected skill when skillNames is specified", async () => {
    const tgzDir = await makeTmpDir();
    try {
      const { tgzPath, integrity } = await buildTarball(tgzDir, {
        "skills/skill-alpha/SKILL.md":
          "---\nname: skill-alpha\ndescription: Alpha skill\n---\n# skill-alpha",
        "skills/skill-beta/SKILL.md":
          "---\nname: skill-beta\ndescription: Beta skill\n---\n# skill-beta",
      });
      const registry = startMockRegistry([
        { name: "multi-skill-pkg", versions: [{ version: "1.0.0", tgzPath, integrity }] },
      ]);
      process.env.NPM_CONFIG_REGISTRY = registry.baseUrl;

      try {
        const result = await installSkill("npm:multi-skill-pkg", {
          scope: "global",
          skillNames: ["skill-alpha"],
          skipScan: true,
        });
        expect(result.ok).toBe(true);
        if (!result.ok) return;

        expect(result.value.records).toHaveLength(1);
        // biome-ignore lint/style/noNonNullAssertion: checked length above
        expect(result.value.records[0]!.name).toBe("skill-alpha");

        const alphaDir = join(homeDir, ".agents", "skills", "skill-alpha");
        const betaDir = join(homeDir, ".agents", "skills", "skill-beta");
        expect(await lstat(alphaDir).then((s) => s.isDirectory())).toBe(true);
        expect(await lstat(betaDir).catch(() => null)).toBeNull();
      } finally {
        registry.stop();
      }
    } finally {
      await removeTmpDir(tgzDir);
    }
  });
});

describe("updateSkill — npm", () => {
  test("reports upToDate when already on the latest version", async () => {
    const tgzDir = await makeTmpDir();
    try {
      const { tgzPath, integrity } = await buildTarball(tgzDir, {
        "SKILL.md":
          "---\nname: npm-skill\ndescription: A skill\n---\n# npm-skill\nInstructions.",
      });
      const registry = startMockRegistry([
        { name: "npm-skill", versions: [{ version: "1.0.0", tgzPath, integrity }] },
      ]);
      process.env.NPM_CONFIG_REGISTRY = registry.baseUrl;

      try {
        await installSkill("npm:npm-skill", { scope: "global", skipScan: true });

        const result = await updateSkill({ yes: true });
        expect(result.ok).toBe(true);
        if (!result.ok) return;

        expect(result.value.upToDate).toContain("npm-skill");
        expect(result.value.updated).toHaveLength(0);
      } finally {
        registry.stop();
      }
    } finally {
      await removeTmpDir(tgzDir);
    }
  });

  test("updates to the newer version when one becomes available", async () => {
    const tgzDir1 = await makeTmpDir();
    const tgzDir2 = await makeTmpDir();
    try {
      const v1 = await buildTarball(tgzDir1, {
        "SKILL.md":
          "---\nname: npm-skill\ndescription: Version one\n---\n# npm-skill\nOld instructions.",
      });
      const v2 = await buildTarball(tgzDir2, {
        "SKILL.md":
          "---\nname: npm-skill\ndescription: Version two\n---\n# npm-skill\nNew instructions.",
      });

      // Start with only v1 in the registry
      const packages: MockPackageEntry[] = [
        {
          name: "npm-skill",
          versions: [{ version: "1.0.0", tgzPath: v1.tgzPath, integrity: v1.integrity }],
        },
      ];
      const registry = startMockRegistry(packages);
      process.env.NPM_CONFIG_REGISTRY = registry.baseUrl;

      try {
        await installSkill("npm:npm-skill", { scope: "global", skipScan: true });

        // Simulate v2 being published by mutating the packages array
        // biome-ignore lint/style/noNonNullAssertion: packages[0] initialized above
        packages[0]!.versions.push({
          version: "2.0.0",
          tgzPath: v2.tgzPath,
          integrity: v2.integrity,
        });

        const result = await updateSkill(
          { yes: true },
          async () => ({ tier: "unverified" as const }),
        );
        expect(result.ok).toBe(true);
        if (!result.ok) return;

        expect(result.value.updated).toContain("npm-skill");
        expect(result.value.upToDate).toHaveLength(0);

        // New content should be on disk
        const skillDir = join(homeDir, ".agents", "skills", "npm-skill");
        const content = await Bun.file(join(skillDir, "SKILL.md")).text();
        expect(content).toContain("Version two");

        // Installed record should reflect the new version
        const installedResult = await loadInstalled();
        expect(installedResult.ok).toBe(true);
        if (!installedResult.ok) return;
        // biome-ignore lint/style/noNonNullAssertion: skill was installed
        expect(installedResult.value.skills[0]!.ref).toBe("2.0.0");
      } finally {
        registry.stop();
      }
    } finally {
      await removeTmpDir(tgzDir1);
      await removeTmpDir(tgzDir2);
    }
  }, 30_000);
});
