import { describe, expect, test } from "bun:test"
import { VERSION } from "./index"

describe("@skilltap/core", () => {
  test("exports VERSION", () => {
    expect(VERSION).toBe("0.1.0")
  })
})
