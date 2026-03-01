import { describe, expect, test } from "bun:test"
import { parse, stringify } from "smol-toml"
import { ConfigSchema, SecurityConfigSchema, AgentModeSchema } from "./config"

describe("SecurityConfigSchema", () => {
  test("applies all defaults", () => {
    const result = SecurityConfigSchema.parse({})
    expect(result.scan).toBe("static")
    expect(result.on_warn).toBe("prompt")
    expect(result.require_scan).toBe(false)
    expect(result.agent).toBe("")
    expect(result.threshold).toBe(5)
    expect(result.max_size).toBe(51200)
    expect(result.ollama_model).toBe("")
  })

  test("accepts all valid scan values", () => {
    expect(SecurityConfigSchema.parse({ scan: "static" }).scan).toBe("static")
    expect(SecurityConfigSchema.parse({ scan: "semantic" }).scan).toBe("semantic")
    expect(SecurityConfigSchema.parse({ scan: "off" }).scan).toBe("off")
  })

  test("rejects invalid scan value", () => {
    expect(SecurityConfigSchema.safeParse({ scan: "none" }).success).toBe(false)
    expect(SecurityConfigSchema.safeParse({ scan: "both" }).success).toBe(false)
  })

  test("accepts all valid on_warn values", () => {
    expect(SecurityConfigSchema.parse({ on_warn: "prompt" }).on_warn).toBe("prompt")
    expect(SecurityConfigSchema.parse({ on_warn: "fail" }).on_warn).toBe("fail")
  })

  test("rejects invalid on_warn value", () => {
    expect(SecurityConfigSchema.safeParse({ on_warn: "ignore" }).success).toBe(false)
  })

  test("threshold min and max bounds", () => {
    expect(SecurityConfigSchema.parse({ threshold: 0 }).threshold).toBe(0)
    expect(SecurityConfigSchema.parse({ threshold: 10 }).threshold).toBe(10)
    expect(SecurityConfigSchema.safeParse({ threshold: -1 }).success).toBe(false)
    expect(SecurityConfigSchema.safeParse({ threshold: 11 }).success).toBe(false)
  })

  test("threshold must be integer", () => {
    expect(SecurityConfigSchema.safeParse({ threshold: 5.5 }).success).toBe(false)
  })
})

describe("AgentModeSchema", () => {
  test("applies all defaults", () => {
    const result = AgentModeSchema.parse({})
    expect(result.enabled).toBe(false)
    expect(result.scope).toBe("project")
  })

  test("accepts all valid scope values", () => {
    expect(AgentModeSchema.parse({ scope: "global" }).scope).toBe("global")
    expect(AgentModeSchema.parse({ scope: "project" }).scope).toBe("project")
  })

  test("rejects invalid scope", () => {
    expect(AgentModeSchema.safeParse({ scope: "local" }).success).toBe(false)
  })
})

describe("ConfigSchema", () => {
  test("applies all defaults when empty", () => {
    const result = ConfigSchema.parse({})
    expect(result.defaults.also).toEqual([])
    expect(result.defaults.yes).toBe(false)
    expect(result.defaults.scope).toBe("")
    expect(result.security.scan).toBe("static")
    expect(result["agent-mode"].enabled).toBe(false)
    expect(result["agent-mode"].scope).toBe("project")
    expect(result.taps).toEqual([])
  })

  test("accepts full valid config", () => {
    const result = ConfigSchema.parse({
      defaults: { also: ["claude-code", "cursor"], yes: true, scope: "global" },
      security: { scan: "semantic", on_warn: "fail", threshold: 8 },
      "agent-mode": { enabled: true, scope: "project" },
      taps: [{ name: "home", url: "https://example.com/tap.git" }],
    })
    expect(result.defaults.also).toEqual(["claude-code", "cursor"])
    expect(result.defaults.yes).toBe(true)
    expect(result.defaults.scope).toBe("global")
    expect(result.security.scan).toBe("semantic")
    expect(result["agent-mode"].enabled).toBe(true)
    expect(result.taps[0].name).toBe("home")
  })

  test("defaults scope accepts empty string", () => {
    const result = ConfigSchema.parse({ defaults: { scope: "" } })
    expect(result.defaults.scope).toBe("")
  })

  test("rejects invalid defaults scope", () => {
    expect(
      ConfigSchema.safeParse({ defaults: { scope: "local" } }).success
    ).toBe(false)
  })

  test("TOML round-trip preserves values", () => {
    const config = ConfigSchema.parse({
      defaults: { also: ["claude-code"], yes: false, scope: "global" },
      security: { threshold: 7 },
      taps: [{ name: "home", url: "https://example.com/tap.git" }],
    })
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const toml = stringify(config as any)
    const parsed = parse(toml)
    const result = ConfigSchema.parse(parsed)
    expect(result.defaults.also).toEqual(["claude-code"])
    expect(result.defaults.scope).toBe("global")
    expect(result.security.threshold).toBe(7)
    expect(result.taps[0].name).toBe("home")
    expect(result.taps[0].url).toBe("https://example.com/tap.git")
  })
})
