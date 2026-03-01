import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { loadInstalled } from "@skilltap/core";
import { makeTmpDir, removeTmpDir } from "@skilltap/test-utils";

const FIXTURES_DIR = join(
  import.meta.dir,
  "../../../test-utils/fixtures/compat",
);

let configDir: string;

beforeEach(async () => {
  configDir = await makeTmpDir();
  process.env.XDG_CONFIG_HOME = configDir;
});

afterEach(async () => {
  delete process.env.XDG_CONFIG_HOME;
  await removeTmpDir(configDir);
});

async function writeInstalledJson(fixture: string): Promise<void> {
  const dir = join(configDir, "skilltap");
  await mkdir(dir, { recursive: true });
  const content = await Bun.file(join(FIXTURES_DIR, fixture)).text();
  await Bun.write(join(dir, "installed.json"), content);
}

describe("installed.json backward compatibility", () => {
  test("skill missing description loads with empty string default", async () => {
    await writeInstalledJson("installed-no-description.json");
    const result = await loadInstalled();
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.skills).toHaveLength(1);
    expect(result.value.skills[0]?.description).toBe("");
  });

  test("skill missing sha and updatedAt loads with null/sentinel defaults", async () => {
    await writeInstalledJson("installed-no-sha.json");
    const result = await loadInstalled();
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.skills[0]?.sha).toBeNull();
    expect(result.value.skills[0]?.updatedAt).toBe("1970-01-01T00:00:00.000Z");
  });

  test("skill missing also and tap loads with empty array and null defaults", async () => {
    await writeInstalledJson("installed-no-also.json");
    const result = await loadInstalled();
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.skills[0]?.also).toEqual([]);
    expect(result.value.skills[0]?.tap).toBeNull();
  });
});
