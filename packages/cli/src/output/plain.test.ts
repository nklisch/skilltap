import { describe, expect, test } from "bun:test";
import { Writable } from "node:stream";
import { createPlainOutput } from "./plain";

function makeMockStream(): Writable & { written: string } {
  const chunks: string[] = [];
  const stream = new Writable({
    write(chunk, _encoding, callback) {
      chunks.push(chunk.toString());
      callback();
    },
  }) as Writable & { written: string };
  Object.defineProperty(stream, "written", {
    get() {
      return chunks.join("");
    },
  });
  return stream;
}

const ANSI_RE = /\x1b\[[0-9;]*m/;

function hasNoAnsi(text: string): boolean {
  return !ANSI_RE.test(text);
}

describe("createPlainOutput", () => {
  test("no ANSI codes in success output", () => {
    const stdout = makeMockStream();
    const stderr = makeMockStream();
    const out = createPlainOutput({ stdout, stderr });
    out.success("Installed X");
    expect(hasNoAnsi(stdout.written)).toBe(true);
  });

  test("no ANSI codes in error output", () => {
    const stdout = makeMockStream();
    const stderr = makeMockStream();
    const out = createPlainOutput({ stdout, stderr });
    out.error("Something failed", "try again");
    expect(hasNoAnsi(stderr.written)).toBe(true);
  });

  test("no ANSI codes in warn output", () => {
    const stdout = makeMockStream();
    const stderr = makeMockStream();
    const out = createPlainOutput({ stdout, stderr });
    out.warn("heads up");
    expect(hasNoAnsi(stderr.written)).toBe(true);
  });

  test("no ANSI codes in progress output", () => {
    const stdout = makeMockStream();
    const stderr = makeMockStream();
    const out = createPlainOutput({ stdout, stderr });
    const p = out.progress("Cloning");
    p.succeed("Cloned");
    expect(hasNoAnsi(stdout.written)).toBe(true);
  });

  test("success writes message + newline to stdout", () => {
    const stdout = makeMockStream();
    const stderr = makeMockStream();
    const out = createPlainOutput({ stdout, stderr });
    out.success("Installed X");
    expect(stdout.written).toBe("Installed X\n");
  });

  test("info writes message + newline to stdout", () => {
    const stdout = makeMockStream();
    const stderr = makeMockStream();
    const out = createPlainOutput({ stdout, stderr });
    out.info("some info");
    expect(stdout.written).toBe("some info\n");
  });

  test("error writes to stderr", () => {
    const stdout = makeMockStream();
    const stderr = makeMockStream();
    const out = createPlainOutput({ stdout, stderr });
    out.error("bad thing", "do this");
    expect(stderr.written).toBe("error: bad thing\n  hint: do this\n");
    expect(stdout.written).toBe("");
  });

  test("warn writes to stderr", () => {
    const stdout = makeMockStream();
    const stderr = makeMockStream();
    const out = createPlainOutput({ stdout, stderr });
    out.warn("heads up", "check docs");
    expect(stderr.written).toBe("warning: heads up\n  hint: check docs\n");
  });

  test("progress emits label... on start and done on succeed", () => {
    const stdout = makeMockStream();
    const stderr = makeMockStream();
    const out = createPlainOutput({ stdout, stderr });
    const p = out.progress("Cloning");
    expect(stdout.written).toContain("Cloning...");
    p.succeed("Cloned");
    expect(stdout.written).toContain("Cloned done");
  });

  test("progress fail writes to stderr", () => {
    const stdout = makeMockStream();
    const stderr = makeMockStream();
    const out = createPlainOutput({ stdout, stderr });
    const p = out.progress("Cloning");
    p.fail("Clone failed");
    expect(stderr.written).toContain("error");
    expect(stderr.written).toContain("Clone failed");
  });

  test("progress update is a no-op (no output)", () => {
    const stdout = makeMockStream();
    const stderr = makeMockStream();
    const out = createPlainOutput({ stdout, stderr });
    const p = out.progress("Loading");
    const writtenBefore = stdout.written;
    p.update("still loading");
    expect(stdout.written).toBe(writtenBefore);
  });

  test("pause and resume are no-ops", () => {
    const stdout = makeMockStream();
    const stderr = makeMockStream();
    const out = createPlainOutput({ stdout, stderr });
    const p = out.progress("Working");
    const beforePause = stdout.written;
    p.pause();
    expect(stdout.written).toBe(beforePause);
    p.resume();
    expect(stdout.written).toBe(beforePause);
  });

  test("quiet=true suppresses info but not success", () => {
    const stdout = makeMockStream();
    const stderr = makeMockStream();
    const out = createPlainOutput({ quiet: true, stdout, stderr });
    out.info("hidden");
    out.success("still shown");
    expect(stdout.written).not.toContain("hidden");
    expect(stdout.written).toContain("still shown");
  });

  test("quiet=true still emits error", () => {
    const stdout = makeMockStream();
    const stderr = makeMockStream();
    const out = createPlainOutput({ quiet: true, stdout, stderr });
    out.error("still shown");
    expect(stderr.written).toContain("still shown");
  });

  test("json() is a no-op", () => {
    const stdout = makeMockStream();
    const stderr = makeMockStream();
    const out = createPlainOutput({ stdout, stderr });
    out.json({ kind: "update:done" });
    expect(stdout.written).toBe("");
    expect(stderr.written).toBe("");
  });

  test("block defaults to stderr", () => {
    const stdout = makeMockStream();
    const stderr = makeMockStream();
    const out = createPlainOutput({ stdout, stderr });
    out.block(["a", "b"]);
    expect(stderr.written).toContain("a");
    expect(stderr.written).toContain("b");
    expect(stdout.written).toBe("");
  });

  test("raw writes to stdout", () => {
    const stdout = makeMockStream();
    const stderr = makeMockStream();
    const out = createPlainOutput({ stdout, stderr });
    out.raw("verbatim");
    expect(stdout.written).toBe("verbatim");
  });

  test("mode is 'plain'", () => {
    const out = createPlainOutput({});
    expect(out.mode).toBe("plain");
  });
});
