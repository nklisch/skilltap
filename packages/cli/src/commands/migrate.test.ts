/**
 * CLI subprocess tests for `skilltap migrate`.
 *
 * Behaviors covered here that aren't in the core unit tests:
 *   - HTTP-tap abort surfaces the offending tap list at the CLI (exit 1).
 *   - Idempotency: re-running migrate after success returns "Already migrated".
 *   - --json output shape on success.
 *   - "Already migrated" message when no legacy markers exist.
 *
 * Core-level migration logic (config translation, state merge, .v1.bak
 * renames) is exercised in packages/core/src/migrate/run.test.ts.
 */

import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { makeTmpDir, removeTmpDir, runSkilltap } from "@skilltap/test-utils";

let homeDir: string;
let configDir: string;

beforeAll(async () => {
  homeDir = await makeTmpDir();
  configDir = await makeTmpDir();
});

afterAll(async () => {
  await removeTmpDir(homeDir);
  await removeTmpDir(configDir);
});

// Each test uses its own isolated home/config so migrate runs don't leak
// between tests. Helper makes a fresh pair.
async function freshEnv(): Promise<{
  homeDir: string;
  configDir: string;
  cleanup: () => Promise<void>;
}> {
  const home = await makeTmpDir();
  const config = await makeTmpDir();
  return {
    homeDir: home,
    configDir: config,
    cleanup: async () => {
      await removeTmpDir(home);
      await removeTmpDir(config);
    },
  };
}

describe("migrate — already migrated", () => {
  test("prints '✓ Already migrated' and exits 0 when no legacy markers exist", async () => {
    const env = await freshEnv();
    try {
      const { exitCode, stdout } = await runSkilltap(
        ["migrate"],
        env.homeDir,
        env.configDir,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("Already migrated");
    } finally {
      await env.cleanup();
    }
  });
});

describe("migrate — HTTP tap abort", () => {
  test("exits 1 with offending tap names listed when legacy config has HTTP taps", async () => {
    // If a legacy tap config references HTTP taps, migrate aborts before any
    // writes and surfaces the offending tap list.
    const env = await freshEnv();
    try {
      // Plant a v1 config with HTTP tap entries, plus an installed.json
      // marker so the v1-detection trigger fires.
      const cfgDir = join(env.configDir, "skilltap");
      await mkdir(cfgDir, { recursive: true });
      await writeFile(
        join(cfgDir, "config.toml"),
        `
[security]
scan = "static"
on_warn = "prompt"

[[taps]]
name = "legacy-http-tap"
url = "https://example.com/api/registry"
type = "http"
`,
      );
      // Plant an installed.json marker so v1 detection triggers.
      await writeFile(
        join(cfgDir, "installed.json"),
        JSON.stringify({ version: 1, skills: [] }, null, 2),
      );

      const { exitCode, stderr } = await runSkilltap(
        ["migrate"],
        env.homeDir,
        env.configDir,
      );
      expect(exitCode).toBe(1);
      expect(stderr).toContain("Migration aborted");
      expect(stderr).toContain("HTTP taps");
      // The offending tap name must appear so the user knows what to fix.
      expect(stderr).toContain("legacy-http-tap");

      // Verify abort happened BEFORE any writes — state.json must not exist.
      const stateExists = await Bun.file(join(cfgDir, "state.json")).exists();
      expect(stateExists).toBe(false);
    } finally {
      await env.cleanup();
    }
  });
});

describe("migrate --json", () => {
  test("emits a parseable object with ok + alreadyMigrated when no markers", async () => {
    // Spec L3223 (cross-cutting --json convention) + migrate.ts L43-50.
    const env = await freshEnv();
    try {
      const { exitCode, stdout } = await runSkilltap(
        ["migrate", "--json"],
        env.homeDir,
        env.configDir,
      );
      expect(exitCode).toBe(0);
      const payload = JSON.parse(stdout) as {
        ok: boolean;
        alreadyMigrated: boolean;
        scopes: string[];
        warnings: string[];
      };
      expect(payload.ok).toBe(true);
      expect(payload.alreadyMigrated).toBe(true);
      expect(Array.isArray(payload.scopes)).toBe(true);
    } finally {
      await env.cleanup();
    }
  });

  test("--json on HTTP-tap abort emits ok=false with error + hint", async () => {
    const env = await freshEnv();
    try {
      const cfgDir = join(env.configDir, "skilltap");
      await mkdir(cfgDir, { recursive: true });
      await writeFile(
        join(cfgDir, "config.toml"),
        `[[taps]]
name = "x"
url = "https://example.com/y"
type = "http"
`,
      );
      await writeFile(
        join(cfgDir, "installed.json"),
        JSON.stringify({ version: 1, skills: [] }, null, 2),
      );

      const { exitCode, stdout } = await runSkilltap(
        ["migrate", "--json"],
        env.homeDir,
        env.configDir,
      );
      expect(exitCode).toBe(1);
      const payload = JSON.parse(stdout) as {
        ok: boolean;
        error: string;
        hint?: string;
      };
      expect(payload.ok).toBe(false);
      expect(payload.error).toContain("HTTP taps");
    } finally {
      await env.cleanup();
    }
  });
});

describe("migrate — idempotency", () => {
  test("re-running migrate after a successful migration is a no-op success", async () => {
    // Spec L3240-3244: migrate is "one-shot". After it succeeds, .v1.bak
    // renames mean the v1 markers no longer exist on disk. Re-running must
    // safely report alreadyMigrated=true.
    const env = await freshEnv();
    try {
      const cfgDir = join(env.configDir, "skilltap");
      await mkdir(cfgDir, { recursive: true });
      // Minimal v1 marker — installed.json with empty skills array. No HTTP
      // taps so the migration completes successfully.
      await writeFile(
        join(cfgDir, "installed.json"),
        JSON.stringify({ version: 1, skills: [] }, null, 2),
      );

      // First run — should migrate successfully.
      const first = await runSkilltap(["migrate"], env.homeDir, env.configDir);
      expect(first.exitCode).toBe(0);
      expect(first.stdout).not.toContain("Already migrated");

      // Verify .v1.bak rename happened.
      const bakExists = await Bun.file(
        join(cfgDir, "installed.json.v1.bak"),
      ).exists();
      expect(bakExists).toBe(true);
      const stateExists = await Bun.file(join(cfgDir, "state.json")).exists();
      expect(stateExists).toBe(true);

      // Second run — must be the alreadyMigrated path.
      const second = await runSkilltap(["migrate"], env.homeDir, env.configDir);
      expect(second.exitCode).toBe(0);
      expect(second.stdout).toContain("Already migrated");
    } finally {
      await env.cleanup();
    }
  });
});
