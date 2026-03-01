import { describe, expect, test } from "bun:test"
import { TapSkillSchema, TapSchema } from "./tap"

const VALID_SKILL = {
  name: "commit-helper",
  description: "Generates conventional commit messages",
  repo: "https://gitea.example.com/nathan/commit-helper",
  tags: ["git", "productivity"],
}

describe("TapSkillSchema", () => {
  test("accepts a fully populated skill", () => {
    const result = TapSkillSchema.safeParse(VALID_SKILL)
    expect(result.success).toBe(true)
    if (result.success) {
      expect(result.data.name).toBe("commit-helper")
      expect(result.data.tags).toEqual(["git", "productivity"])
    }
  })

  test("defaults tags to empty array when omitted", () => {
    const { tags: _, ...withoutTags } = VALID_SKILL
    const result = TapSkillSchema.safeParse(withoutTags)
    expect(result.success).toBe(true)
    if (result.success) expect(result.data.tags).toEqual([])
  })

  test("rejects missing name", () => {
    const { name: _, ...withoutName } = VALID_SKILL
    expect(TapSkillSchema.safeParse(withoutName).success).toBe(false)
  })

  test("rejects missing description", () => {
    const { description: _, ...withoutDesc } = VALID_SKILL
    expect(TapSkillSchema.safeParse(withoutDesc).success).toBe(false)
  })

  test("rejects missing repo", () => {
    const { repo: _, ...withoutRepo } = VALID_SKILL
    expect(TapSkillSchema.safeParse(withoutRepo).success).toBe(false)
  })
})

describe("TapSchema", () => {
  test("accepts full tap with description", () => {
    const result = TapSchema.safeParse({
      name: "nathan's skills",
      description: "My curated skill collection",
      skills: [VALID_SKILL],
    })
    expect(result.success).toBe(true)
    if (result.success) {
      expect(result.data.name).toBe("nathan's skills")
      expect(result.data.description).toBe("My curated skill collection")
      expect(result.data.skills).toHaveLength(1)
    }
  })

  test("accepts tap without description (optional)", () => {
    const result = TapSchema.safeParse({
      name: "my-tap",
      skills: [],
    })
    expect(result.success).toBe(true)
    if (result.success) expect(result.data.description).toBeUndefined()
  })

  test("accepts empty skills array", () => {
    const result = TapSchema.safeParse({ name: "empty-tap", skills: [] })
    expect(result.success).toBe(true)
    if (result.success) expect(result.data.skills).toEqual([])
  })

  test("accepts multiple skills", () => {
    const result = TapSchema.safeParse({
      name: "multi-tap",
      skills: [
        VALID_SKILL,
        { ...VALID_SKILL, name: "other-skill", tags: [] },
      ],
    })
    expect(result.success).toBe(true)
    if (result.success) expect(result.data.skills).toHaveLength(2)
  })

  test("rejects missing name", () => {
    expect(TapSchema.safeParse({ skills: [] }).success).toBe(false)
  })

  test("rejects missing skills array", () => {
    expect(TapSchema.safeParse({ name: "tap" }).success).toBe(false)
  })

  test("propagates skill validation errors", () => {
    expect(
      TapSchema.safeParse({
        name: "tap",
        skills: [{ ...VALID_SKILL, repo: 123 }],
      }).success
    ).toBe(false)
  })
})
