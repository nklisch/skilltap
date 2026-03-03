import { afterEach, describe, expect, test } from "bun:test";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { debug, flushDebug, resetDebug } from "./debug";

describe("debug", () => {
  let tmpDir: string;
  const origDebug = process.env.SKILLTAP_DEBUG;
  const origXdg = process.env.XDG_CONFIG_HOME;

  afterEach(async () => {
    resetDebug();
    if (origDebug === undefined) delete process.env.SKILLTAP_DEBUG;
    else process.env.SKILLTAP_DEBUG = origDebug;
    if (origXdg === undefined) delete process.env.XDG_CONFIG_HOME;
    else process.env.XDG_CONFIG_HOME = origXdg;
    if (tmpDir) await rm(tmpDir, { recursive: true, force: true });
  });

  test("no-op when SKILLTAP_DEBUG is not set", async () => {
    tmpDir = await mkdtemp(join(tmpdir(), "debug-test-"));
    process.env.XDG_CONFIG_HOME = tmpDir;
    delete process.env.SKILLTAP_DEBUG;

    debug("should not write");
    await flushDebug();

    const logPath = join(tmpDir, "skilltap", "debug.log");
    const exists = await Bun.file(logPath).exists();
    expect(exists).toBe(false);
  });

  test("writes to debug.log when SKILLTAP_DEBUG=1", async () => {
    tmpDir = await mkdtemp(join(tmpdir(), "debug-test-"));
    process.env.XDG_CONFIG_HOME = tmpDir;
    process.env.SKILLTAP_DEBUG = "1";

    debug("hello world");
    await flushDebug();

    const logPath = join(tmpDir, "skilltap", "debug.log");
    const content = await readFile(logPath, "utf-8");
    expect(content).toContain("hello world");
    expect(content).toMatch(/^\[\d{4}-\d{2}-\d{2}T/);
  });

  test("includes context as JSON", async () => {
    tmpDir = await mkdtemp(join(tmpdir(), "debug-test-"));
    process.env.XDG_CONFIG_HOME = tmpDir;
    process.env.SKILLTAP_DEBUG = "1";

    debug("test op", { exitCode: 18, stderr: "denied" });
    await flushDebug();

    const logPath = join(tmpDir, "skilltap", "debug.log");
    const content = await readFile(logPath, "utf-8");
    expect(content).toContain('"exitCode":18');
    expect(content).toContain('"stderr":"denied"');
  });

  test("rotates when log exceeds 1 MB", async () => {
    tmpDir = await mkdtemp(join(tmpdir(), "debug-test-"));
    process.env.XDG_CONFIG_HOME = tmpDir;
    process.env.SKILLTAP_DEBUG = "1";

    const logDir = join(tmpDir, "skilltap");
    const logPath = join(logDir, "debug.log");
    const { mkdir } = await import("node:fs/promises");
    await mkdir(logDir, { recursive: true });

    // Write >1 MB of content
    const bigContent = "X".repeat(1_200_000);
    await writeFile(logPath, bigContent);

    debug("after rotation");
    await flushDebug();

    const content = await readFile(logPath, "utf-8");
    // Should be truncated to ~500KB + new line
    expect(content.length).toBeLessThan(600_000);
    expect(content).toContain("after rotation");
  });

  test("multiple writes are serialized", async () => {
    tmpDir = await mkdtemp(join(tmpdir(), "debug-test-"));
    process.env.XDG_CONFIG_HOME = tmpDir;
    process.env.SKILLTAP_DEBUG = "1";

    debug("line 1");
    debug("line 2");
    debug("line 3");
    await flushDebug();

    const logPath = join(tmpDir, "skilltap", "debug.log");
    const content = await readFile(logPath, "utf-8");
    const lines = content.trim().split("\n");
    expect(lines).toHaveLength(3);
    expect(lines[0]).toContain("line 1");
    expect(lines[2]).toContain("line 3");
  });
});
