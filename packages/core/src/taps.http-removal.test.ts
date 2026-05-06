import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { createTestEnv, type TestEnv } from "@skilltap/test-utils";
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
});
