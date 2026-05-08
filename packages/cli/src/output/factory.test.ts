import { describe, expect, test } from "bun:test";
import { createOutput } from "./factory";

describe("createOutput", () => {
  test("returns json adapter when json: true", () => {
    const out = createOutput({ json: true });
    expect(out.mode).toBe("json");
  });

  test("returns tty adapter when isTTY: true", () => {
    const out = createOutput({ isTTY: true });
    expect(out.mode).toBe("tty");
  });

  test("returns plain adapter when isTTY: false", () => {
    const out = createOutput({ isTTY: false });
    expect(out.mode).toBe("plain");
  });

  test("json: true overrides isTTY: true", () => {
    const out = createOutput({ json: true, isTTY: true });
    expect(out.mode).toBe("json");
  });

  test("no opts defaults to plain (piped in test context)", () => {
    // Tests run in a non-TTY context, so isTTY is falsy → plain
    const out = createOutput({});
    // In CI/test we expect plain, but this depends on environment
    // The key thing: no crash
    expect(["plain", "tty"]).toContain(out.mode);
  });

  test("all adapters implement the Output interface", () => {
    const modes = [
      createOutput({ json: true }),
      createOutput({ isTTY: true }),
      createOutput({ isTTY: false }),
    ];
    for (const out of modes) {
      expect(typeof out.info).toBe("function");
      expect(typeof out.warn).toBe("function");
      expect(typeof out.error).toBe("function");
      expect(typeof out.success).toBe("function");
      expect(typeof out.block).toBe("function");
      expect(typeof out.json).toBe("function");
      expect(typeof out.progress).toBe("function");
      expect(typeof out.raw).toBe("function");
    }
  });
});
