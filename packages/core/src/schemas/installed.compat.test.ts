import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import { loadSkillState } from "@skilltap/core";
import { makeTmpDir, removeTmpDir } from "@skilltap/test-utils";
import { LegacyInstalledJsonSchema as InstalledJsonSchema } from "../migrate/legacy-schemas";

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

// Write fixture skills into state.json (v2 format) so loadSkillState can read them.
// The fixtures test that old-format skills (missing optional fields) are still parseable.
async function writeStateFromFixture(fixture: string): Promise<void> {
  const dir = join(configDir, "skilltap");
  await mkdir(dir, { recursive: true });
  const content = await Bun.file(join(FIXTURES_DIR, fixture)).text();
  const raw = JSON.parse(content);
  // Parse through InstalledJsonSchema to apply defaults (as migrate would do)
  const parsed = InstalledJsonSchema.safeParse(raw);
  if (!parsed.success) throw new Error(`Fixture parse failed: ${parsed.error}`);
  const state = {
    version: 2,
    skills: parsed.data.skills,
    plugins: [],
    mcpServers: [],
  };
  await Bun.write(join(dir, "state.json"), JSON.stringify(state));
}

describe("installed.json backward compatibility", () => {
  test("skill missing description loads with empty string default", async () => {
    await writeStateFromFixture("installed-no-description.json");
    const result = await loadSkillState();
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value).toHaveLength(1);
    expect(result.value[0]?.description).toBe("");
  });

  test("skill missing sha and updatedAt loads with null/sentinel defaults", async () => {
    await writeStateFromFixture("installed-no-sha.json");
    const result = await loadSkillState();
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value[0]?.sha).toBeNull();
    expect(result.value[0]?.updatedAt).toBe("1970-01-01T00:00:00.000Z");
  });

  test("skill missing also and tap loads with empty array and null defaults", async () => {
    await writeStateFromFixture("installed-no-also.json");
    const result = await loadSkillState();
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value[0]?.also).toEqual([]);
    expect(result.value[0]?.tap).toBeNull();
  });
});
