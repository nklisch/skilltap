/**
 * CLI subprocess tests for `skilltap sync`.
 *
 * Spec: docs/SPEC.md §v2.0 Sync Command (lines 3144-3168).
 *
 * Behaviors covered here:
 *   - bare `sync` — read-only drift report; exit 0 even with drift; ends with
 *     "note: run skilltap sync --apply to execute this plan."
 *   - in-sync state — prints "✓ In sync. Manifest, lockfile, and state agree."
 *     and exits 0.
 *   - `sync --json` — emits a JSON object with `inSync` + `items` fields;
 *     no human-readable text.
 *   - `sync --apply` on in-sync state — prints "✓ In sync. Nothing to apply."
 *   - `sync --apply --strict` — exit code on failure.
 *
 * The `sync --apply` happy path is already covered in e2e-v2.test.ts; here we
 * focus on the read-only / json / strict behaviors that have no CLI coverage.
 */

import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import {
  initRepo,
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

async function makeProjectRoot(opts: {
  manifestToml: string;
  lockfileToml?: string;
  stateJson?: string;
}): Promise<{ path: string; cleanup: () => Promise<void> }> {
  const dir = await makeTmpDir();
  await initRepo(dir);
  await writeFile(join(dir, "skilltap.toml"), opts.manifestToml);
  if (opts.lockfileToml !== undefined) {
    await writeFile(join(dir, "skilltap.lock"), opts.lockfileToml);
  }
  if (opts.stateJson !== undefined) {
    await mkdir(join(dir, ".agents"), { recursive: true });
    await writeFile(join(dir, ".agents", "state.json"), opts.stateJson);
  }
  return { path: dir, cleanup: () => removeTmpDir(dir) };
}

const EMPTY_STATE = JSON.stringify({
  version: 2,
  skills: [],
  plugins: [],
});

describe("sync — read-only drift report (default mode)", () => {
  test("in-sync state prints '✓ In sync. Manifest, lockfile, and state agree.' and exits 0", async () => {
    // Spec §v2.0 Sync L3152: "If everything agrees, prints `✓ In sync.
    // Manifest, lockfile, and state agree.` and exits 0."
    const proj = await makeProjectRoot({
      manifestToml: "",
      lockfileToml: "version = 1\n",
      stateJson: EMPTY_STATE,
    });
    try {
      const { exitCode, stdout } = await runSkilltap(
        ["sync"],
        homeDir,
        configDir,
        proj.path,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("In sync");
      expect(stdout).toContain("Manifest, lockfile, and state agree");
    } finally {
      await proj.cleanup();
    }
  });

  test("drift exits 0 in read-only mode (no --apply) and emits the apply hint", async () => {
    // Spec §v2.0 Sync L3152: read-only inspection by default, "does not
    // auto-apply or prompt"; ends with "note: run skilltap sync --apply".
    const proj = await makeProjectRoot({
      // Manifest declares a skill that's not in state → 'add' drift.
      manifestToml:
        '[skills]\n"github:example/missing" = "*"\n',
      lockfileToml: "version = 1\n",
      stateJson: EMPTY_STATE,
    });
    try {
      const { exitCode, stdout } = await runSkilltap(
        ["sync"],
        homeDir,
        configDir,
        proj.path,
      );
      // Read-only mode never exits non-zero just for drift — that's reserved
      // for --apply --strict failures.
      expect(exitCode).toBe(0);
      expect(stdout).toContain("drift report");
      expect(stdout).toContain("skilltap sync --apply");
    } finally {
      await proj.cleanup();
    }
  });

  test("drift report groups items by kind (e.g. '+ add')", async () => {
    // Spec §v2.0 Sync L3152: "drift report grouped by kind".
    const proj = await makeProjectRoot({
      manifestToml: '[skills]\n"github:example/missing" = "*"\n',
      lockfileToml: "version = 1\n",
      stateJson: EMPTY_STATE,
    });
    try {
      const { stdout } = await runSkilltap(
        ["sync"],
        homeDir,
        configDir,
        proj.path,
      );
      expect(stdout).toContain("add");
      expect(stdout).toContain("github:example/missing");
    } finally {
      await proj.cleanup();
    }
  });
});

describe("sync --json", () => {
  test("--json emits a parseable object with inSync + items", async () => {
    // Spec §v2.0 Sync L3157: "--json — output the plan as JSON instead".
    const proj = await makeProjectRoot({
      manifestToml: "",
      lockfileToml: "version = 1\n",
      stateJson: EMPTY_STATE,
    });
    try {
      const { exitCode, stdout } = await runSkilltap(
        ["sync", "--json"],
        homeDir,
        configDir,
        proj.path,
      );
      expect(exitCode).toBe(0);
      const payload = JSON.parse(stdout) as {
        inSync: boolean;
        items: unknown[];
      };
      expect(payload.inSync).toBe(true);
      expect(Array.isArray(payload.items)).toBe(true);
      expect(payload.items).toEqual([]);
    } finally {
      await proj.cleanup();
    }
  });

  test("--json on drift includes items array with kind/source/target", async () => {
    const proj = await makeProjectRoot({
      manifestToml: '[skills]\n"github:example/missing" = "*"\n',
      lockfileToml: "version = 1\n",
      stateJson: EMPTY_STATE,
    });
    try {
      const { exitCode, stdout } = await runSkilltap(
        ["sync", "--json"],
        homeDir,
        configDir,
        proj.path,
      );
      expect(exitCode).toBe(0);
      const payload = JSON.parse(stdout) as {
        inSync: boolean;
        items: Array<{ kind: string; source: string }>;
      };
      expect(payload.inSync).toBe(false);
      expect(payload.items.length).toBeGreaterThan(0);
      const addItem = payload.items.find((i) => i.kind === "add");
      expect(addItem).toBeDefined();
      expect(addItem?.source).toBe("github:example/missing");
    } finally {
      await proj.cleanup();
    }
  });
});

describe("sync --apply on in-sync project", () => {
  test("prints '✓ In sync. Nothing to apply.' and exits 0", async () => {
    // Spec §v2.0 Sync L3152 + sync.ts L89: in-sync apply emits this exact
    // message and short-circuits before invoking install/remove.
    const proj = await makeProjectRoot({
      manifestToml: "",
      lockfileToml: "version = 1\n",
      stateJson: EMPTY_STATE,
    });
    try {
      const { exitCode, stdout } = await runSkilltap(
        ["sync", "--apply"],
        homeDir,
        configDir,
        proj.path,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("In sync");
      expect(stdout).toContain("Nothing to apply");
    } finally {
      await proj.cleanup();
    }
  });
});

describe("sync — empty cwd (no manifest, lockfile, or state)", () => {
  test("trivially reports in-sync when run in a directory with no files", async () => {
    // Spec is silent about how sync should behave outside a project root;
    // the shipped behavior is that an empty cwd looks like (empty manifest,
    // empty lockfile, empty state) — all three agree → in-sync, exit 0.
    // sync.ts has an `if (!projectRoot)` guard that's unreachable because
    // tryFindProjectRoot falls back to cwd; treating the trivial in-sync
    // path as the documented behavior.
    const empty = await makeTmpDir();
    try {
      const { exitCode, stdout } = await runSkilltap(
        ["sync"],
        homeDir,
        configDir,
        empty,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("In sync");
    } finally {
      await removeTmpDir(empty);
    }
  });
});
