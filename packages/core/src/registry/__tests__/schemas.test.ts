import { describe, expect, test } from "bun:test";
import {
  RegistryDetailResponseSchema,
  RegistryListResponseSchema,
  RegistrySkillSchema,
  RegistrySourceSchema,
} from "../types";

describe("RegistrySourceSchema", () => {
  test("accepts git source", () => {
    const r = RegistrySourceSchema.safeParse({ type: "git", url: "https://github.com/owner/repo" });
    expect(r.success).toBe(true);
    if (!r.success) return;
    expect(r.data.type).toBe("git");
    if (r.data.type === "git") expect(r.data.url).toBe("https://github.com/owner/repo");
  });

  test("accepts git source with optional ref", () => {
    const r = RegistrySourceSchema.safeParse({
      type: "git",
      url: "https://github.com/owner/repo",
      ref: "v1.0.0",
    });
    expect(r.success).toBe(true);
  });

  test("accepts github source", () => {
    const r = RegistrySourceSchema.safeParse({ type: "github", repo: "owner/repo" });
    expect(r.success).toBe(true);
    if (!r.success) return;
    if (r.data.type === "github") expect(r.data.repo).toBe("owner/repo");
  });

  test("accepts npm source", () => {
    const r = RegistrySourceSchema.safeParse({ type: "npm", package: "@scope/name" });
    expect(r.success).toBe(true);
    if (!r.success) return;
    if (r.data.type === "npm") expect(r.data.package).toBe("@scope/name");
  });

  test("accepts npm source with version", () => {
    const r = RegistrySourceSchema.safeParse({
      type: "npm",
      package: "@scope/name",
      version: "2.0.0",
    });
    expect(r.success).toBe(true);
  });

  test("accepts url source", () => {
    const r = RegistrySourceSchema.safeParse({
      type: "url",
      url: "https://registry.example.com/skills/commit-helper.tar.gz",
    });
    expect(r.success).toBe(true);
    if (!r.success) return;
    if (r.data.type === "url")
      expect(r.data.url).toBe("https://registry.example.com/skills/commit-helper.tar.gz");
  });

  test("rejects unknown type", () => {
    const r = RegistrySourceSchema.safeParse({ type: "unknown", url: "https://example.com" });
    expect(r.success).toBe(false);
  });

  test("rejects missing type", () => {
    const r = RegistrySourceSchema.safeParse({ url: "https://example.com" });
    expect(r.success).toBe(false);
  });
});

describe("RegistrySkillSchema", () => {
  const VALID_SKILL = {
    name: "commit-helper",
    description: "Generates conventional commit messages",
    source: { type: "git", url: "https://github.com/owner/commit-helper" },
  };

  test("accepts minimal skill", () => {
    const r = RegistrySkillSchema.safeParse(VALID_SKILL);
    expect(r.success).toBe(true);
    if (!r.success) return;
    expect(r.data.tags).toEqual([]);
  });

  test("accepts full skill", () => {
    const r = RegistrySkillSchema.safeParse({
      ...VALID_SKILL,
      version: "1.2.0",
      author: "nathan",
      tags: ["git", "productivity"],
      trust: { verified: true, verifiedBy: "nathan" },
    });
    expect(r.success).toBe(true);
    if (!r.success) return;
    expect(r.data.tags).toEqual(["git", "productivity"]);
    expect(r.data.trust?.verified).toBe(true);
  });

  test("rejects missing name", () => {
    const r = RegistrySkillSchema.safeParse({ ...VALID_SKILL, name: undefined });
    expect(r.success).toBe(false);
  });

  test("rejects missing description", () => {
    const r = RegistrySkillSchema.safeParse({ ...VALID_SKILL, description: undefined });
    expect(r.success).toBe(false);
  });

  test("defaults trust.verified to false", () => {
    const r = RegistrySkillSchema.safeParse({
      ...VALID_SKILL,
      trust: {},
    });
    expect(r.success).toBe(true);
    if (!r.success) return;
    expect(r.data.trust?.verified).toBe(false);
  });
});

describe("RegistryListResponseSchema", () => {
  const VALID_LIST = {
    skills: [
      {
        name: "commit-helper",
        description: "Commit messages",
        source: { type: "git", url: "https://github.com/owner/commit-helper" },
      },
    ],
  };

  test("accepts minimal list response", () => {
    const r = RegistryListResponseSchema.safeParse(VALID_LIST);
    expect(r.success).toBe(true);
    if (!r.success) return;
    expect(r.data.skills).toHaveLength(1);
    expect(r.data.total).toBeUndefined();
    expect(r.data.cursor).toBeUndefined();
  });

  test("accepts list response with pagination", () => {
    const r = RegistryListResponseSchema.safeParse({
      ...VALID_LIST,
      total: 42,
      cursor: "eyJvZmZzZXQiOjUwfQ==",
    });
    expect(r.success).toBe(true);
    if (!r.success) return;
    expect(r.data.total).toBe(42);
    expect(r.data.cursor).toBe("eyJvZmZzZXQiOjUwfQ==");
  });

  test("accepts empty skills array", () => {
    const r = RegistryListResponseSchema.safeParse({ skills: [] });
    expect(r.success).toBe(true);
  });

  test("rejects missing skills array", () => {
    const r = RegistryListResponseSchema.safeParse({ total: 0 });
    expect(r.success).toBe(false);
  });
});

describe("RegistryDetailResponseSchema", () => {
  const VALID_DETAIL = {
    name: "commit-helper",
    description: "Generates conventional commit messages",
    source: { type: "git", url: "https://github.com/owner/commit-helper" },
  };

  test("accepts minimal detail response", () => {
    const r = RegistryDetailResponseSchema.safeParse(VALID_DETAIL);
    expect(r.success).toBe(true);
    if (!r.success) return;
    expect(r.data.versions).toEqual([]);
    expect(r.data.tags).toEqual([]);
  });

  test("accepts full detail response", () => {
    const r = RegistryDetailResponseSchema.safeParse({
      ...VALID_DETAIL,
      author: "nathan",
      license: "MIT",
      tags: ["git"],
      versions: [
        { version: "1.2.0", publishedAt: "2026-02-28T12:00:00Z" },
        { version: "1.1.0" },
      ],
      trust: { verified: true },
    });
    expect(r.success).toBe(true);
    if (!r.success) return;
    expect(r.data.versions).toHaveLength(2);
    expect(r.data.trust?.verified).toBe(true);
  });
});
