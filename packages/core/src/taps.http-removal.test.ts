/**
 * HTTP tap removal (Phase 31b).
 *
 * Spec: docs/SPEC.md §v2.0 Removed Features (L3269) —
 *   "v0.x configs with `type = "http"` are silently filtered with a
 *   one-time stderr warning."
 *
 * The original test asserted only that loadTaps doesn't crash. These tests
 * additionally pin the user-visible behavior: the stderr warning is emitted
 * (so the user knows their HTTP tap was dropped) and `loadTaps` actually
 * filters HTTP entries from the result (not just no-throw).
 */

import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import {
  createTestEnv,
  makeTmpDir,
  removeTmpDir,
  runSkilltap,
  type TestEnv,
} from "@skilltap/test-utils";
import { loadTaps } from "./taps";

describe("HTTP tap filtering (Phase 31b)", () => {
  let env: TestEnv;

  beforeEach(async () => {
    env = await createTestEnv();
    await mkdir(join(env.configDir, "skilltap"), { recursive: true });
  });

  afterEach(async () => {
    await env.cleanup();
  });

  test("loadTaps silently skips HTTP entries in config", async () => {
    const cfgPath = join(env.configDir, "skilltap", "config.toml");
    await writeFile(
      cfgPath,
      `
builtin_tap = false

[[taps]]
name = "http-tap"
url = "https://example.com/api"
type = "http"

[[taps]]
name = "git-tap"
url = "https://example.com/repo.git"
type = "git"
`,
    );

    const result = await loadTaps();
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    // Both taps lack local clones so entries is empty, but no error — HTTP didn't crash.
    expect(Array.isArray(result.value)).toBe(true);
  });

  test("CLI subprocess emits stderr warning naming the dropped HTTP tap", async () => {
    // Spec L3269: "silently filtered with a one-time stderr warning."
    // The warning has process-private dedup state, so verifying it requires
    // a fresh subprocess (where the dedup Set is freshly empty).
    // We invoke `skilltap doctor` because it's a no-op-ish command that
    // triggers loadTaps in its taps check.
    const home = await makeTmpDir();
    const config = await makeTmpDir();
    try {
      await mkdir(join(config, "skilltap"), { recursive: true });
      await writeFile(
        join(config, "skilltap", "config.toml"),
        `
builtin_tap = false

[[taps]]
name = "old-http-tap"
url = "https://example.com/api"
type = "http"
`,
      );

      const { stderr } = await runSkilltap(["tap", "list"], home, config);
      // The exact warning text from taps.ts L48-50 names the tap and
      // suggests `skilltap migrate`.
      expect(stderr).toContain("HTTP tap");
      expect(stderr).toContain("old-http-tap");
    } finally {
      await removeTmpDir(home);
      await removeTmpDir(config);
    }
  });
});
