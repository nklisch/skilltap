import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { createTestEnv, type TestEnv } from "@skilltap/test-utils";
import { loadSkillState, saveSkillState } from "../config";
import type { InstalledSkill } from "../schemas/installed";
import { applySkillStateChange } from "./apply";

function makeSkill(name: string, overrides?: Partial<InstalledSkill>): InstalledSkill {
  return {
    name,
    description: `${name} description`,
    repo: `https://github.com/example/${name}`,
    ref: "main",
    sha: "abc123",
    scope: "global",
    path: null,
    tap: null,
    also: [],
    installedAt: "2024-01-01T00:00:00.000Z",
    updatedAt: "2024-01-01T00:00:00.000Z",
    ...overrides,
  };
}

describe("applySkillStateChange", () => {
  let env: TestEnv;

  beforeEach(async () => {
    env = await createTestEnv();
  });
  afterEach(async () => {
    await env.cleanup();
  });

  test("normal add — appends record to state", async () => {
    const existing = makeSkill("existing");
    await saveSkillState([existing]);

    const newSkill = makeSkill("new-skill");
    const result = await applySkillStateChange({
      scope: "global",
      mutate: (current) => [...current, newSkill],
    });

    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.map((r) => r.name)).toEqual(["existing", "new-skill"]);

    const reloaded = await loadSkillState();
    expect(reloaded.ok).toBe(true);
    if (!reloaded.ok) return;
    expect(reloaded.value.map((r) => r.name)).toEqual(["existing", "new-skill"]);
  });

  test("normal remove — filters record from state", async () => {
    const a = makeSkill("skill-a");
    const b = makeSkill("skill-b");
    await saveSkillState([a, b]);

    const result = await applySkillStateChange({
      scope: "global",
      mutate: (current) => current.filter((r) => r.name !== "skill-a"),
    });

    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.map((r) => r.name)).toEqual(["skill-b"]);

    const reloaded = await loadSkillState();
    expect(reloaded.ok).toBe(true);
    if (!reloaded.ok) return;
    expect(reloaded.value.map((r) => r.name)).toEqual(["skill-b"]);
  });

  test("mutate returns null — aborts, state unchanged, returns before array", async () => {
    const skill = makeSkill("unchanged");
    await saveSkillState([skill]);

    const result = await applySkillStateChange({
      scope: "global",
      mutate: () => null,
    });

    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.map((r) => r.name)).toEqual(["unchanged"]);

    // State should be untouched
    const reloaded = await loadSkillState();
    expect(reloaded.ok).toBe(true);
    if (!reloaded.ok) return;
    expect(reloaded.value.map((r) => r.name)).toEqual(["unchanged"]);
  });

  test("manifestSync not called when projectRoot is undefined", async () => {
    const newSkill = makeSkill("my-skill");
    let addedCalled = false;

    const result = await applySkillStateChange({
      scope: "global",
      mutate: (current) => [...current, newSkill],
      manifestSync: {
        onAdded: async () => {
          addedCalled = true;
        },
      },
      // projectRoot intentionally omitted
    });

    expect(result.ok).toBe(true);
    expect(addedCalled).toBe(false);
  });

  test("manifestSync.onAdded called for new records when projectRoot provided", async () => {
    const newSkill = makeSkill("added-skill");
    const addedNames: string[] = [];

    const result = await applySkillStateChange({
      scope: "global",
      projectRoot: "/fake/project",
      mutate: (current) => [...current, newSkill],
      manifestSync: {
        onAdded: async (record) => {
          addedNames.push(record.name);
        },
      },
    });

    expect(result.ok).toBe(true);
    expect(addedNames).toEqual(["added-skill"]);
  });

  test("manifestSync.onRemoved called for removed records when projectRoot provided", async () => {
    const skill = makeSkill("to-remove");
    await saveSkillState([skill]);

    const removedNames: string[] = [];

    const result = await applySkillStateChange({
      scope: "global",
      projectRoot: "/fake/project",
      mutate: (current) => current.filter((r) => r.name !== "to-remove"),
      manifestSync: {
        onRemoved: async (record) => {
          removedNames.push(record.name);
        },
      },
    });

    expect(result.ok).toBe(true);
    expect(removedNames).toEqual(["to-remove"]);
  });
});
