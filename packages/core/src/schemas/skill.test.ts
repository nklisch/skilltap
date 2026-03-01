import { describe, expect, test } from "bun:test";
import { SkillFrontmatterSchema } from "./skill";

const VALID = {
  name: "commit-helper",
  description: "Generates conventional commit messages for git.",
};

describe("SkillFrontmatterSchema", () => {
  test("accepts minimal valid frontmatter", () => {
    const result = SkillFrontmatterSchema.safeParse(VALID);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.name).toBe("commit-helper");
      expect(result.data.license).toBeUndefined();
    }
  });

  test("accepts all optional fields", () => {
    const result = SkillFrontmatterSchema.safeParse({
      ...VALID,
      license: "MIT",
      compatibility: "Requires Python 3.8+",
      metadata: { author: "nathan", version: "1.0" },
    });
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.license).toBe("MIT");
      expect(result.data.compatibility).toBe("Requires Python 3.8+");
      expect(result.data.metadata).toEqual({
        author: "nathan",
        version: "1.0",
      });
    }
  });

  describe("name validation", () => {
    test("accepts lowercase alphanumeric", () => {
      expect(
        SkillFrontmatterSchema.safeParse({ ...VALID, name: "abc123" }).success,
      ).toBe(true);
    });

    test("accepts hyphen-separated segments", () => {
      expect(
        SkillFrontmatterSchema.safeParse({ ...VALID, name: "my-skill" })
          .success,
      ).toBe(true);
      expect(
        SkillFrontmatterSchema.safeParse({ ...VALID, name: "a-b-c-d" }).success,
      ).toBe(true);
    });

    test("rejects uppercase letters", () => {
      expect(
        SkillFrontmatterSchema.safeParse({ ...VALID, name: "MySkill" }).success,
      ).toBe(false);
      expect(
        SkillFrontmatterSchema.safeParse({ ...VALID, name: "MY-SKILL" })
          .success,
      ).toBe(false);
    });

    test("rejects leading hyphen", () => {
      expect(
        SkillFrontmatterSchema.safeParse({ ...VALID, name: "-skill" }).success,
      ).toBe(false);
    });

    test("rejects trailing hyphen", () => {
      expect(
        SkillFrontmatterSchema.safeParse({ ...VALID, name: "skill-" }).success,
      ).toBe(false);
    });

    test("rejects consecutive hyphens", () => {
      expect(
        SkillFrontmatterSchema.safeParse({ ...VALID, name: "skill--name" })
          .success,
      ).toBe(false);
    });

    test("rejects underscores", () => {
      expect(
        SkillFrontmatterSchema.safeParse({ ...VALID, name: "my_skill" })
          .success,
      ).toBe(false);
    });

    test("rejects spaces", () => {
      expect(
        SkillFrontmatterSchema.safeParse({ ...VALID, name: "my skill" })
          .success,
      ).toBe(false);
    });

    test("rejects empty name", () => {
      expect(
        SkillFrontmatterSchema.safeParse({ ...VALID, name: "" }).success,
      ).toBe(false);
    });

    test("rejects name longer than 64 chars", () => {
      const long = "a".repeat(65);
      expect(
        SkillFrontmatterSchema.safeParse({ ...VALID, name: long }).success,
      ).toBe(false);
    });

    test("accepts name exactly 64 chars", () => {
      // 64 chars of valid pattern: alternating letter segments
      const _name = `a${"-a".repeat(31)}`; // "a-a-a-..." = 1 + 31*2 = 63 chars, need 64
      // Just use a 64-char lowercase alphanumeric string
      const exact64 = "a".repeat(64);
      expect(
        SkillFrontmatterSchema.safeParse({ ...VALID, name: exact64 }).success,
      ).toBe(true);
    });
  });

  describe("description validation", () => {
    test("rejects empty description", () => {
      expect(
        SkillFrontmatterSchema.safeParse({ ...VALID, description: "" }).success,
      ).toBe(false);
    });

    test("rejects description longer than 1024 chars", () => {
      const long = "x".repeat(1025);
      expect(
        SkillFrontmatterSchema.safeParse({ ...VALID, description: long })
          .success,
      ).toBe(false);
    });

    test("accepts description exactly 1024 chars", () => {
      const exact = "x".repeat(1024);
      expect(
        SkillFrontmatterSchema.safeParse({ ...VALID, description: exact })
          .success,
      ).toBe(true);
    });
  });

  describe("compatibility validation", () => {
    test("rejects compatibility longer than 500 chars", () => {
      const long = "x".repeat(501);
      expect(
        SkillFrontmatterSchema.safeParse({ ...VALID, compatibility: long })
          .success,
      ).toBe(false);
    });

    test("accepts compatibility exactly 500 chars", () => {
      const exact = "x".repeat(500);
      expect(
        SkillFrontmatterSchema.safeParse({ ...VALID, compatibility: exact })
          .success,
      ).toBe(true);
    });
  });

  test("metadata accepts arbitrary key-value pairs", () => {
    const result = SkillFrontmatterSchema.safeParse({
      ...VALID,
      metadata: {
        author: "alice",
        tags: ["git", "ci"],
        count: 42,
        nested: { x: true },
      },
    });
    expect(result.success).toBe(true);
  });

  test("rejects missing name", () => {
    const { name: _, ...withoutName } = VALID;
    expect(SkillFrontmatterSchema.safeParse(withoutName).success).toBe(false);
  });

  test("rejects missing description", () => {
    const { description: _, ...withoutDesc } = VALID;
    expect(SkillFrontmatterSchema.safeParse(withoutDesc).success).toBe(false);
  });
});
