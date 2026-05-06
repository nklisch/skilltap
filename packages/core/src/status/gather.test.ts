import { describe, expect, test, beforeEach, afterEach } from "bun:test";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createTestEnv, type TestEnv } from "@skilltap/test-utils";
import { gatherStatus } from "./gather";

let env: TestEnv;
let projectRoot: string;

beforeEach(async () => {
  env = await createTestEnv();
  await mkdir(join(env.configDir, "skilltap"), { recursive: true });
  projectRoot = await mkdtemp(join(tmpdir(), "skilltap-status-"));
});

afterEach(async () => {
  await env.cleanup();
  await rm(projectRoot, { recursive: true, force: true });
});

describe("gatherStatus — empty/clean state", () => {
  test("returns empty report when nothing is installed", async () => {
    const result = await gatherStatus({ projectRootHint: null });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.skills).toEqual([]);
    expect(result.value.plugins).toEqual([]);
    expect(result.value.fromV2State).toBe(false);
    expect(result.value.drift).toBeNull();
  });

  test("includes built-in tap by default", async () => {
    const result = await gatherStatus({ projectRootHint: null });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.taps.some((t) => t.builtin)).toBe(true);
  });
});

describe("gatherStatus — v2 state.json", () => {
  test("reads skills + plugins from state.json when present", async () => {
    const cfgDir = join(env.configDir, "skilltap");
    await writeFile(
      join(cfgDir, "state.json"),
      JSON.stringify({
        version: 2,
        skills: [
          {
            name: "commit-helper",
            description: "",
            repo: "github:n/r",
            ref: "v1.0",
            sha: "abc",
            scope: "global",
            path: null,
            tap: null,
            also: ["claude-code"],
            installedAt: "2026-05-05T00:00:00.000Z",
            updatedAt: "2026-05-05T00:00:00.000Z",
            active: true,
          },
        ],
        plugins: [
          {
            name: "dev-toolkit",
            description: "",
            format: "skilltap",
            repo: "github:c/d",
            ref: "main",
            sha: "def",
            scope: "global",
            also: [],
            tap: null,
            components: [
              { type: "skill", name: "code-review", active: true },
              { type: "agent", name: "reviewer", active: true, platform: "claude-code" },
            ],
            installedAt: "2026-05-05T00:00:00.000Z",
            updatedAt: "2026-05-05T00:00:00.000Z",
            active: true,
          },
        ],
        mcpServers: [],
      }),
    );

    const result = await gatherStatus({ projectRootHint: null });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.fromV2State).toBe(true);
    expect(result.value.skills).toHaveLength(1);
    expect(result.value.skills[0].name).toBe("commit-helper");
    expect(result.value.plugins).toHaveLength(1);
    expect(result.value.plugins[0].componentSummary).toContain("1 skill");
    expect(result.value.plugins[0].componentSummary).toContain("1 agent");
  });
});

describe("gatherStatus — v1 fallback", () => {
  test("falls back to installed.json + plugins.json when state.json absent", async () => {
    const cfgDir = join(env.configDir, "skilltap");
    await writeFile(
      join(cfgDir, "installed.json"),
      JSON.stringify({
        version: 1,
        skills: [
          {
            name: "old-skill",
            repo: "github:n/r",
            ref: "v1",
            sha: "abc",
            scope: "global",
            path: null,
            tap: null,
            also: [],
            installedAt: "2026-05-05T00:00:00.000Z",
            updatedAt: "2026-05-05T00:00:00.000Z",
          },
        ],
      }),
    );
    await writeFile(
      join(cfgDir, "plugins.json"),
      JSON.stringify({ version: 1, plugins: [] }),
    );

    const result = await gatherStatus({ projectRootHint: null });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.fromV2State).toBe(false);
    expect(result.value.skills).toHaveLength(1);
    expect(result.value.skills[0].name).toBe("old-skill");
  });
});

describe("gatherStatus — drift detection", () => {
  test("returns null drift when no manifest exists", async () => {
    const result = await gatherStatus({ projectRootHint: projectRoot });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.hasManifest).toBe(false);
    expect(result.value.drift).toBeNull();
  });

  test("computes drift when manifest exists", async () => {
    // Create skilltap.toml with a declared skill
    await writeFile(
      join(projectRoot, "skilltap.toml"),
      `[skills]\n"github:n/foo" = "^1.0"\n`,
    );
    // Empty state — declared not installed → 1 add
    await mkdir(join(projectRoot, ".agents"), { recursive: true });
    await writeFile(
      join(projectRoot, ".agents", "state.json"),
      JSON.stringify({ version: 2, skills: [], plugins: [], mcpServers: [] }),
    );

    const result = await gatherStatus({ projectRootHint: projectRoot });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.hasManifest).toBe(true);
    expect(result.value.drift).not.toBeNull();
    expect(result.value.drift?.inSync).toBe(false);
    expect(result.value.drift?.items[0]).toMatchObject({
      kind: "add",
      source: "github:n/foo",
    });
  });

  test("inSync when manifest matches state", async () => {
    await writeFile(join(projectRoot, "skilltap.toml"), "");
    await mkdir(join(projectRoot, ".agents"), { recursive: true });
    await writeFile(
      join(projectRoot, ".agents", "state.json"),
      JSON.stringify({ version: 2, skills: [], plugins: [], mcpServers: [] }),
    );
    const result = await gatherStatus({ projectRootHint: projectRoot });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.drift?.inSync).toBe(true);
  });
});
