import {
  afterEach,
  beforeEach,
  describe,
  expect,
  setDefaultTimeout,
  test,
} from "bun:test";
import {
  cliCmd,
  createTestEnv,
  runInteractive,
  type TestEnv,
} from "@skilltap/test-utils";

setDefaultTimeout(60_000);

let env: TestEnv;

beforeEach(async () => {
  env = await createTestEnv();
});

afterEach(async () => {
  await env.cleanup();
});

describe("TUI smoke", () => {
  test("bare skilltap (TTY) opens dashboard and exits on q", async () => {
    const session = await runInteractive(cliCmd(), {
      cwd: process.cwd(),
      env: {
        SKILLTAP_HOME: env.homeDir,
        XDG_CONFIG_HOME: env.configDir,
        SKILLTAP_NO_STARTUP: "1",
      },
    });
    await session.waitForText("Installed", 20_000);
    session.send("q");
    const { exitCode } = await session.finish(5000);
    expect(exitCode).toBe(0);
  });

  test("dashboard 'f' key navigates to Find screen, then 1-4 are dashboard tab keys", async () => {
    // Combined coverage: verifies (1) the per-screen `f` binding navigates to
    // Find, and (2) on Dashboard the `1-4` keys are intercepted as tab
    // switches (Unit 3.17 fix) instead of being the global navigate keys.
    // Combined into one PTY session to keep the smoke suite small — under
    // cold-cache, every additional spawn adds compile cost.
    const session = await runInteractive(cliCmd(), {
      cwd: process.cwd(),
      env: {
        SKILLTAP_HOME: env.homeDir,
        XDG_CONFIG_HOME: env.configDir,
        SKILLTAP_NO_STARTUP: "1",
      },
    });
    await session.waitForText("Installed", 20_000);
    // Ink enables raw mode via useEffect after the first paint; "Installed"
    // appearing in the buffer comes from the initial render but the input
    // pipeline may not be wired yet. Sleep briefly so the first key isn't
    // dropped (echoes back into the buffer instead of dispatching).
    await new Promise((resolve) => setTimeout(resolve, 200));
    // 1-4 stay on Dashboard.
    session.send("2");
    await session.waitForText("2 Taps", 3000);
    // `f` navigates to Find.
    session.send("f");
    await session.waitForText("Search:", 3000);
    session.send("q");
    const { exitCode } = await session.finish(5000);
    expect(exitCode).toBe(0);
  });

  test("Ctrl+C exits cleanly", async () => {
    const session = await runInteractive(cliCmd(), {
      cwd: process.cwd(),
      env: {
        SKILLTAP_HOME: env.homeDir,
        XDG_CONFIG_HOME: env.configDir,
        SKILLTAP_NO_STARTUP: "1",
      },
    });
    await session.waitForText("Installed", 20_000);
    session.sendKey("CTRL_C");
    const { exitCode } = await session.finish(5000);
    // Ink exits cleanly with 0 on SIGINT; some environments yield 130.
    expect([0, 130]).toContain(exitCode);
  });

  test("bare skilltap (non-TTY) errors with hint", async () => {
    const proc = Bun.spawn([...cliCmd()], {
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
    });
    const stderr = await new Response(proc.stderr).text();
    await proc.exited;
    expect(proc.exitCode).toBe(1);
    expect(stderr).toContain("skilltap requires a TTY");
    expect(stderr).toContain("skilltap status");
  });
});
