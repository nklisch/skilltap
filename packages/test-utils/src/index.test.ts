import { describe, expect, test } from "bun:test"
import { TEST_UTILS_VERSION } from "./index"

describe("@skilltap/test-utils", () => {
  test("exports TEST_UTILS_VERSION", () => {
    expect(TEST_UTILS_VERSION).toBe("0.1.0")
  })
})
