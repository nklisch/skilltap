import { describe, expect, test } from "bun:test"
import { AgentResponseSchema, ResolvedSourceSchema } from "./agent"

describe("AgentResponseSchema", () => {
  test("accepts valid response", () => {
    const result = AgentResponseSchema.safeParse({ score: 3, reason: "Low risk content" })
    expect(result.success).toBe(true)
    if (result.success) {
      expect(result.data.score).toBe(3)
      expect(result.data.reason).toBe("Low risk content")
    }
  })

  test("accepts score 0 (minimum)", () => {
    expect(AgentResponseSchema.safeParse({ score: 0, reason: "Safe" }).success).toBe(true)
  })

  test("accepts score 10 (maximum)", () => {
    expect(AgentResponseSchema.safeParse({ score: 10, reason: "Dangerous" }).success).toBe(true)
  })

  test("rejects score below 0", () => {
    expect(AgentResponseSchema.safeParse({ score: -1, reason: "x" }).success).toBe(false)
  })

  test("rejects score above 10", () => {
    expect(AgentResponseSchema.safeParse({ score: 11, reason: "x" }).success).toBe(false)
  })

  test("rejects non-integer score", () => {
    expect(AgentResponseSchema.safeParse({ score: 5.5, reason: "x" }).success).toBe(false)
  })

  test("rejects missing score", () => {
    expect(AgentResponseSchema.safeParse({ reason: "x" }).success).toBe(false)
  })

  test("rejects missing reason", () => {
    expect(AgentResponseSchema.safeParse({ score: 5 }).success).toBe(false)
  })

  test("accepts empty reason string", () => {
    expect(AgentResponseSchema.safeParse({ score: 0, reason: "" }).success).toBe(true)
  })
})

describe("ResolvedSourceSchema", () => {
  test("accepts with optional ref", () => {
    const result = ResolvedSourceSchema.safeParse({
      url: "https://github.com/user/repo.git",
      ref: "v1.0.0",
      adapter: "github",
    })
    expect(result.success).toBe(true)
    if (result.success) {
      expect(result.data.url).toBe("https://github.com/user/repo.git")
      expect(result.data.ref).toBe("v1.0.0")
      expect(result.data.adapter).toBe("github")
    }
  })

  test("accepts without ref (optional)", () => {
    const result = ResolvedSourceSchema.safeParse({
      url: "https://github.com/user/repo.git",
      adapter: "git",
    })
    expect(result.success).toBe(true)
    if (result.success) expect(result.data.ref).toBeUndefined()
  })

  test("rejects missing url", () => {
    expect(ResolvedSourceSchema.safeParse({ adapter: "git" }).success).toBe(false)
  })

  test("rejects missing adapter", () => {
    expect(
      ResolvedSourceSchema.safeParse({ url: "https://example.com/repo.git" }).success
    ).toBe(false)
  })
})
