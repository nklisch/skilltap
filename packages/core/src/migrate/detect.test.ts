import { describe, expect, test, beforeEach, afterEach } from "bun:test";
import { writeFile, mkdir } from "node:fs/promises";
import { join } from "node:path";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { createTestEnv, type TestEnv } from "@skilltap/test-utils";
import {
  detectV1StateGlobal,
  detectV1StateProject,
  hasAnyV1Markers,
} from "./detect";

describe("detectV1StateGlobal", () => {
  let env: TestEnv;
  beforeEach(async () => {
    env = await createTestEnv();
    await mkdir(join(env.configDir, "skilltap"), { recursive: true });
  });
  afterEach(async () => {
    await env.cleanup();
  });

  test("returns no markers when files don't exist", async () => {
    const m = await detectV1StateGlobal();
    expect(m.installedJson).toBeNull();
    expect(m.pluginsJson).toBeNull();
    expect(m.configToml).toBeNull();
    expect(m.configHasV1Keys).toBe(false);
    expect(hasAnyV1Markers(m)).toBe(false);
  });

  test("detects installed.json", async () => {
    const path = join(env.configDir, "skilltap", "installed.json");
    await writeFile(path, "{}");
    const m = await detectV1StateGlobal();
    expect(m.installedJson).toBe(path);
    expect(hasAnyV1Markers(m)).toBe(true);
  });

  test("detects plugins.json", async () => {
    const path = join(env.configDir, "skilltap", "plugins.json");
    await writeFile(path, "{}");
    const m = await detectV1StateGlobal();
    expect(m.pluginsJson).toBe(path);
    expect(hasAnyV1Markers(m)).toBe(true);
  });

  test("flags v1 keys in config.toml ([security.human])", async () => {
    const path = join(env.configDir, "skilltap", "config.toml");
    await writeFile(path, `[security.human]\nscan = "static"\n`);
    const m = await detectV1StateGlobal();
    expect(m.configToml).toBe(path);
    expect(m.configHasV1Keys).toBe(true);
    expect(hasAnyV1Markers(m)).toBe(true);
  });

  test("flags v1 keys in config.toml ([agent-mode])", async () => {
    const path = join(env.configDir, "skilltap", "config.toml");
    await writeFile(path, `["agent-mode"]\nenabled = false\n`);
    const m = await detectV1StateGlobal();
    expect(m.configHasV1Keys).toBe(true);
  });

  test("does not flag v2-only config.toml", async () => {
    const path = join(env.configDir, "skilltap", "config.toml");
    await writeFile(
      path,
      `
[security]
scan = "static"

[agent]
default = false
`,
    );
    const m = await detectV1StateGlobal();
    expect(m.configToml).toBe(path);
    expect(m.configHasV1Keys).toBe(false);
    // configToml alone (without v1 keys) doesn't count as a v1 marker
    expect(hasAnyV1Markers(m)).toBe(false);
  });
});

describe("detectV1StateProject", () => {
  test("detects project-scope v1 files", async () => {
    const projectRoot = await mkdtemp(join(tmpdir(), "skilltap-proj-"));
    try {
      const agentsDir = join(projectRoot, ".agents");
      await mkdir(agentsDir, { recursive: true });
      await writeFile(join(agentsDir, "installed.json"), "{}");

      const m = await detectV1StateProject(projectRoot);
      expect(m.scope).toBe("project");
      expect(m.installedJson).toBe(join(agentsDir, "installed.json"));
      expect(m.pluginsJson).toBeNull();
      expect(hasAnyV1Markers(m)).toBe(true);
    } finally {
      await rm(projectRoot, { recursive: true, force: true });
    }
  });

  test("returns no markers for empty project", async () => {
    const projectRoot = await mkdtemp(join(tmpdir(), "skilltap-proj-"));
    try {
      const m = await detectV1StateProject(projectRoot);
      expect(hasAnyV1Markers(m)).toBe(false);
    } finally {
      await rm(projectRoot, { recursive: true, force: true });
    }
  });
});
