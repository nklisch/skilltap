import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { stat } from "node:fs/promises";

export type TestEnv = {
  homeDir: string;
  configDir: string;
  cleanup: () => Promise<void>;
};

/**
 * Create an isolated test environment with temp dirs for SKILLTAP_HOME and XDG_CONFIG_HOME.
 * Saves the current env values, sets them to temp dirs, and returns a cleanup function
 * that restores the originals and removes the dirs.
 *
 * Usage:
 * ```ts
 * let env: TestEnv;
 * beforeEach(async () => { env = await createTestEnv(); });
 * afterEach(async () => { await env.cleanup(); });
 * ```
 */
export async function createTestEnv(): Promise<TestEnv> {
  const homeDir = await mkdtemp(join(tmpdir(), "skilltap-test-"));
  const configDir = await mkdtemp(join(tmpdir(), "skilltap-cfg-"));

  const savedHome = process.env.SKILLTAP_HOME;
  const savedXdg = process.env.XDG_CONFIG_HOME;

  process.env.SKILLTAP_HOME = homeDir;
  process.env.XDG_CONFIG_HOME = configDir;

  return {
    homeDir,
    configDir,
    cleanup: async () => {
      if (savedHome !== undefined) process.env.SKILLTAP_HOME = savedHome;
      else delete process.env.SKILLTAP_HOME;
      if (savedXdg !== undefined) process.env.XDG_CONFIG_HOME = savedXdg;
      else delete process.env.XDG_CONFIG_HOME;
      await rm(homeDir, { recursive: true, force: true });
      await rm(configDir, { recursive: true, force: true });
    },
  };
}

/**
 * Check if a path exists (any type — file, directory, symlink).
 * Useful in tests for verifying filesystem side effects.
 */
export async function pathExists(p: string): Promise<boolean> {
  try {
    await stat(p);
    return true;
  } catch {
    return false;
  }
}
