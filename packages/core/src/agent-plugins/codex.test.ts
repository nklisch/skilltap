import { describe, expect, test } from "bun:test";
import { createCodexScanner } from "./codex";

describe("createCodexScanner", () => {
  test("detect() always returns false", async () => {
    const scanner = createCodexScanner();
    expect(await scanner.detect()).toBe(false);
  });

  test("scan() always returns ok([])", async () => {
    const scanner = createCodexScanner();
    const result = await scanner.scan();
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value).toEqual([]);
  });

  test("name is 'codex'", () => {
    const scanner = createCodexScanner();
    expect(scanner.name).toBe("codex");
  });
});
