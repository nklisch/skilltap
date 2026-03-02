import { describe, expect, test } from "bun:test";
import { npmAdapter } from "./npm";

describe("npmAdapter.canHandle", () => {
  test("accepts npm: prefix", () => {
    expect(npmAdapter.canHandle("npm:some-package")).toBe(true);
  });

  test("accepts scoped package", () => {
    expect(npmAdapter.canHandle("npm:@scope/pkg")).toBe(true);
  });

  test("accepts package with version", () => {
    expect(npmAdapter.canHandle("npm:pkg@1.2.0")).toBe(true);
  });

  test("accepts scoped package with version", () => {
    expect(npmAdapter.canHandle("npm:@scope/pkg@1.0.0")).toBe(true);
  });

  test("rejects https:// URLs", () => {
    expect(npmAdapter.canHandle("https://registry.npmjs.org/pkg")).toBe(false);
  });

  test("rejects github: prefix", () => {
    expect(npmAdapter.canHandle("github:user/repo")).toBe(false);
  });

  test("rejects bare names", () => {
    expect(npmAdapter.canHandle("some-package")).toBe(false);
  });

  test("rejects local paths", () => {
    expect(npmAdapter.canHandle("./local")).toBe(false);
    expect(npmAdapter.canHandle("/abs/path")).toBe(false);
  });
});
