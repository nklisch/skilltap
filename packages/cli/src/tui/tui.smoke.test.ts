import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { createTestEnv, runInteractive, type TestEnv } from "@skilltap/test-utils";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const CLI_ENTRY = join(
  dirname(fileURLToPath(import.meta.url)),
  "..",
  "index.ts",
);

let env: TestEnv;

beforeEach(async () => {
  env = await createTestEnv();
});

afterEach(async () => {
  await env.cleanup();
});

describe("TUI smoke", () => {
  test("bare skilltap (TTY) opens dashboard and exits on q", async () => {
    const session = await runInteractive(["bun", "run", CLI_ENTRY], {
      cwd: process.cwd(),
      env: {
        SKILLTAP_HOME: env.homeDir,
        XDG_CONFIG_HOME: env.configDir,
        SKILLTAP_NO_STARTUP: "1",
      },
    });
    await session.waitForText("Installed", 8000);
    session.send("q");
    const { exitCode } = await session.finish(5000);
    expect(exitCode).toBe(0);
  });

  test("global key 2 navigates to Find screen", async () => {
    const session = await runInteractive(["bun", "run", CLI_ENTRY], {
      cwd: process.cwd(),
      env: {
        SKILLTAP_HOME: env.homeDir,
        XDG_CONFIG_HOME: env.configDir,
        SKILLTAP_NO_STARTUP: "1",
      },
    });
    await session.waitForText("Installed", 8000);
    session.send("2");
    // Find screen renders "Search:" — no heading says "Find" explicitly
    await session.waitForText("Search:", 3000);
    session.send("q");
    const { exitCode } = await session.finish(5000);
    expect(exitCode).toBe(0);
  });

  test("Ctrl+C exits cleanly", async () => {
    const session = await runInteractive(["bun", "run", CLI_ENTRY], {
      cwd: process.cwd(),
      env: {
        SKILLTAP_HOME: env.homeDir,
        XDG_CONFIG_HOME: env.configDir,
        SKILLTAP_NO_STARTUP: "1",
      },
    });
    await session.waitForText("Installed", 8000);
    session.sendKey("CTRL_C");
    const { exitCode } = await session.finish(5000);
    // Ink exits cleanly with 0 on SIGINT; some environments yield 130.
    expect([0, 130]).toContain(exitCode);
  });

  test("bare skilltap (non-TTY) errors with hint", async () => {
    const proc = Bun.spawn(
      ["bun", "run", CLI_ENTRY],
      {
        cwd: process.cwd(),
        env: {
          ...process.env,
          SKILLTAP_HOME: env.homeDir,
          XDG_CONFIG_HOME: env.configDir,
          SKILLTAP_NO_STARTUP: "1",
        },
        stdin: "pipe",
        stdout: "pipe",
        stderr: "pipe",
      },
    );
    const stderr = await new Response(proc.stderr).text();
    await proc.exited;
    expect(proc.exitCode).toBe(1);
    expect(stderr).toContain("skilltap requires a TTY");
    expect(stderr).toContain("skilltap status");
  });
});
