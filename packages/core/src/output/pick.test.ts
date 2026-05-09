import { describe, expect, test } from "bun:test";
import { pickMode } from "./pick";

describe("pickMode", () => {
  test("explicit json: true → 'json' regardless of TTY", () => {
    expect(pickMode({ json: true, isTTY: true })).toBe("json");
    expect(pickMode({ json: true, isTTY: false })).toBe("json");
    expect(pickMode({ json: true })).toBe("json");
  });

  test("json: false + isTTY: true → 'tty'", () => {
    expect(pickMode({ json: false, isTTY: true })).toBe("tty");
  });

  test("json: false + isTTY: false → 'plain'", () => {
    expect(pickMode({ json: false, isTTY: false })).toBe("plain");
  });

  test("no opts + process.stdout.isTTY falsy → 'plain'", () => {
    // In test/subprocess context isTTY is undefined (piped) — plain
    const original = process.stdout.isTTY;
    Object.defineProperty(process.stdout, "isTTY", {
      value: undefined,
      configurable: true,
    });
    expect(pickMode()).toBe("plain");
    Object.defineProperty(process.stdout, "isTTY", {
      value: original,
      configurable: true,
    });
  });

  test("no opts + process.stdout.isTTY === true → 'tty'", () => {
    const original = process.stdout.isTTY;
    Object.defineProperty(process.stdout, "isTTY", {
      value: true,
      configurable: true,
    });
    expect(pickMode()).toBe("tty");
    Object.defineProperty(process.stdout, "isTTY", {
      value: original,
      configurable: true,
    });
  });

  test("isTTY override takes precedence over process.stdout.isTTY", () => {
    const original = process.stdout.isTTY;
    Object.defineProperty(process.stdout, "isTTY", {
      value: true,
      configurable: true,
    });
    expect(pickMode({ isTTY: false })).toBe("plain");
    Object.defineProperty(process.stdout, "isTTY", {
      value: original,
      configurable: true,
    });
  });
});
