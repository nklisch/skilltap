import { describe, expect, test } from "bun:test"
import {
  ok,
  err,
  SkilltapError,
  UserError,
  GitError,
  ScanError,
  NetworkError,
  EXIT_SUCCESS,
  EXIT_ERROR,
  EXIT_CANCELLED,
} from "./types"

describe("ok", () => {
  test("returns ok result with value", () => {
    const result = ok(42)
    expect(result.ok).toBe(true)
    if (result.ok) expect(result.value).toBe(42)
  })

  test("works with undefined (void)", () => {
    const result = ok(undefined)
    expect(result.ok).toBe(true)
  })

  test("works with objects", () => {
    const result = ok({ name: "test" })
    expect(result.ok).toBe(true)
    if (result.ok) expect(result.value.name).toBe("test")
  })
})

describe("err", () => {
  test("returns error result", () => {
    const error = new SkilltapError("test error")
    const result = err(error)
    expect(result.ok).toBe(false)
    if (!result.ok) expect(result.error).toBe(error)
  })

  test("works with any error type", () => {
    const result = err(new Error("plain error"))
    expect(result.ok).toBe(false)
  })
})

describe("SkilltapError", () => {
  test("has message, name, and optional hint", () => {
    const error = new SkilltapError("something failed", "try this instead")
    expect(error.message).toBe("something failed")
    expect(error.hint).toBe("try this instead")
    expect(error.name).toBe("SkilltapError")
  })

  test("hint is optional", () => {
    const error = new SkilltapError("message")
    expect(error.hint).toBeUndefined()
  })

  test("is an instance of Error", () => {
    expect(new SkilltapError("x")).toBeInstanceOf(Error)
  })
})

describe("error hierarchy", () => {
  test("UserError extends SkilltapError", () => {
    const e = new UserError("bad input", "fix it like this")
    expect(e).toBeInstanceOf(SkilltapError)
    expect(e).toBeInstanceOf(Error)
    expect(e.name).toBe("UserError")
    expect(e.hint).toBe("fix it like this")
  })

  test("GitError extends SkilltapError", () => {
    const e = new GitError("clone failed")
    expect(e).toBeInstanceOf(SkilltapError)
    expect(e).toBeInstanceOf(Error)
    expect(e.name).toBe("GitError")
  })

  test("ScanError extends SkilltapError", () => {
    const e = new ScanError("scan failed", "check agent")
    expect(e).toBeInstanceOf(SkilltapError)
    expect(e).toBeInstanceOf(Error)
    expect(e.name).toBe("ScanError")
    expect(e.hint).toBe("check agent")
  })

  test("NetworkError extends SkilltapError", () => {
    const e = new NetworkError("unreachable")
    expect(e).toBeInstanceOf(SkilltapError)
    expect(e).toBeInstanceOf(Error)
    expect(e.name).toBe("NetworkError")
  })

  test("instanceof distinguishes subclasses", () => {
    const user = new UserError("x")
    const git = new GitError("x")
    expect(user).toBeInstanceOf(UserError)
    expect(user).not.toBeInstanceOf(GitError)
    expect(git).toBeInstanceOf(GitError)
    expect(git).not.toBeInstanceOf(UserError)
  })
})

describe("exit codes", () => {
  test("EXIT_SUCCESS is 0", () => {
    expect(EXIT_SUCCESS).toBe(0)
  })

  test("EXIT_ERROR is 1", () => {
    expect(EXIT_ERROR).toBe(1)
  })

  test("EXIT_CANCELLED is 2", () => {
    expect(EXIT_CANCELLED).toBe(2)
  })
})
