import { describe, expect, test } from "bun:test";
import { extractExitCode, extractStderr, wrapShell } from "./shell";

describe("extractStderr", () => {
  test("extracts Uint8Array stderr", () => {
    const e = Object.assign(new Error("fail"), {
      stderr: new TextEncoder().encode("permission denied"),
    });
    expect(extractStderr(e)).toBe("permission denied");
  });

  test("extracts string stderr", () => {
    const e = Object.assign(new Error("fail"), { stderr: "not found" });
    expect(extractStderr(e)).toBe("not found");
  });

  test("falls back to String(e) for plain errors", () => {
    const e = new Error("something broke");
    expect(extractStderr(e)).toBe("Error: something broke");
  });

  test("handles non-Error values", () => {
    expect(extractStderr("raw string")).toBe("raw string");
    expect(extractStderr(42)).toBe("42");
  });

  test("trims whitespace from stderr", () => {
    const e = Object.assign(new Error("fail"), { stderr: "  spaced  \n" });
    expect(extractStderr(e)).toBe("spaced");
  });
});

describe("extractExitCode", () => {
  test("extracts exitCode from ShellError-like object", () => {
    const e = Object.assign(new Error("fail"), { exitCode: 18 });
    expect(extractExitCode(e)).toBe(18);
  });

  test("returns undefined for plain errors", () => {
    expect(extractExitCode(new Error("nope"))).toBeUndefined();
  });

  test("returns undefined for non-Error values", () => {
    expect(extractExitCode("string")).toBeUndefined();
  });
});

describe("wrapShell", () => {
  test("returns ok on success", async () => {
    const result = await wrapShell(() => Promise.resolve(42), "test op");
    expect(result.ok).toBe(true);
    if (result.ok) expect(result.value).toBe(42);
  });

  test("returns err(UserError) with stderr on failure", async () => {
    const shellError = Object.assign(new Error("Failed with exit code 1"), {
      stderr: new TextEncoder().encode("No such file or directory"),
      exitCode: 1,
    });
    const result = await wrapShell(
      () => Promise.reject(shellError),
      "Failed to copy skill",
      "Check permissions.",
    );
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.error.message).toContain("Failed to copy skill");
      expect(result.error.message).toContain("No such file or directory");
      expect(result.error.hint).toBe("Check permissions.");
    }
  });

  test("falls back to exit code when no stderr", async () => {
    const shellError = Object.assign(new Error("Failed with exit code 18"), {
      stderr: new Uint8Array(0),
      exitCode: 18,
    });
    const result = await wrapShell(
      () => Promise.reject(shellError),
      "mv failed",
    );
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.error.message).toContain("exit code 18");
    }
  });
});
