import { describe, expect, test } from "bun:test";
import {
  LockEntrySchema,
  LockfileSchema,
  ManifestEntryDetailSchema,
  ManifestEntrySchema,
  ProjectManifestSchema,
  TargetsSchema,
} from "./schemas";

describe("TargetsSchema", () => {
  test("applies nested defaults from {}", () => {
    const result = TargetsSchema.safeParse({});
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.also).toEqual([]);
      expect(result.data.scope).toBe("");
    }
  });

  test("applies nested defaults when omitted entirely", () => {
    const result = TargetsSchema.safeParse(undefined);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.also).toEqual([]);
      expect(result.data.scope).toBe("");
    }
  });

  test("accepts populated values", () => {
    const result = TargetsSchema.safeParse({
      also: ["claude-code", "cursor"],
      scope: "project",
    });
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.also).toEqual(["claude-code", "cursor"]);
      expect(result.data.scope).toBe("project");
    }
  });

  test("rejects invalid scope value", () => {
    const result = TargetsSchema.safeParse({ scope: "linked" });
    expect(result.success).toBe(false);
  });
});

describe("ManifestEntrySchema", () => {
  test("accepts a string range", () => {
    expect(ManifestEntrySchema.safeParse("^1.0").success).toBe(true);
    expect(ManifestEntrySchema.safeParse("*").success).toBe(true);
    expect(ManifestEntrySchema.safeParse("v1.2.3").success).toBe(true);
  });

  test("accepts an inline-table form with ref", () => {
    const result = ManifestEntrySchema.safeParse({ ref: "main" });
    expect(result.success).toBe(true);
  });

  test("accepts an inline-table form with components", () => {
    const result = ManifestEntrySchema.safeParse({
      ref: "v1.0",
      components: { "test-skipper": false, "code-review": true },
    });
    expect(result.success).toBe(true);
  });

  test("accepts empty inline table (everything optional)", () => {
    const result = ManifestEntryDetailSchema.safeParse({});
    expect(result.success).toBe(true);
  });

  test("rejects non-boolean component values", () => {
    const result = ManifestEntrySchema.safeParse({
      components: { foo: "yes" },
    });
    expect(result.success).toBe(false);
  });
});

describe("ProjectManifestSchema", () => {
  test("accepts an empty manifest with all defaults", () => {
    const result = ProjectManifestSchema.safeParse({});
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.skills).toEqual({});
      expect(result.data.plugins).toEqual({});
      expect(result.data.taps).toEqual({});
      expect(result.data.targets.also).toEqual([]);
    }
  });

  test("accepts a fully-populated manifest", () => {
    const result = ProjectManifestSchema.safeParse({
      targets: { also: ["claude-code"], scope: "project" },
      skills: {
        "github:nathan/commit-helper": "^1.0",
        "npm:@corp/code-review": "*",
      },
      plugins: {
        "github:corp/dev-toolkit": { ref: "v2.1", components: { "test-skipper": false } },
      },
      taps: {
        home: "https://gitea.example.com/nathan/my-tap",
      },
    });
    expect(result.success).toBe(true);
    if (result.success) {
      expect(Object.keys(result.data.skills)).toHaveLength(2);
      expect(Object.keys(result.data.plugins)).toHaveLength(1);
      expect(result.data.taps["home"]).toBe(
        "https://gitea.example.com/nathan/my-tap",
      );
    }
  });

  test("rejects non-string tap URL", () => {
    const result = ProjectManifestSchema.safeParse({
      taps: { home: 42 },
    });
    expect(result.success).toBe(false);
  });
});

describe("LockEntrySchema / LockfileSchema", () => {
  const VALID_ENTRY = {
    source: "github:nathan/commit-helper",
    ref: "v1.2.0",
    sha: "abc123def456",
    range: "^1.0",
  };

  test("accepts a fully-populated lock entry", () => {
    const result = LockEntrySchema.safeParse(VALID_ENTRY);
    expect(result.success).toBe(true);
  });

  test("accepts a lock entry without sha", () => {
    const { sha: _, ...rest } = VALID_ENTRY;
    const result = LockEntrySchema.safeParse(rest);
    expect(result.success).toBe(true);
  });

  test("rejects a lock entry without source", () => {
    const { source: _, ...rest } = VALID_ENTRY;
    const result = LockEntrySchema.safeParse(rest);
    expect(result.success).toBe(false);
  });

  test("rejects a lock entry without ref", () => {
    const { ref: _, ...rest } = VALID_ENTRY;
    const result = LockEntrySchema.safeParse(rest);
    expect(result.success).toBe(false);
  });

  test("accepts an empty lockfile", () => {
    const result = LockfileSchema.safeParse({ version: 1 });
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.skill).toEqual([]);
      expect(result.data.plugin).toEqual([]);
    }
  });

  test("accepts a populated lockfile", () => {
    const result = LockfileSchema.safeParse({
      version: 1,
      skill: [VALID_ENTRY],
      plugin: [{ ...VALID_ENTRY, source: "github:corp/dev-toolkit" }],
    });
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.skill).toHaveLength(1);
      expect(result.data.plugin).toHaveLength(1);
    }
  });

  test("rejects wrong lockfile version", () => {
    expect(LockfileSchema.safeParse({ version: 2 }).success).toBe(false);
    expect(LockfileSchema.safeParse({ version: 0 }).success).toBe(false);
  });
});
