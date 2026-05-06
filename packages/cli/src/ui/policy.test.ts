import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { createTestEnv } from "@skilltap/test-utils";
import { isAgentMode } from "./policy";

describe("isAgentMode flag precedence", () => {
  let env: Awaited<ReturnType<typeof createTestEnv>>;
  let savedArgv: string[];
  let savedEnv: string | undefined;

  beforeEach(async () => {
    env = await createTestEnv();
    savedArgv = process.argv;
    savedEnv = process.env.SKILLTAP_AGENT;
    delete process.env.SKILLTAP_AGENT;
  });

  afterEach(async () => {
    process.argv = savedArgv;
    if (savedEnv === undefined) delete process.env.SKILLTAP_AGENT;
    else process.env.SKILLTAP_AGENT = savedEnv;
    await env.cleanup();
  });

  test("returns true when --agent flag is present in argv", async () => {
    process.argv = ["bun", "skilltap", "disable", "foo", "--agent"];
    expect(await isAgentMode()).toBe(true);
  });

  test("returns true for --agent=true", async () => {
    process.argv = ["bun", "skilltap", "disable", "foo", "--agent=true"];
    expect(await isAgentMode()).toBe(true);
  });

  test("returns true for --agent=1", async () => {
    process.argv = ["bun", "skilltap", "disable", "foo", "--agent=1"];
    expect(await isAgentMode()).toBe(true);
  });

  test("returns true when SKILLTAP_AGENT=1 even without flag", async () => {
    process.argv = ["bun", "skilltap", "disable", "foo"];
    process.env.SKILLTAP_AGENT = "1";
    expect(await isAgentMode()).toBe(true);
  });

  test("returns false when neither flag, env, nor config is set", async () => {
    process.argv = ["bun", "skilltap", "disable", "foo"];
    expect(await isAgentMode()).toBe(false);
  });
});
