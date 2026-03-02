import { describe, expect, test } from "bun:test";
import {
  type NpmPackageMetadata,
  parseNpmSource,
  resolveVersion,
} from "./npm-registry";

describe("parseNpmSource", () => {
  test("bare package name → latest", () => {
    const r = parseNpmSource("npm:some-package");
    expect(r.name).toBe("some-package");
    expect(r.version).toBe("latest");
  });

  test("package with exact version", () => {
    const r = parseNpmSource("npm:some-package@1.2.0");
    expect(r.name).toBe("some-package");
    expect(r.version).toBe("1.2.0");
  });

  test("scoped package → latest", () => {
    const r = parseNpmSource("npm:@scope/pkg");
    expect(r.name).toBe("@scope/pkg");
    expect(r.version).toBe("latest");
  });

  test("scoped package with version", () => {
    const r = parseNpmSource("npm:@scope/pkg@2.0.0");
    expect(r.name).toBe("@scope/pkg");
    expect(r.version).toBe("2.0.0");
  });

  test("scoped package with prerelease version", () => {
    const r = parseNpmSource("npm:@scope/pkg@1.0.0-beta.1");
    expect(r.name).toBe("@scope/pkg");
    expect(r.version).toBe("1.0.0-beta.1");
  });

  test("works without npm: prefix", () => {
    const r = parseNpmSource("some-package@1.0.0");
    expect(r.name).toBe("some-package");
    expect(r.version).toBe("1.0.0");
  });

  test("package named 'latest' without version", () => {
    const r = parseNpmSource("npm:latest");
    expect(r.name).toBe("latest");
    expect(r.version).toBe("latest");
  });
});

describe("resolveVersion", () => {
  const meta: NpmPackageMetadata = {
    name: "@acme/pkg",
    description: "A test package",
    distTags: { latest: "1.2.0", beta: "2.0.0-beta.1" },
    versions: {
      "1.0.0": {
        version: "1.0.0",
        dist: { tarball: "https://example.com/pkg-1.0.0.tgz", integrity: "" },
      },
      "1.2.0": {
        version: "1.2.0",
        dist: { tarball: "https://example.com/pkg-1.2.0.tgz", integrity: "" },
      },
      "2.0.0-beta.1": {
        version: "2.0.0-beta.1",
        dist: {
          tarball: "https://example.com/pkg-2.0.0-beta.1.tgz",
          integrity: "",
        },
      },
    },
  };

  test("resolves 'latest' dist-tag", () => {
    const r = resolveVersion(meta, "latest");
    expect(r.ok).toBe(true);
    if (r.ok) {
      expect(r.value.version).toBe("1.2.0");
      expect(r.value.dist.tarball).toContain("1.2.0");
    }
  });

  test("resolves exact version", () => {
    const r = resolveVersion(meta, "1.0.0");
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.value.version).toBe("1.0.0");
  });

  test("resolves named dist-tag", () => {
    const r = resolveVersion(meta, "beta");
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.value.version).toBe("2.0.0-beta.1");
  });

  test("errors on nonexistent version", () => {
    const r = resolveVersion(meta, "9.9.9");
    expect(r.ok).toBe(false);
    if (!r.ok) {
      expect(r.error.message).toContain("9.9.9");
      expect(r.error.message).toContain("@acme/pkg");
      expect(r.error.hint).toContain("Available:");
    }
  });
});
