import { describe, expect, test } from "bun:test";
import { Writable } from "node:stream";
import { createTtyOutput } from "./tty";

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

// Matches any ANSI escape sequence
// biome-ignore lint/suspicious/noControlCharactersInRegex: checking for ANSI codes
const ANSI_RE = /\x1b\[[0-9;]*m/;

describe("createTtyOutput", () => {
  test("success writes green check + message to stdout", () => {
    const stdout = makeMockStream();
    const stderr = makeMockStream();
    const out = createTtyOutput({ isTTY: true, stdout, stderr });
    out.success("All done");
    expect(stdout.written).toContain("✓");
    expect(stdout.written).toContain("All done");
    expect(ANSI_RE.test(stdout.written)).toBe(true);
  });

  test("error writes red error + message to stderr", () => {
    const stdout = makeMockStream();
    const stderr = makeMockStream();
    const out = createTtyOutput({ isTTY: true, stdout, stderr });
    out.error("Something broke", "try again");
    expect(stderr.written).toContain("error");
    expect(stderr.written).toContain("Something broke");
    expect(stderr.written).toContain("hint");
    expect(stderr.written).toContain("try again");
    expect(ANSI_RE.test(stderr.written)).toBe(true);
  });

  test("error without hint omits hint line", () => {
    const stdout = makeMockStream();
    const stderr = makeMockStream();
    const out = createTtyOutput({ isTTY: true, stdout, stderr });
    out.error("Failed");
    expect(stderr.written).not.toContain("hint");
  });

  test("info writes message to stdout", () => {
    const stdout = makeMockStream();
    const stderr = makeMockStream();
    const out = createTtyOutput({ isTTY: true, stdout, stderr });
    out.info("some info");
    expect(stdout.written).toContain("some info");
  });

  test("warn writes yellow warning to stderr", () => {
    const stdout = makeMockStream();
    const stderr = makeMockStream();
    const out = createTtyOutput({ isTTY: true, stdout, stderr });
    out.warn("heads up", "check the docs");
    expect(stderr.written).toContain("warning");
    expect(stderr.written).toContain("heads up");
    expect(stderr.written).toContain("hint");
    expect(stderr.written).toContain("check the docs");
    expect(ANSI_RE.test(stderr.written)).toBe(true);
  });

  test("quiet=true suppresses info but not success", () => {
    const stdout = makeMockStream();
    const stderr = makeMockStream();
    const out = createTtyOutput({ isTTY: true, quiet: true, stdout, stderr });
    out.info("should not appear");
    out.success("still shown");
    expect(stdout.written).not.toContain("should not appear");
    expect(stdout.written).toContain("still shown");
  });

  test("quiet=true still emits error", () => {
    const stdout = makeMockStream();
    const stderr = makeMockStream();
    const out = createTtyOutput({ isTTY: true, quiet: true, stdout, stderr });
    out.error("real error");
    expect(stderr.written).toContain("real error");
  });

  test("json() is a no-op", () => {
    const stdout = makeMockStream();
    const stderr = makeMockStream();
    const out = createTtyOutput({ isTTY: true, stdout, stderr });
    out.json({ kind: "install:done" });
    expect(stdout.written).toBe("");
    expect(stderr.written).toBe("");
  });

  test("block writes to stderr by default", () => {
    const stdout = makeMockStream();
    const stderr = makeMockStream();
    const out = createTtyOutput({ isTTY: true, stdout, stderr });
    out.block(["line1", "line2"]);
    expect(stderr.written).toContain("line1");
    expect(stderr.written).toContain("line2");
    expect(stdout.written).toBe("");
  });

  test("block with stream:'stdout' writes to stdout", () => {
    const stdout = makeMockStream();
    const stderr = makeMockStream();
    const out = createTtyOutput({ isTTY: true, stdout, stderr });
    out.block(["line1"], { stream: "stdout" });
    expect(stdout.written).toContain("line1");
    expect(stderr.written).toBe("");
  });

  test("raw writes directly to stdout", () => {
    const stdout = makeMockStream();
    const stderr = makeMockStream();
    const out = createTtyOutput({ isTTY: true, stdout, stderr });
    out.raw("raw text");
    expect(stdout.written).toBe("raw text");
  });

  test("mode is 'tty'", () => {
    const out = createTtyOutput({ isTTY: true });
    expect(out.mode).toBe("tty");
  });

  test("quiet=true: progress returns noop (no crash)", () => {
    const out = createTtyOutput({ isTTY: true, quiet: true });
    const p = out.progress("Loading");
    expect(() => {
      p.update("still loading");
      p.succeed("done");
    }).not.toThrow();
  });
});
