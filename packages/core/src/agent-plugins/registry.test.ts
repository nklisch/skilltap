import { describe, expect, test } from "bun:test";
import { UserError } from "../types";
import type { AgentPluginScanner, DiscoveredAgentPlugin } from "./types";
import { scanAllAgentPlugins } from "./registry";

function makeMockPlugin(name: string): DiscoveredAgentPlugin {
  return {
    scannerName: "test",
    name,
    sourceUrl: null,
    installPath: "/some/path",
    version: "1.0.0",
    sha: null,
    scope: "global",
    installedAt: "2026-01-01T00:00:00.000Z",
    updatedAt: "2026-01-01T00:00:00.000Z",
    manifest: {
      name,
      format: "claude-code",
      pluginRoot: "/some/path",
      components: [],
    },
  };
}

function makeScanner(
  name: string,
  detectResult: boolean,
  plugins: DiscoveredAgentPlugin[],
): AgentPluginScanner {
  return {
    name,
    async detect() {
      return detectResult;
    },
    async scan() {
      return { ok: true as const, value: plugins };
    },
  };
}

function makeFailingScanner(name: string): AgentPluginScanner {
  return {
    name,
    async detect() {
      return true;
    },
    async scan() {
      return {
        ok: false as const,
        error: new UserError(`Scanner ${name} failed`),
      };
    },
  };
}

describe("scanAllAgentPlugins", () => {
  test("returns empty result when no scanners provided", async () => {
    const result = await scanAllAgentPlugins([]);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.plugins).toHaveLength(0);
    expect(result.value.errors).toHaveLength(0);
  });

  test("skips scanners where detect() returns false", async () => {
    let scanCalled = false;
    const scanner: AgentPluginScanner = {
      name: "skipped",
      async detect() {
        return false;
      },
      async scan() {
        scanCalled = true;
        return { ok: true as const, value: [makeMockPlugin("p")] };
      },
    };
    const result = await scanAllAgentPlugins([scanner]);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(scanCalled).toBe(false);
    expect(result.value.plugins).toHaveLength(0);
  });

  test("aggregates plugins from multiple scanners", async () => {
    const scannerA = makeScanner("a", true, [makeMockPlugin("pa1"), makeMockPlugin("pa2")]);
    const scannerB = makeScanner("b", true, [makeMockPlugin("pb1")]);
    const result = await scanAllAgentPlugins([scannerA, scannerB]);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.plugins).toHaveLength(3);
  });

  test("failing scanner is recorded in errors, not fatal", async () => {
    const good = makeScanner("good", true, [makeMockPlugin("p")]);
    const bad = makeFailingScanner("bad");
    const result = await scanAllAgentPlugins([good, bad]);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.plugins).toHaveLength(1);
    expect(result.value.errors).toHaveLength(1);
    expect(result.value.errors[0]!.scanner).toBe("bad");
  });

  test("detect-false scanner does not add to errors", async () => {
    const skipped = makeScanner("skipped", false, []);
    const result = await scanAllAgentPlugins([skipped]);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.errors).toHaveLength(0);
  });
});
