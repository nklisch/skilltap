import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createTestEnv, type TestEnv } from "@skilltap/test-utils";
import { fileExists } from "../../fs";
import type { State } from "../../state/schema";
import { checkV1Orphans } from "./v1-orphans";

let env: TestEnv;
let projectRoot: string;

const POPULATED_STATE: State = {
  version: 2,
  skills: [
    {
      name: "x",
      description: "",
      repo: "github:n/x",
      ref: "main",
      sha: "abc",
      scope: "global",
      path: null,
      tap: null,
      also: [],
      installedAt: "2026-05-06T00:00:00.000Z",
      updatedAt: "2026-05-06T00:00:00.000Z",
      active: true,
    },
  ],
  plugins: [],
  mcpServers: [],
};

const EMPTY_STATE: State = {
  version: 2,
  skills: [],
  plugins: [],
  mcpServers: [],
};

beforeEach(async () => {
  env = await createTestEnv();
  await mkdir(join(env.configDir, "skilltap"), { recursive: true });
  projectRoot = await mkdtemp(join(tmpdir(), "skilltap-orphan-"));
  await mkdir(join(projectRoot, ".agents"), { recursive: true });
});

afterEach(async () => {
  await env.cleanup();
  await rm(projectRoot, { recursive: true, force: true });
});

describe("checkV1Orphans", () => {
  test("passes (n/a) when state is null", async () => {
    const result = await checkV1Orphans(null, projectRoot);
    expect(result.status).toBe("pass");
    expect(result.detail).toContain("n/a");
  });

  test("passes (n/a) when state is empty (unmigrated user)", async () => {
    // Even if installed.json exists, an empty state means the fallback is
    // still actively reading it. Don't flag it.
    await writeFile(
      join(env.configDir, "skilltap", "installed.json"),
      JSON.stringify({ version: 1, skills: [] }),
    );
    const result = await checkV1Orphans(EMPTY_STATE, projectRoot);
    expect(result.status).toBe("pass");
    expect(result.detail).toContain("n/a");
  });

  test("passes when state is populated AND no v0.x files exist", async () => {
    const result = await checkV1Orphans(POPULATED_STATE, projectRoot);
    expect(result.status).toBe("pass");
    expect(result.detail).toContain("no orphaned");
  });

  test("warns when state is populated AND global installed.json is on disk", async () => {
    await writeFile(
      join(env.configDir, "skilltap", "installed.json"),
      JSON.stringify({ version: 1, skills: [] }),
    );
    const result = await checkV1Orphans(POPULATED_STATE, projectRoot);
    expect(result.status).toBe("warn");
    expect(result.issues).toHaveLength(1);
    expect(result.issues?.[0]?.message).toContain("global installed.json");
    expect(result.issues?.[0]?.fixable).toBe(true);
  });

  test("warns for both global plugins.json and project installed.json", async () => {
    await writeFile(
      join(env.configDir, "skilltap", "plugins.json"),
      JSON.stringify({ version: 1, plugins: [] }),
    );
    await writeFile(
      join(projectRoot, ".agents", "installed.json"),
      JSON.stringify({ version: 1, skills: [] }),
    );
    const result = await checkV1Orphans(POPULATED_STATE, projectRoot);
    expect(result.status).toBe("warn");
    expect(result.issues).toHaveLength(2);
  });

  test("--fix renames orphan to .v1.bak", async () => {
    const orphanPath = join(env.configDir, "skilltap", "installed.json");
    await writeFile(orphanPath, JSON.stringify({ version: 1, skills: [] }));

    const result = await checkV1Orphans(POPULATED_STATE, projectRoot);
    expect(result.issues).toHaveLength(1);
    await result.issues?.[0]?.fix?.();

    expect(await fileExists(orphanPath)).toBe(false);
    expect(await fileExists(`${orphanPath}.v1.bak`)).toBe(true);
  });
});
