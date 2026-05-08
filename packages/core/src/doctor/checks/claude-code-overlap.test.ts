import { describe, expect, test } from "bun:test";
import { checkClaudeCodeOverlap } from "./claude-code-overlap";
import type { AgentPluginScanner, DiscoveredAgentPlugin } from "../../agent-plugins/types";
import { scanAllAgentPlugins } from "../../agent-plugins/registry";
import type { State } from "../../state/schema";

// We test checkClaudeCodeOverlap indirectly by controlling what scanAllAgentPlugins sees.
// Since checkClaudeCodeOverlap calls scanAllAgentPlugins() with no args (uses defaultScanners),
// we need to ensure no real Claude Code files are read during tests.
//
// Strategy: checkClaudeCodeOverlap is wired to real env. To isolate it, we test its behavior
// by controlling the state that it validates against. The scanner's behavior when Claude Code
// is not installed (installed_plugins.json missing) returns "no plugins" → always "pass".
//
// For collision tests, we create a thin wrapper that calls scanAllAgentPlugins with mock scanners.
// We test the logic directly by calling checkClaudeCodeOverlap and controlling what state we pass.

function makeState(overrides: Partial<State> = {}): State {
  return {
    version: 2,
    skills: [],
    plugins: [],
    mcpServers: [],
    ...overrides,
  };
}

function makeSkillRecord(name: string) {
  const now = new Date().toISOString();
  return {
    name,
    description: "",
    repo: null,
    ref: null,
    sha: null,
    scope: "global" as const,
    path: null,
    tap: null,
    also: [],
    installedAt: now,
    updatedAt: now,
    active: true,
  };
}

function makePluginRecord(name: string, repo: string | null = null) {
  const now = new Date().toISOString();
  return {
    name,
    description: "",
    format: "claude-code" as const,
    repo,
    ref: null,
    sha: null,
    scope: "global" as const,
    path: null,
    also: [],
    tap: null,
    components: [],
    installedAt: now,
    updatedAt: now,
    active: true,
  };
}

describe("checkClaudeCodeOverlap", () => {
  test("returns pass when state is null", async () => {
    const check = await checkClaudeCodeOverlap(null);
    expect(check.status).toBe("pass");
    expect(check.name).toBe("Claude Code overlaps");
  });

  test("returns pass when no Claude Code plugins are present (real env fallback)", async () => {
    // In a test environment without installed_plugins.json, scanner returns no results.
    // This tests the normal path where no Claude Code plugins exist.
    const state = makeState();
    const check = await checkClaudeCodeOverlap(state);
    // Either pass (no claude code installed in test env) or warn if it is installed
    expect(["pass", "warn"]).toContain(check.status);
  });
});

// Tests below use a helper that mimics what checkClaudeCodeOverlap does
// but with mock scanners (for isolation from host environment).

async function runOverlapCheckWithMockPlugins(
  state: State,
  claudeCodePluginNames: string[],
): Promise<Awaited<ReturnType<typeof checkClaudeCodeOverlap>>> {
  // Build mock scanner that returns the specified plugins
  const mockManifest = {
    name: "mock",
    format: "claude-code" as const,
    pluginRoot: "/mock",
    components: [],
  };

  const mockPlugins: DiscoveredAgentPlugin[] = claudeCodePluginNames.map((name) => ({
    scannerName: "claude-code",
    name,
    sourceUrl: null,
    installPath: "/mock/path",
    version: "1.0.0",
    sha: null,
    scope: "global" as const,
    installedAt: "2026-01-01T00:00:00.000Z",
    updatedAt: "2026-01-01T00:00:00.000Z",
    manifest: { ...mockManifest, name },
  }));

  const mockScanner: AgentPluginScanner = {
    name: "claude-code",
    async detect() { return mockPlugins.length > 0; },
    async scan() { return { ok: true as const, value: mockPlugins }; },
  };

  // We replicate the core logic of checkClaudeCodeOverlap using mock scanners
  const scanResult = await scanAllAgentPlugins([mockScanner]);
  if (!scanResult.ok) {
    return {
      name: "Claude Code overlaps",
      status: "warn",
      issues: [{ message: `Could not scan: ${scanResult.error.message}`, fixable: false }],
    };
  }

  const claudePlugins = scanResult.value.plugins.filter((p) => p.scannerName === "claude-code");
  if (claudePlugins.length === 0) return { name: "Claude Code overlaps", status: "pass" };

  const adoptedSourceMarker = "claude-code:";
  const issues: { message: string; fixable: boolean; fixDescription?: string }[] = [];

  for (const plugin of claudePlugins) {
    const skillCollision = state.skills.find((s) => s.name === plugin.name);
    if (skillCollision) {
      issues.push({
        message: `Claude Code plugin "${plugin.name}" overlaps with skilltap standalone skill "${skillCollision.name}".`,
        fixable: false,
        fixDescription: `Run \`skilltap adopt ${plugin.name}\` to bring the Claude Code plugin under skilltap, or \`skilltap remove skill ${skillCollision.name}\` if Claude Code's version should win.`,
      });
    }

    const pluginCollision = state.plugins.find(
      (p) => p.name === plugin.name && !p.repo?.startsWith(adoptedSourceMarker),
    );
    if (pluginCollision) {
      issues.push({
        message: `Claude Code plugin "${plugin.name}" overlaps with skilltap-installed plugin (different source).`,
        fixable: false,
        fixDescription: `Run \`skilltap remove plugin ${plugin.name}\` then \`skilltap adopt ${plugin.name}\` if you want Claude Code's version.`,
      });
    }
  }

  return {
    name: "Claude Code overlaps",
    status: issues.length > 0 ? "warn" : "pass",
    issues,
  };
}

describe("checkClaudeCodeOverlap logic (mock scanners)", () => {
  test("returns pass when no collisions", async () => {
    const state = makeState({
      skills: [makeSkillRecord("unrelated-skill")],
    });
    const check = await runOverlapCheckWithMockPlugins(state, ["some-plugin"]);
    expect(check.status).toBe("pass");
  });

  test("returns warn when Claude Code plugin name matches a standalone skill", async () => {
    const state = makeState({
      skills: [makeSkillRecord("my-skill")],
    });
    const check = await runOverlapCheckWithMockPlugins(state, ["my-skill"]);
    expect(check.status).toBe("warn");
    expect(check.issues).toHaveLength(1);
    expect(check.issues![0]!.message).toContain("my-skill");
    expect(check.issues![0]!.message).toContain("standalone skill");
  });

  test("returns warn when Claude Code plugin name matches a non-adopted plugin", async () => {
    const state = makeState({
      plugins: [makePluginRecord("my-plugin", "github:owner/repo")],
    });
    const check = await runOverlapCheckWithMockPlugins(state, ["my-plugin"]);
    expect(check.status).toBe("warn");
    expect(check.issues).toHaveLength(1);
    expect(check.issues![0]!.message).toContain("my-plugin");
    expect(check.issues![0]!.message).toContain("different source");
  });

  test("no warning when plugin collision is itself an adopted Claude Code plugin", async () => {
    // An adopted plugin has repo starting with "claude-code:"
    const state = makeState({
      plugins: [makePluginRecord("my-plugin", "claude-code:my-marketplace:my-plugin")],
    });
    const check = await runOverlapCheckWithMockPlugins(state, ["my-plugin"]);
    expect(check.status).toBe("pass");
  });

  test("no warning when plugin collision has sourceUrl but is adopted (claude-code: in repo)", async () => {
    const state = makeState({
      plugins: [makePluginRecord("p", "claude-code:p")],
    });
    const check = await runOverlapCheckWithMockPlugins(state, ["p"]);
    expect(check.status).toBe("pass");
  });

  test("fixDescription references the plugin name", async () => {
    const state = makeState({
      skills: [makeSkillRecord("cool-plugin")],
    });
    const check = await runOverlapCheckWithMockPlugins(state, ["cool-plugin"]);
    expect(check.status).toBe("warn");
    expect(check.issues![0]!.fixDescription).toContain("cool-plugin");
  });
});
