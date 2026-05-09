/**
 * Subprocess tests for multi-plugin install syntax (Unit 3.13 of v2.2-cleanup).
 *
 * Exercises:
 *   - `install plugin user/repo:auth` selects a single plugin from a multi-plugin repo
 *   - `install plugin user/repo:*`    installs every publishable plugin in sequence
 *   - `install plugin user/repo`      (multi-plugin, no suffix) errors with available names
 *   - `install plugin user/repo:bogus` errors with "not found"
 *
 * The fixture is a small git repo with two `.skilltap/<plugin>.toml` files
 * declaring two distinct plugins. Each plugin declares one skill so the
 * normalize step has something concrete to install.
 */

import {
  afterEach,
  beforeEach,
  describe,
  expect,
  setDefaultTimeout,
  test,
} from "bun:test";
import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import { loadPlugins } from "@skilltap/core";
import {
  commitAll,
  createTestEnv,
  initRepo,
  makeTmpDir,
  removeTmpDir,
  runSkilltap,
  type TestEnv,
} from "@skilltap/test-utils";

setDefaultTimeout(90_000);

let env: TestEnv;
let homeDir: string;
let configDir: string;

beforeEach(async () => {
  env = await createTestEnv();
  homeDir = env.homeDir;
  configDir = env.configDir;
});

afterEach(async () => {
  await env.cleanup();
});

async function disableBuiltinTap(configDir: string): Promise<void> {
  const dir = join(configDir, "skilltap");
  await mkdir(dir, { recursive: true });
  await Bun.write(join(dir, "config.toml"), "builtin_tap = false\n");
}

async function createMultiPluginRepo(): Promise<{
  path: string;
  cleanup: () => Promise<void>;
}> {
  const dir = await makeTmpDir();
  const skilltapDir = join(dir, ".skilltap");
  await mkdir(skilltapDir, { recursive: true });

  // Auth plugin with one skill at auth-skill/.
  await Bun.write(
    join(skilltapDir, "auth.toml"),
    `name = "auth"
version = "0.1.0"
description = "Auth plugin"
publish = true

[[skills]]
name = "auth-skill"
path = "auth-skill"
description = "Auth helper"
`,
  );
  await mkdir(join(dir, "auth-skill"), { recursive: true });
  await Bun.write(
    join(dir, "auth-skill", "SKILL.md"),
    `---\nname: auth-skill\ndescription: Auth helper skill\n---\n# Auth Skill\nTest content.\n`,
  );

  // Billing plugin with one skill at billing-skill/.
  await Bun.write(
    join(skilltapDir, "billing.toml"),
    `name = "billing"
version = "0.1.0"
description = "Billing plugin"
publish = true

[[skills]]
name = "billing-skill"
path = "billing-skill"
description = "Billing helper"
`,
  );
  await mkdir(join(dir, "billing-skill"), { recursive: true });
  await Bun.write(
    join(dir, "billing-skill", "SKILL.md"),
    `---\nname: billing-skill\ndescription: Billing helper skill\n---\n# Billing Skill\nTest content.\n`,
  );

  await initRepo(dir);
  await commitAll(dir);
  return { path: dir, cleanup: () => removeTmpDir(dir) };
}

async function createSinglePluginRepo(): Promise<{
  path: string;
  cleanup: () => Promise<void>;
}> {
  const dir = await makeTmpDir();
  const skilltapDir = join(dir, ".skilltap");
  await mkdir(skilltapDir, { recursive: true });
  await Bun.write(
    join(skilltapDir, "solo.toml"),
    `name = "solo"
version = "0.1.0"
description = "Solo plugin"
publish = true

[[skills]]
name = "solo-skill"
path = "solo-skill"
description = "Solo helper"
`,
  );
  await mkdir(join(dir, "solo-skill"), { recursive: true });
  await Bun.write(
    join(dir, "solo-skill", "SKILL.md"),
    `---\nname: solo-skill\ndescription: Solo helper skill\n---\n# Solo\nTest.\n`,
  );
  await initRepo(dir);
  await commitAll(dir);
  return { path: dir, cleanup: () => removeTmpDir(dir) };
}

describe("install plugin — multi-plugin source syntax", () => {
  test("user/repo:<name> selects a single plugin", async () => {
    await disableBuiltinTap(configDir);
    const repo = await createMultiPluginRepo();
    try {
      const { exitCode, stdout, stderr } = await runSkilltap(
        [
          "install",
          "plugin",
          `${repo.path}:auth`,
          "--yes",
          "--scope",
          "global",
          "--skip-scan",
        ],
        homeDir,
        configDir,
      );
      const combined = stdout + stderr;
      expect(exitCode).toBe(0);
      expect(combined).toContain("auth");
      expect(combined).not.toContain("billing");

      const plugins = await loadPlugins();
      expect(plugins.ok).toBe(true);
      if (!plugins.ok) return;
      const names = plugins.value.plugins.map((p) => p.name);
      expect(names).toContain("auth");
      expect(names).not.toContain("billing");
    } finally {
      await repo.cleanup();
    }
  });

  test("user/repo:* installs every publishable plugin in sequence", async () => {
    await disableBuiltinTap(configDir);
    const repo = await createMultiPluginRepo();
    try {
      const { exitCode, stdout, stderr } = await runSkilltap(
        [
          "install",
          "plugin",
          `${repo.path}:*`,
          "--yes",
          "--scope",
          "global",
          "--skip-scan",
        ],
        homeDir,
        configDir,
      );
      const combined = stdout + stderr;
      expect(exitCode).toBe(0);
      expect(combined).toContain("auth");
      expect(combined).toContain("billing");

      const plugins = await loadPlugins();
      expect(plugins.ok).toBe(true);
      if (!plugins.ok) return;
      const names = plugins.value.plugins.map((p) => p.name).sort();
      expect(names).toEqual(["auth", "billing"]);
    } finally {
      await repo.cleanup();
    }
  });

  test("ambiguous (no suffix, multi-plugin) errors with available names", async () => {
    await disableBuiltinTap(configDir);
    const repo = await createMultiPluginRepo();
    try {
      const { exitCode, stdout, stderr } = await runSkilltap(
        [
          "install",
          "plugin",
          repo.path,
          "--yes",
          "--scope",
          "global",
          "--skip-scan",
        ],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(1);
      const combined = stdout + stderr;
      expect(combined).toContain("auth");
      expect(combined).toContain("billing");
    } finally {
      await repo.cleanup();
    }
  });

  test("user/repo:bogus errors with 'not found'", async () => {
    await disableBuiltinTap(configDir);
    const repo = await createMultiPluginRepo();
    try {
      const { exitCode, stdout, stderr } = await runSkilltap(
        [
          "install",
          "plugin",
          `${repo.path}:bogus`,
          "--yes",
          "--scope",
          "global",
          "--skip-scan",
        ],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(1);
      const combined = stdout + stderr;
      expect(combined.toLowerCase()).toContain("bogus");
      expect(combined.toLowerCase()).toContain("not found");
    } finally {
      await repo.cleanup();
    }
  });

  test("single-plugin repo without suffix installs the only plugin", async () => {
    await disableBuiltinTap(configDir);
    const repo = await createSinglePluginRepo();
    try {
      const { exitCode } = await runSkilltap(
        [
          "install",
          "plugin",
          repo.path,
          "--yes",
          "--scope",
          "global",
          "--skip-scan",
        ],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);

      const plugins = await loadPlugins();
      expect(plugins.ok).toBe(true);
      if (!plugins.ok) return;
      expect(plugins.value.plugins.map((p) => p.name)).toContain("solo");
    } finally {
      await repo.cleanup();
    }
  });
});
