import { describe, expect, test } from "bun:test";
import type { AgentAdapter, Output, Progress } from "@skilltap/core";
import { Glob } from "bun";
import { createInstallCallbacks } from "./install-callbacks";
import { createStepLogger } from "./install-steps";

const fakeProgress: Progress = {
  update: () => {},
  succeed: () => {},
  fail: () => {},
  pause: () => {},
  resume: () => {},
};

const fakeOut: Output = {
  mode: "plain",
  info: () => {},
  warn: () => {},
  error: () => {},
  success: () => {},
  block: () => {},
  json: () => {},
  progress: () => fakeProgress,
  raw: () => {},
};

const fakeAgent: AgentAdapter = {
  name: "test-agent",
  cliName: "test-cli",
  detect: async () => true,
  invoke: async () => ({ ok: true as const, value: { score: 0, reason: "" } }),
};

const baseCtx = {
  out: fakeOut,
  progress: fakeProgress,
  onWarn: "prompt" as const,
  skipScan: false,
  yes: false,
  source: "test-skill",
  steps: createStepLogger(false),
};

describe("createInstallCallbacks", () => {
  test("onWarnings is defined when skipScan is false", () => {
    const { callbacks } = createInstallCallbacks({
      ...baseCtx,
      agent: fakeAgent,
    });
    expect(callbacks.onWarnings).toBeDefined();
  });

  test("onWarnings is undefined when skipScan is true", () => {
    const { callbacks } = createInstallCallbacks({
      ...baseCtx,
      agent: undefined,
      skipScan: true,
    });
    expect(callbacks.onWarnings).toBeUndefined();
  });

  test("onWarnings handles all three kinds (skill-static, plugin-static, skill-semantic)", () => {
    const { callbacks } = createInstallCallbacks({
      ...baseCtx,
      agent: undefined,
    });
    // onWarnings is defined and accepts the unified signature
    expect(callbacks.onWarnings).toBeDefined();
    expect(typeof callbacks.onWarnings).toBe("function");
  });

  test("onConfirmInstall is undefined when yes=true", () => {
    const { callbacks } = createInstallCallbacks({
      ...baseCtx,
      yes: true,
    });
    expect(callbacks.onConfirmInstall).toBeUndefined();
  });

  test("onConfirmInstall is defined when yes=false", () => {
    const { callbacks } = createInstallCallbacks({
      ...baseCtx,
      yes: false,
    });
    expect(callbacks.onConfirmInstall).toBeDefined();
  });
});

describe("semantic scan wiring — source-level regression guard", () => {
  test("no call site hardcodes agent: undefined in createInstallCallbacks", async () => {
    const commandsDir = `${import.meta.dir}/../commands`;
    const glob = new Glob("**/*.ts");
    const violations: string[] = [];

    for await (const path of glob.scan({ cwd: commandsDir, absolute: true })) {
      if (path.endsWith(".test.ts")) continue;
      const content = await Bun.file(path).text();
      if (!content.includes("createInstallCallbacks")) continue;

      // Find createInstallCallbacks({ ... }) blocks and check for agent: undefined
      const pattern =
        /createInstallCallbacks\(\{[^}]*agent:\s*undefined[^}]*\}\)/gs;
      const matches = content.match(pattern);
      if (matches) {
        violations.push(`${path}: hardcodes agent: undefined`);
      }
    }

    expect(violations).toEqual([]);
  });

  test("every installSkill call with skipScan: false passes semantic and threshold", async () => {
    const commandsDir = `${import.meta.dir}/../commands`;
    const glob = new Glob("**/*.ts");
    const violations: string[] = [];

    for await (const path of glob.scan({ cwd: commandsDir, absolute: true })) {
      if (path.endsWith(".test.ts")) continue;
      const content = await Bun.file(path).text();
      if (!content.includes("installSkill(")) continue;

      // Find installSkill calls that have skipScan: false but lack semantic:
      const blocks = content.split(/installSkill\(/);
      for (let i = 1; i < blocks.length; i++) {
        // Grab enough of the call to check options (up to closing paren or 800 chars)
        const callBlock = blocks[i].slice(0, 800);
        if (
          callBlock.includes("skipScan: false") &&
          !callBlock.includes("semantic:")
        ) {
          const line = content
            .slice(0, content.indexOf(blocks[i]))
            .split("\n").length;
          violations.push(
            `${path}:${line} — installSkill with skipScan: false but no semantic option`,
          );
        }
      }
    }

    expect(violations).toEqual([]);
  });

  test("every updateSkill call passes semantic and threshold when agent is in scope", async () => {
    const commandsDir = `${import.meta.dir}/../commands`;
    const glob = new Glob("**/*.ts");
    const violations: string[] = [];

    for await (const path of glob.scan({ cwd: commandsDir, absolute: true })) {
      if (path.endsWith(".test.ts")) continue;
      const content = await Bun.file(path).text();
      if (!content.includes("updateSkill(")) continue;

      // If the file resolves an agent (has resolveAgent or resolveSemanticInteractive),
      // then every updateSkill call should pass semantic:
      const hasAgentResolution =
        content.includes("resolveSemanticInteractive") ||
        content.includes("resolveAgentForAgentMode");
      if (!hasAgentResolution) continue;

      const blocks = content.split(/updateSkill\(/);
      for (let i = 1; i < blocks.length; i++) {
        const callBlock = blocks[i].slice(0, 800);
        if (!callBlock.includes("semantic:")) {
          const line = content
            .slice(0, content.indexOf(blocks[i]))
            .split("\n").length;
          violations.push(
            `${path}:${line} — updateSkill missing semantic option`,
          );
        }
      }
    }

    expect(violations).toEqual([]);
  });
});
