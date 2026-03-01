import { describe, expect, test } from "bun:test"
import { InstalledSkillSchema, InstalledJsonSchema } from "./installed"

const VALID_SKILL = {
  name: "commit-helper",
  repo: "https://gitea.example.com/nathan/commit-helper",
  ref: "v1.2.0",
  sha: "abc123def456",
  scope: "global" as const,
  path: null,
  tap: "home",
  also: ["claude-code"],
  installedAt: "2026-02-28T12:00:00.000Z",
  updatedAt: "2026-02-28T12:00:00.000Z",
}

describe("InstalledSkillSchema", () => {
  test("accepts a fully populated skill", () => {
    const result = InstalledSkillSchema.safeParse(VALID_SKILL)
    expect(result.success).toBe(true)
    if (result.success) {
      expect(result.data.name).toBe("commit-helper")
      expect(result.data.also).toEqual(["claude-code"])
    }
  })

  test("accepts linked skill with all nulls", () => {
    const linked = {
      ...VALID_SKILL,
      repo: null,
      ref: null,
      sha: null,
      path: null,
      tap: null,
      scope: "linked" as const,
    }
    const result = InstalledSkillSchema.safeParse(linked)
    expect(result.success).toBe(true)
  })

  test("accepts all scope values", () => {
    for (const scope of ["global", "project", "linked"] as const) {
      expect(InstalledSkillSchema.safeParse({ ...VALID_SKILL, scope }).success).toBe(true)
    }
  })

  test("rejects invalid scope", () => {
    expect(InstalledSkillSchema.safeParse({ ...VALID_SKILL, scope: "local" }).success).toBe(false)
  })

  test("accepts empty also array", () => {
    const result = InstalledSkillSchema.safeParse({ ...VALID_SKILL, also: [] })
    expect(result.success).toBe(true)
    if (result.success) expect(result.data.also).toEqual([])
  })

  test("rejects missing required fields", () => {
    const { name: _, ...withoutName } = VALID_SKILL
    expect(InstalledSkillSchema.safeParse(withoutName).success).toBe(false)
  })

  test("rejects non-datetime installedAt", () => {
    expect(
      InstalledSkillSchema.safeParse({ ...VALID_SKILL, installedAt: "not-a-date" }).success
    ).toBe(false)
  })

  test("accepts multi-skill path", () => {
    const result = InstalledSkillSchema.safeParse({
      ...VALID_SKILL,
      path: ".agents/skills/commit-helper",
    })
    expect(result.success).toBe(true)
    if (result.success) expect(result.data.path).toBe(".agents/skills/commit-helper")
  })
})

describe("InstalledJsonSchema", () => {
  test("accepts empty skills list", () => {
    const result = InstalledJsonSchema.safeParse({ version: 1, skills: [] })
    expect(result.success).toBe(true)
    if (result.success) {
      expect(result.data.version).toBe(1)
      expect(result.data.skills).toEqual([])
    }
  })

  test("accepts populated skills list", () => {
    const result = InstalledJsonSchema.safeParse({
      version: 1,
      skills: [VALID_SKILL],
    })
    expect(result.success).toBe(true)
    if (result.success) expect(result.data.skills).toHaveLength(1)
  })

  test("rejects wrong version", () => {
    expect(InstalledJsonSchema.safeParse({ version: 2, skills: [] }).success).toBe(false)
    expect(InstalledJsonSchema.safeParse({ version: 0, skills: [] }).success).toBe(false)
  })

  test("rejects missing version", () => {
    expect(InstalledJsonSchema.safeParse({ skills: [] }).success).toBe(false)
  })

  test("rejects invalid skill in array", () => {
    expect(
      InstalledJsonSchema.safeParse({
        version: 1,
        skills: [{ ...VALID_SKILL, scope: "bad" }],
      }).success
    ).toBe(false)
  })
})
