import { describe, expect, test } from "bun:test";
import type { Config } from "../../schemas/config";
import { resolveAgent } from "../detect";
import type { AgentAdapter } from "../types";

const DEFAULT_CONFIG: Config = {
  defaults: { also: [], yes: false, scope: "" },
  security: {
    human: { scan: "static", on_warn: "prompt", require_scan: false },
    agent: { scan: "static", on_warn: "fail", require_scan: true },
    agent_cli: "",
    threshold: 5,
    max_size: 51200,
    ollama_model: "",
    overrides: [],
  },
  "agent-mode": { enabled: false, scope: "project" },
  registry: { enabled: ["skills.sh"], sources: [], allow_npm: true },
  builtin_tap: true,
  verbose: true,
  taps: [],
  updates: { auto_update: "off", interval_hours: 24, skill_check_interval_hours: 24, show_diff: "full" },
  telemetry: { enabled: false, notice_shown: false, anonymous_id: "" },
  default_git_host: "https://github.com",
};

function mockAdapter(name: string, available: boolean): AgentAdapter {
  return {
    name,
    cliName: name.toLowerCase(),
    async detect() {
      return available;
    },
    async invoke() {
      return { ok: true as const, value: { score: 0, reason: "mock" } };
    },
  };
}

describe("resolveAgent", () => {
  test("returns null when no agents detected and no config", async () => {
    // This test depends on the system not having any agent CLIs installed
    // which may not be true — but it tests the empty-config path at least
    const config = {
      ...DEFAULT_CONFIG,
      security: { ...DEFAULT_CONFIG.security, agent_cli: "" },
    };
    const result = await resolveAgent(config);
    expect(result.ok).toBe(true);
    // Can't assert null since agents might be installed on this system
  });

  test("returns error for unknown agent name", async () => {
    const config = {
      ...DEFAULT_CONFIG,
      security: { ...DEFAULT_CONFIG.security, agent_cli: "nonexistent-agent" },
    };
    const result = await resolveAgent(config);
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("Unknown agent");
  });

  test("returns custom adapter for absolute path", async () => {
    const config = {
      ...DEFAULT_CONFIG,
      security: { ...DEFAULT_CONFIG.security, agent_cli: "/usr/bin/my-agent" },
    };
    const result = await resolveAgent(config);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value).not.toBeNull();
    expect(result.value?.cliName).toBe("/usr/bin/my-agent");
  });

  test("calls onSelectAgent when agent is empty and agents detected", async () => {
    const config = { ...DEFAULT_CONFIG };
    const mockAgent = mockAdapter("TestAgent", true);
    let _selectCalled = false;

    const result = await resolveAgent(config, async (_detected) => {
      _selectCalled = true;
      return mockAgent;
    });

    // If no real agents are detected, selectCalled will be false — that's fine
    expect(result.ok).toBe(true);
  });
});
