import { describe, expect, test, beforeEach, afterEach } from "bun:test";
import { writeFile, mkdir, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { checkPluginManifests } from "./plugin-manifests";

let projectRoot: string;
beforeEach(async () => {
  projectRoot = await mkdtemp(join(tmpdir(), "skilltap-pm-test-"));
});
afterEach(async () => {
  await rm(projectRoot, { recursive: true, force: true });
});

describe("checkPluginManifests", () => {
  test("returns n/a when no projectRoot provided", async () => {
    const check = await checkPluginManifests(undefined);
    expect(check.status).toBe("pass");
    expect(check.detail).toBe("n/a (no project root)");
  });

  test("returns n/a when .skilltap/ dir is missing", async () => {
    const check = await checkPluginManifests(projectRoot);
    expect(check.status).toBe("pass");
    expect(check.detail).toBe("n/a (no .skilltap/ publish manifests)");
  });

  test("returns n/a when .skilltap/ exists but has no .toml files", async () => {
    await mkdir(join(projectRoot, ".skilltap"), { recursive: true });
    const check = await checkPluginManifests(projectRoot);
    expect(check.status).toBe("pass");
    expect(check.detail).toBe("n/a (no .skilltap/ publish manifests)");
  });

  test("passes with count when all manifests are valid and publish=true", async () => {
    const dir = join(projectRoot, ".skilltap");
    await mkdir(dir, { recursive: true });
    await writeFile(
      join(dir, "my-plugin.toml"),
      `name = "my-plugin"\nversion = "1.0.0"\npublish = true\n`,
    );
    const check = await checkPluginManifests(projectRoot);
    expect(check.status).toBe("pass");
    expect(check.detail).toBe("1 valid");
    expect(check.issues).toBeUndefined();
  });

  test("warns on invalid TOML but does not flag publish=false entries", async () => {
    const dir = join(projectRoot, ".skilltap");
    await mkdir(dir, { recursive: true });
    await writeFile(join(dir, "bad.toml"), `not = valid = toml`);
    await writeFile(
      join(dir, "private.toml"),
      `name = "private"\nversion = "0.1.0"\npublish = false\n`,
    );
    const check = await checkPluginManifests(projectRoot);
    expect(check.status).toBe("warn");
    expect(check.issues).toHaveLength(1);
    expect(check.issues![0].message).toContain("bad.toml");
    expect(check.issues![0].fixable).toBe(false);
  });

  test("warns on schema mismatch (uppercase name violates regex)", async () => {
    const dir = join(projectRoot, ".skilltap");
    await mkdir(dir, { recursive: true });
    await writeFile(
      join(dir, "invalid-schema.toml"),
      `name = "BadName"\nversion = "1.0.0"\npublish = true\n`,
    );
    const check = await checkPluginManifests(projectRoot);
    expect(check.status).toBe("warn");
    expect(check.issues).toHaveLength(1);
    expect(check.issues![0].message).toContain("invalid-schema.toml");
    expect(check.issues![0].message).toContain("Schema mismatch");
    expect(check.issues![0].fixable).toBe(false);
  });

  test("publish=false entries are intentional and never counted as issues", async () => {
    const dir = join(projectRoot, ".skilltap");
    await mkdir(dir, { recursive: true });
    await writeFile(
      join(dir, "internal.toml"),
      `name = "internal"\nversion = "0.1.0"\npublish = false\n`,
    );
    const check = await checkPluginManifests(projectRoot);
    expect(check.status).toBe("pass");
    expect(check.detail).toBe("n/a (no .skilltap/ publish manifests)");
    expect(check.issues).toBeUndefined();
  });

  test("reports valid count and invalid count together", async () => {
    const dir = join(projectRoot, ".skilltap");
    await mkdir(dir, { recursive: true });
    await writeFile(
      join(dir, "good.toml"),
      `name = "good-plugin"\nversion = "1.0.0"\npublish = true\n`,
    );
    await writeFile(join(dir, "broken.toml"), `totally : broken = yaml lookalike`);
    const check = await checkPluginManifests(projectRoot);
    expect(check.status).toBe("warn");
    expect(check.detail).toBe("1 valid, 1 invalid");
    expect(check.issues).toHaveLength(1);
  });
});
