import { describe, expect, test } from "bun:test";
import { Writable } from "node:stream";
import { createJsonOutput } from "./json";

function makeMockStream(): Writable & { written: string; lines: string[] } {
  const chunks: string[] = [];
  const stream = new Writable({
    write(chunk, _encoding, callback) {
      chunks.push(chunk.toString());
      callback();
    },
  }) as Writable & { written: string; lines: string[] };
  Object.defineProperty(stream, "written", {
    get() {
      return chunks.join("");
    },
  });
  Object.defineProperty(stream, "lines", {
    get() {
      return chunks.join("").split("\n").filter(Boolean);
    },
  });
  return stream;
}

function parseLines(stream: { lines: string[] }): unknown[] {
  return stream.lines.map((line) => JSON.parse(line));
}

describe("createJsonOutput", () => {
  test("info produces no output", () => {
    const stdout = makeMockStream();
    const out = createJsonOutput({ stdout });
    out.info("hello");
    expect(stdout.written).toBe("");
  });

  test("success produces no output", () => {
    const stdout = makeMockStream();
    const out = createJsonOutput({ stdout });
    out.success("done");
    expect(stdout.written).toBe("");
  });

  test("block produces no output", () => {
    const stdout = makeMockStream();
    const out = createJsonOutput({ stdout });
    out.block(["line1", "line2"]);
    expect(stdout.written).toBe("");
  });

  test("error writes { kind: 'error' } JSON line to stdout", () => {
    const stdout = makeMockStream();
    const out = createJsonOutput({ stdout });
    out.error("bad thing", "fix it");
    const [parsed] = parseLines(stdout) as Array<{ kind: string; message: string; hint: string }>;
    expect(parsed.kind).toBe("error");
    expect(parsed.message).toBe("bad thing");
    expect(parsed.hint).toBe("fix it");
  });

  test("warn writes { kind: 'warn' } JSON line to stdout", () => {
    const stdout = makeMockStream();
    const out = createJsonOutput({ stdout });
    out.warn("heads up");
    const [parsed] = parseLines(stdout) as Array<{ kind: string; message: string }>;
    expect(parsed.kind).toBe("warn");
    expect(parsed.message).toBe("heads up");
  });

  test("json(event) writes the event as JSON", () => {
    const stdout = makeMockStream();
    const out = createJsonOutput({ stdout });
    out.json({ kind: "install:done", records: ["foo"] });
    const [parsed] = parseLines(stdout) as Array<{ kind: string; records: string[] }>;
    expect(parsed.kind).toBe("install:done");
    expect(parsed.records).toEqual(["foo"]);
  });

  test("every emission is followed by exactly one newline", () => {
    const stdout = makeMockStream();
    const out = createJsonOutput({ stdout });
    out.error("x");
    out.json({ kind: "y" });
    expect(stdout.written.endsWith("\n")).toBe(true);
    // Each line ends with exactly one newline
    const parts = stdout.written.split("\n");
    // Last entry should be empty string (trailing newline)
    expect(parts[parts.length - 1]).toBe("");
  });

  test("progress lifecycle emits progress:start, progress:update, progress:done", () => {
    const stdout = makeMockStream();
    const out = createJsonOutput({ stdout });
    const p = out.progress("Loading");
    p.update("halfway");
    p.succeed("loaded");

    const events = parseLines(stdout) as Array<{ kind: string; label: string; message?: string }>;
    expect(events[0]).toMatchObject({ kind: "progress:start", label: "Loading" });
    expect(events[1]).toMatchObject({ kind: "progress:update", label: "Loading", message: "halfway" });
    expect(events[2]).toMatchObject({ kind: "progress:done", label: "Loading", message: "loaded" });
  });

  test("progress fail emits progress:fail event", () => {
    const stdout = makeMockStream();
    const out = createJsonOutput({ stdout });
    const p = out.progress("Cloning");
    p.fail("network error");

    const events = parseLines(stdout) as Array<{ kind: string; message?: string }>;
    expect(events[1]).toMatchObject({ kind: "progress:fail", message: "network error" });
  });

  test("all emitted lines are valid JSON", () => {
    const stdout = makeMockStream();
    const out = createJsonOutput({ stdout });
    out.error("e");
    out.warn("w");
    out.json({ kind: "x" });
    const p = out.progress("L");
    p.update("u");
    p.succeed("s");

    for (const line of stdout.lines) {
      expect(() => JSON.parse(line)).not.toThrow();
    }
  });

  test("raw writes directly to stdout", () => {
    const stdout = makeMockStream();
    const out = createJsonOutput({ stdout });
    out.raw("verbatim text");
    expect(stdout.written).toBe("verbatim text");
  });

  test("mode is 'json'", () => {
    const out = createJsonOutput({});
    expect(out.mode).toBe("json");
  });

  test("pause and resume are no-ops", () => {
    const stdout = makeMockStream();
    const out = createJsonOutput({ stdout });
    const p = out.progress("Working");
    const linesBefore = stdout.lines.length;
    p.pause();
    p.resume();
    expect(stdout.lines.length).toBe(linesBefore);
  });
});
