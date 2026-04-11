import {
  afterEach,
  beforeEach,
  describe,
  expect,
  setDefaultTimeout,
  test,
} from "bun:test";
import { lstat } from "node:fs/promises";
import { dirname, join } from "node:path";
import { createTestEnv, makeTmpDir, removeTmpDir, type TestEnv } from "@skilltap/test-utils";
import { $ } from "bun";
import { loadInstalled } from "./config";
import { disableSkill } from "./disable";
import { installSkill } from "./install";
import { removeSkill } from "./remove";
import { updateSkill } from "./update";

setDefaultTimeout(60_000);

// --- Mock registry helpers (copied from install.npm.test.ts) ---

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

// --- Env ---

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

// ---------------------------------------------------------------------------
// Journey 7: npm standalone — install → version update → disable → remove
// ---------------------------------------------------------------------------
describe("npm standalone lifecycle", () => {
  test("install → update to new version → disable → remove", async () => {
    const tgzDir1 = await makeTmpDir();
    const tgzDir2 = await makeTmpDir();
    try {
      const v1 = await buildTarball(tgzDir1, {
        "SKILL.md":
          "---\nname: npm-skill\ndescription: Version one\n---\n# npm-skill\nOld.",
      });
      const v2 = await buildTarball(tgzDir2, {
        "SKILL.md":
          "---\nname: npm-skill\ndescription: Version two\n---\n# npm-skill\nNew.",
      });

      const packages: MockPackageEntry[] = [
        {
          name: "npm-skill",
          versions: [
            { version: "1.0.0", tgzPath: v1.tgzPath, integrity: v1.integrity },
          ],
        },
      ];
      const registry = startMockRegistry(packages);
      process.env.NPM_CONFIG_REGISTRY = registry.baseUrl;

      try {
        // --- Install v1 ---
        const install = await installSkill("npm:npm-skill", {
          scope: "global",
          skipScan: true,
        });
        expect(install.ok).toBe(true);
        if (!install.ok) return;

        expect(install.value.records[0]!.repo).toBe("npm:npm-skill");
        expect(install.value.records[0]!.ref).toBe("1.0.0");
        expect(install.value.records[0]!.sha).toBeNull();

        // --- Publish v2, then update ---
        packages[0]!.versions.push({
          version: "2.0.0",
          tgzPath: v2.tgzPath,
          integrity: v2.integrity,
        });

        const up = await updateSkill(
          { yes: true },
          async () => ({ tier: "unverified" as const }),
        );
        expect(up.ok).toBe(true);
        if (!up.ok) return;
        expect(up.value.updated).toContain("npm-skill");

        // Verify new content on disk
        const content = await Bun.file(
          join(homeDir, ".agents", "skills", "npm-skill", "SKILL.md"),
        ).text();
        expect(content).toContain("Version two");

        // Verify ref updated in record
        const loaded1 = await loadInstalled();
        expect(loaded1.ok).toBe(true);
        if (!loaded1.ok) return;
        expect(loaded1.value.skills[0]!.ref).toBe("2.0.0");

        // --- Disable ---
        const dis = await disableSkill("npm-skill");
        expect(dis.ok).toBe(true);

        const disabledDir = join(
          homeDir,
          ".agents",
          "skills",
          ".disabled",
          "npm-skill",
        );
        expect((await lstat(disabledDir)).isDirectory()).toBe(true);

        // --- Remove ---
        const rm = await removeSkill("npm-skill");
        expect(rm.ok).toBe(true);

        const loaded2 = await loadInstalled();
        expect(loaded2.ok).toBe(true);
        if (!loaded2.ok) return;
        expect(loaded2.value.skills).toHaveLength(0);
      } finally {
        registry.stop();
      }
    } finally {
      await removeTmpDir(tgzDir1);
      await removeTmpDir(tgzDir2);
    }
  });
});

// ---------------------------------------------------------------------------
// Journey 8: npm multi-skill — install → update → selective remove
// ---------------------------------------------------------------------------
describe("npm multi-skill lifecycle", () => {
  test("install all → update → remove one → remove other", async () => {
    const tgzDir1 = await makeTmpDir();
    const tgzDir2 = await makeTmpDir();
    try {
      const v1 = await buildTarball(tgzDir1, {
        "skills/alpha/SKILL.md":
          "---\nname: alpha\ndescription: Alpha v1\n---\n# Alpha\nOld.",
        "skills/beta/SKILL.md":
          "---\nname: beta\ndescription: Beta v1\n---\n# Beta\nOld.",
      });
      const v2 = await buildTarball(tgzDir2, {
        "skills/alpha/SKILL.md":
          "---\nname: alpha\ndescription: Alpha v2\n---\n# Alpha\nNew.",
        "skills/beta/SKILL.md":
          "---\nname: beta\ndescription: Beta v2\n---\n# Beta\nNew.",
      });

      const packages: MockPackageEntry[] = [
        {
          name: "multi-pkg",
          versions: [
            { version: "1.0.0", tgzPath: v1.tgzPath, integrity: v1.integrity },
          ],
        },
      ];
      const registry = startMockRegistry(packages);
      process.env.NPM_CONFIG_REGISTRY = registry.baseUrl;

      try {
        // --- Install both skills ---
        const install = await installSkill("npm:multi-pkg", {
          scope: "global",
          skipScan: true,
        });
        expect(install.ok).toBe(true);
        if (!install.ok) return;
        expect(install.value.records).toHaveLength(2);

        const names = install.value.records.map((r) => r.name).sort();
        expect(names).toEqual(["alpha", "beta"]);

        // Both should have path (multi-skill)
        for (const rec of install.value.records) {
          expect(rec.path).not.toBeNull();
          expect(rec.ref).toBe("1.0.0");
        }

        // --- Publish v2, then update ---
        packages[0]!.versions.push({
          version: "2.0.0",
          tgzPath: v2.tgzPath,
          integrity: v2.integrity,
        });

        const up = await updateSkill(
          { yes: true },
          async () => ({ tier: "unverified" as const }),
        );
        expect(up.ok).toBe(true);
        if (!up.ok) return;
        // npm replaces the whole package, so both get updated
        expect(up.value.updated.sort()).toEqual(["alpha", "beta"]);

        // Verify new content on disk
        const alphaContent = await Bun.file(
          join(homeDir, ".agents", "skills", "alpha", "SKILL.md"),
        ).text();
        expect(alphaContent).toContain("Alpha v2");

        // --- Remove alpha ---
        const rm1 = await removeSkill("alpha");
        expect(rm1.ok).toBe(true);

        const loaded1 = await loadInstalled();
        expect(loaded1.ok).toBe(true);
        if (!loaded1.ok) return;
        expect(loaded1.value.skills).toHaveLength(1);
        expect(loaded1.value.skills[0]!.name).toBe("beta");

        // --- Remove beta ---
        const rm2 = await removeSkill("beta");
        expect(rm2.ok).toBe(true);

        const loaded2 = await loadInstalled();
        expect(loaded2.ok).toBe(true);
        if (!loaded2.ok) return;
        expect(loaded2.value.skills).toHaveLength(0);
      } finally {
        registry.stop();
      }
    } finally {
      await removeTmpDir(tgzDir1);
      await removeTmpDir(tgzDir2);
    }
  });
});
