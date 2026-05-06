import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createTestEnv, type TestEnv } from "@skilltap/test-utils";
import { saveState } from "./save";
import { loadActiveInstalled, loadActivePlugins } from "./read-bridge";

let env: TestEnv;
let projectRoot: string;

const SKILL = (name: string) => ({
  name,
  description: "",
  repo: `github:n/${name}`,
  ref: "main",
  sha: "abc",
  scope: "project" as const,
  path: null,
  tap: null,
  also: ["claude-code"],
  installedAt: "2026-05-06T00:00:00.000Z",
  updatedAt: "2026-05-06T00:00:00.000Z",
  active: true,
});

const PLUGIN = (name: string) => ({
  name,
  description: "",
  format: "skilltap" as const,
  repo: `github:c/${name}`,
  ref: "main",
  sha: "def",
  scope: "project" as const,
  also: [],
  tap: null,
  components: [],
  installedAt: "2026-05-06T00:00:00.000Z",
  updatedAt: "2026-05-06T00:00:00.000Z",
  active: true,
});

beforeEach(async () => {
  env = await createTestEnv();
  await mkdir(join(env.configDir, "skilltap"), { recursive: true });
  projectRoot = await mkdtemp(join(tmpdir(), "skilltap-readbridge-"));
});

afterEach(async () => {
  await env.cleanup();
  await rm(projectRoot, { recursive: true, force: true });
});

describe("loadActiveInstalled", () => {
  test("returns state.json skills when state has entries", async () => {
    await saveState(
      { version: 2, skills: [SKILL("from-state")], plugins: [], mcpServers: [] },
      projectRoot,
    );
    // Stale installed.json should NOT be read in this case.
    await mkdir(join(projectRoot, ".agents"), { recursive: true });
    await writeFile(
      join(projectRoot, ".agents", "installed.json"),
      JSON.stringify({ version: 1, skills: [SKILL("stale")] }),
    );

    const result = await loadActiveInstalled("project", projectRoot);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.skills.map((s) => s.name)).toEqual(["from-state"]);
  });

  test("falls back to installed.json when state.json is empty (unmigrated v0.x user)", async () => {
    await mkdir(join(projectRoot, ".agents"), { recursive: true });
    await writeFile(
      join(projectRoot, ".agents", "installed.json"),
      JSON.stringify({ version: 1, skills: [SKILL("legacy")] }),
    );

    const result = await loadActiveInstalled("project", projectRoot);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.skills.map((s) => s.name)).toEqual(["legacy"]);
  });

  test("returns empty when neither file exists", async () => {
    const result = await loadActiveInstalled("project", projectRoot);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.skills).toEqual([]);
  });
});

describe("loadActivePlugins", () => {
  test("returns state.json plugins when state has entries", async () => {
    await saveState(
      { version: 2, skills: [], plugins: [PLUGIN("from-state")], mcpServers: [] },
      projectRoot,
    );

    const result = await loadActivePlugins("project", projectRoot);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.plugins.map((p) => p.name)).toEqual(["from-state"]);
  });

  test("falls back to plugins.json when state.json is empty", async () => {
    await mkdir(join(projectRoot, ".agents"), { recursive: true });
    await writeFile(
      join(projectRoot, ".agents", "plugins.json"),
      JSON.stringify({ version: 1, plugins: [PLUGIN("legacy")] }),
    );

    const result = await loadActivePlugins("project", projectRoot);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.plugins.map((p) => p.name)).toEqual(["legacy"]);
  });
});
