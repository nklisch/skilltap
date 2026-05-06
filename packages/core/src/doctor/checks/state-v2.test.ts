import { describe, expect, test, beforeEach, afterEach } from "bun:test";
import { writeFile, mkdir } from "node:fs/promises";
import { join } from "node:path";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { createTestEnv, pathExists, type TestEnv } from "@skilltap/test-utils";
import { checkStateV2 } from "./state-v2";

const VALID_STATE = {
  version: 2,
  skills: [
    {
      name: "commit-helper",
      repo: "https://github.com/n/r",
      ref: "v1.0",
      sha: "abc123",
      scope: "global",
      path: null,
      tap: null,
      also: [],
      installedAt: "2026-05-05T00:00:00.000Z",
      updatedAt: "2026-05-05T00:00:00.000Z",
    },
  ],
  plugins: [],
  mcpServers: [],
};

let env: TestEnv;
let projectRoot: string;

beforeEach(async () => {
  env = await createTestEnv();
  await mkdir(join(env.configDir, "skilltap"), { recursive: true });
  projectRoot = await mkdtemp(join(tmpdir(), "skilltap-doctor-state-test-"));
});

afterEach(async () => {
  await env.cleanup();
  await rm(projectRoot, { recursive: true, force: true });
});

describe("checkStateV2", () => {
  test("pass with n/a detail when no state.json exists", async () => {
    const result = await checkStateV2();
    expect(result.check.status).toBe("pass");
    expect(result.check.detail).toContain("no v2 state");
    expect(result.state).toBeNull();
  });

  test("pass with count detail when state.json is present and valid", async () => {
    const statePath = join(env.configDir, "skilltap", "state.json");
    await writeFile(statePath, JSON.stringify(VALID_STATE, null, 2));

    const result = await checkStateV2();
    expect(result.check.status).toBe("pass");
    expect(result.check.detail).toContain("1 skill");
    expect(result.check.detail).toContain("0 plugins");
    expect(result.state).not.toBeNull();
    expect(result.state?.skills).toHaveLength(1);
  });

  test("fail with fixable issue when state.json is corrupt JSON", async () => {
    const statePath = join(env.configDir, "skilltap", "state.json");
    await writeFile(statePath, "{ not valid json {{{{");

    const result = await checkStateV2();
    expect(result.check.status).toBe("fail");
    expect(result.check.issues).toHaveLength(1);
    expect(result.check.issues![0].fixable).toBe(true);
    expect(result.check.issues![0].message).toContain("corrupt");

    await result.check.issues![0].fix!();
    expect(await pathExists(`${statePath}.bak`)).toBe(true);
    const fresh = await Bun.file(statePath).json();
    expect(fresh.version).toBe(2);
    expect(fresh.skills).toEqual([]);
  });

  test("fail with fixable issue when state.json has invalid schema", async () => {
    const statePath = join(env.configDir, "skilltap", "state.json");
    await writeFile(statePath, JSON.stringify({ version: 99, skills: "wrong" }, null, 2));

    const result = await checkStateV2();
    expect(result.check.status).toBe("fail");
    expect(result.check.issues).toHaveLength(1);
    expect(result.check.issues![0].fixable).toBe(true);
    expect(result.check.issues![0].message).toContain("invalid");

    await result.check.issues![0].fix!();
    expect(await pathExists(`${statePath}.bak`)).toBe(true);
  });

  test("merges global and project state into single state object", async () => {
    const globalStatePath = join(env.configDir, "skilltap", "state.json");
    await writeFile(globalStatePath, JSON.stringify(VALID_STATE, null, 2));

    const projectAgentsDir = join(projectRoot, ".agents");
    await mkdir(projectAgentsDir, { recursive: true });
    const projectStatePath = join(projectAgentsDir, "state.json");
    const projectState = {
      version: 2,
      skills: [
        {
          name: "proj-skill",
          repo: "https://github.com/n/proj",
          ref: "main",
          sha: "def456",
          scope: "project",
          path: null,
          tap: null,
          also: [],
          installedAt: "2026-05-05T00:00:00.000Z",
          updatedAt: "2026-05-05T00:00:00.000Z",
        },
      ],
      plugins: [],
      mcpServers: [],
    };
    await writeFile(projectStatePath, JSON.stringify(projectState, null, 2));

    const result = await checkStateV2(projectRoot);
    expect(result.check.status).toBe("pass");
    expect(result.state?.skills).toHaveLength(2);
    expect(result.check.detail).toContain("2 skills");
  });
});
