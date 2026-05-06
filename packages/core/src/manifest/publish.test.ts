import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { discoverPublishablePlugins } from "./publish";

let repoRoot: string;
beforeEach(async () => {
  repoRoot = await mkdtemp(join(tmpdir(), "skilltap-pub-"));
});
afterEach(async () => {
  await rm(repoRoot, { recursive: true, force: true });
});

describe("discoverPublishablePlugins", () => {
  test("returns empty when .skilltap/ doesn't exist", async () => {
    const result = await discoverPublishablePlugins(repoRoot);
    expect(result.publishable).toEqual([]);
    expect(result.rejected).toEqual([]);
  });

  test("returns empty when .skilltap/ exists but has no .toml files", async () => {
    await mkdir(join(repoRoot, ".skilltap"), { recursive: true });
    const result = await discoverPublishablePlugins(repoRoot);
    expect(result.publishable).toEqual([]);
    expect(result.rejected).toEqual([]);
  });

  test("returns publishable manifests with publish=true", async () => {
    const dir = join(repoRoot, ".skilltap");
    await mkdir(dir, { recursive: true });
    await writeFile(
      join(dir, "team-tools.toml"),
      `
name = "team-tools"
version = "1.0.0"
description = "Internal dev tools"
publish = true

[[skills]]
name = "code-review"
path = "./skills/code-review"
`,
    );

    const result = await discoverPublishablePlugins(repoRoot);
    expect(result.publishable).toHaveLength(1);
    expect(result.publishable[0].name).toBe("team-tools");
    expect(result.publishable[0].publish).toBe(true);
    expect(result.publishable[0].skills).toHaveLength(1);
    expect(result.rejected).toEqual([]);
  });

  test("rejects manifests with publish=false (or omitted)", async () => {
    const dir = join(repoRoot, ".skilltap");
    await mkdir(dir, { recursive: true });
    await writeFile(
      join(dir, "private-stuff.toml"),
      `
name = "private-stuff"
version = "0.1.0"
publish = false
`,
    );
    await writeFile(
      join(dir, "missing-publish.toml"),
      `
name = "missing-publish"
version = "0.1.0"
`,
    );

    const result = await discoverPublishablePlugins(repoRoot);
    expect(result.publishable).toEqual([]);
    expect(result.rejected).toHaveLength(2);
    for (const r of result.rejected) {
      expect(r.reason).toContain("publish = false");
    }
  });

  test("rejects invalid TOML and schema mismatches", async () => {
    const dir = join(repoRoot, ".skilltap");
    await mkdir(dir, { recursive: true });
    await writeFile(join(dir, "bad-toml.toml"), `not = valid = toml`);
    await writeFile(
      join(dir, "schema-mismatch.toml"),
      `
name = "Bad Name With Caps"
version = "1.0.0"
publish = true
`,
    );

    const result = await discoverPublishablePlugins(repoRoot);
    expect(result.publishable).toEqual([]);
    expect(result.rejected).toHaveLength(2);
    expect(result.rejected.some((r) => r.reason.includes("Invalid TOML"))).toBe(
      true,
    );
    expect(
      result.rejected.some((r) => r.reason.includes("Schema mismatch")),
    ).toBe(true);
  });

  test("handles multiple plugins (mix of publishable + rejected)", async () => {
    const dir = join(repoRoot, ".skilltap");
    await mkdir(dir, { recursive: true });
    await writeFile(
      join(dir, "publish-a.toml"),
      `name = "publish-a"\nversion = "1.0.0"\npublish = true\n`,
    );
    await writeFile(
      join(dir, "publish-b.toml"),
      `name = "publish-b"\nversion = "0.1.0"\npublish = true\n`,
    );
    await writeFile(
      join(dir, "internal.toml"),
      `name = "internal"\nversion = "0.1.0"\npublish = false\n`,
    );

    const result = await discoverPublishablePlugins(repoRoot);
    expect(result.publishable).toHaveLength(2);
    expect(result.rejected).toHaveLength(1);
    expect(result.publishable.map((p) => p.name).sort()).toEqual([
      "publish-a",
      "publish-b",
    ]);
  });
});
