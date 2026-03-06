import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, setDefaultTimeout, test } from "bun:test";
setDefaultTimeout(45_000);
import {
  commitAll,
  createStandaloneSkillRepo,
  initRepo,
  makeTmpDir,
  removeTmpDir,
  runInteractive,
  runSkilltap,
} from "@skilltap/test-utils";

const CLI_DIR = `${import.meta.dir}/../../..`;
const CMD = ["bun", "run", "--bun", "src/index.ts"] as const;

let homeDir: string;
let configDir: string;

beforeEach(async () => {
  homeDir = await makeTmpDir();
  configDir = await makeTmpDir();
});

afterEach(async () => {
  await removeTmpDir(homeDir);
  await removeTmpDir(configDir);
});

function env() {
  return {
    SKILLTAP_HOME: homeDir,
    XDG_CONFIG_HOME: configDir,
    DO_NOT_TRACK: "1",
  };
}

async function writeConfig(toml: string): Promise<void> {
  const dir = join(configDir, "skilltap");
  await mkdir(dir, { recursive: true });
  await Bun.write(join(dir, "config.toml"), toml);
}

async function createLocalTap(
  skills: Array<{ name: string; description: string; repo: string; tags?: string[] }>,
): Promise<{ path: string; cleanup: () => Promise<void> }> {
  const tapDir = await makeTmpDir();
  const tapJson = {
    name: "test-tap",
    description: "Test tap",
    skills: skills.map((s) => ({ tags: [], ...s })),
  };
  await Bun.write(join(tapDir, "tap.json"), JSON.stringify(tapJson, null, 2));
  await initRepo(tapDir);
  await commitAll(tapDir);
  return { path: tapDir, cleanup: () => removeTmpDir(tapDir) };
}

async function addTap(tapPath: string): Promise<void> {
  const proc = Bun.spawn([...CMD, "tap", "add", "home", tapPath], {
    cwd: CLI_DIR,
    stdout: "pipe",
    stderr: "pipe",
    env: { ...process.env, SKILLTAP_HOME: homeDir, XDG_CONFIG_HOME: configDir },
  });
  const exitCode = await proc.exited;
  if (exitCode !== 0) {
    const stderr = await new Response(proc.stderr).text();
    throw new Error(`tap add failed (code ${exitCode}): ${stderr}`);
  }
}

describe("tap install — agent selection prompt", () => {
  test(
    "shows agent selection prompt after scope selection when no defaults set",
    async () => {
      await writeConfig("builtin_tap = false\n");
      const tap = await createLocalTap([
        {
          name: "commit-helper",
          description: "Generates commit messages",
          repo: "https://example.com/a",
        },
      ]);
      try {
        await addTap(tap.path);

        const session = await runInteractive(
          [...CMD, "tap", "install"],
          { cwd: CLI_DIR, env: env() },
        );

        await session.waitForText("Select tap skills to install:");
        await session.waitForText("commit-helper");
        session.sendKey("SPACE"); // toggle selection
        session.sendKey("ENTER"); // confirm

        await session.waitForText("Install to:");
        session.sendKey("ENTER"); // accept Global

        await session.waitForText("Which agents should this skill be available to?");

        session.sendKey("CTRL_C");
        const { exitCode } = await session.finish();
        expect(exitCode).toBe(2);
      } finally {
        await tap.cleanup();
      }
    },
    30_000,
  );

  test(
    "--yes skips agent selection prompt",
    async () => {
      await writeConfig("builtin_tap = false\n");
      const repo = await createStandaloneSkillRepo();
      const tap = await createLocalTap([
        {
          name: "standalone-skill",
          description: "A standalone skill",
          repo: repo.path,
        },
      ]);
      try {
        await addTap(tap.path);

        const { exitCode, stdout, stderr } = await runSkilltap(
          ["tap", "install", "--yes", "--global", "--skip-scan"],
          homeDir,
          configDir,
        );

        expect(exitCode).toBe(0);
        expect(stdout + stderr).not.toContain("Which agents should this skill be available to?");
      } finally {
        await tap.cleanup();
        await repo.cleanup();
      }
    },
  );
});
