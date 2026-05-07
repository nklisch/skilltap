/**
 * CLI subprocess tests for `skilltap migrate`.
 *
 * Spec: docs/SPEC.md §v2.0 Migrate Command (lines 3226-3244).
 *
 * Behaviors covered here that aren't in the core unit tests:
 *   - HTTP-tap abort surfaces the offending tap list at the CLI (exit 1).
 *   - Idempotency: re-running migrate after success returns "already on v2.0".
 *   - --json output shape on success.
 *   - "Already on v2.0" message when no v1 markers exist.
 *
 * Core-level migration logic (config translation, state merge, .v1.bak
 * renames) is exercised in packages/core/src/migrate/run.test.ts.
 */

import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import {
  makeTmpDir,
  removeTmpDir,
  runSkilltap,
} from "@skilltap/test-utils";

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

describe("migrate — already on v2.0", () => {
  test("prints '✓ Already on v2.0' and exits 0 when no v1 markers exist", async () => {
    // Spec L3232: when no v1 state present, migrate is a no-op success.
    // migrate.ts L53-58 emits this exact message.
    const env = await freshEnv();
    try {
      const { exitCode, stdout } = await runSkilltap(
        ["migrate"],
        env.homeDir,
        env.configDir,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("Already on v2.0");
    } finally {
      await env.cleanup();
    }
  });
});

describe("migrate — HTTP tap abort (Phase 31b)", () => {
  test("exits 1 with offending tap names listed when v1 config has HTTP taps", async () => {
    // Spec L3243: "If a v1.0 tap.json references HTTP taps, aborts before
    // any writes with `Migration aborted: HTTP taps are not supported in
    // v2.0...`" — surfaces the offending tap list.
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
      const stateExists = await Bun.file(
        join(cfgDir, "state.json"),
      ).exists();
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
      const first = await runSkilltap(
        ["migrate"],
        env.homeDir,
        env.configDir,
      );
      expect(first.exitCode).toBe(0);
      expect(first.stdout).not.toContain("Already on v2.0");

      // Verify .v1.bak rename happened.
      const bakExists = await Bun.file(
        join(cfgDir, "installed.json.v1.bak"),
      ).exists();
      expect(bakExists).toBe(true);
      const stateExists = await Bun.file(
        join(cfgDir, "state.json"),
      ).exists();
      expect(stateExists).toBe(true);

      // Second run — must be the alreadyMigrated path.
      const second = await runSkilltap(
        ["migrate"],
        env.homeDir,
        env.configDir,
      );
      expect(second.exitCode).toBe(0);
      expect(second.stdout).toContain("Already on v2.0");
    } finally {
      await env.cleanup();
    }
  });
});
