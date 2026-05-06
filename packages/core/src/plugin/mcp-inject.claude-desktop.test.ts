import { describe, expect, test } from "bun:test";
import { MCP_AGENT_CONFIGS, mcpConfigPath } from "./mcp-inject";

describe("Claude Desktop in MCP_AGENT_CONFIGS", () => {
  test("registers the entry on macOS or Linux", () => {
    if (process.platform === "darwin") {
      expect(MCP_AGENT_CONFIGS["claude-desktop"]).toBe(
        "Library/Application Support/Claude/claude_desktop_config.json",
      );
    } else if (process.platform === "linux") {
      expect(MCP_AGENT_CONFIGS["claude-desktop"]).toBe(
        ".config/Claude/claude_desktop_config.json",
      );
    } else {
      // Windows or other — entry not registered
      expect(MCP_AGENT_CONFIGS["claude-desktop"]).toBeUndefined();
    }
  });

  test("existing 5 agents remain registered", () => {
    expect(MCP_AGENT_CONFIGS["claude-code"]).toBe(".claude/settings.json");
    expect(MCP_AGENT_CONFIGS["cursor"]).toBe(".cursor/mcp.json");
    expect(MCP_AGENT_CONFIGS["codex"]).toBe(".codex/mcp.json");
    expect(MCP_AGENT_CONFIGS["gemini"]).toBe(".gemini/settings.json");
    expect(MCP_AGENT_CONFIGS["windsurf"]).toBe(".windsurf/mcp.json");
  });

  test("mcpConfigPath resolves Claude Desktop on supported platforms", () => {
    if (process.platform !== "darwin" && process.platform !== "linux") return;
    const path = mcpConfigPath("claude-desktop", "global");
    expect(path).not.toBeNull();
    expect(path).toContain("Claude");
    expect(path).toContain("claude_desktop_config.json");
  });

  test("mcpConfigPath returns null for unknown agent", () => {
    expect(mcpConfigPath("not-an-agent", "global")).toBeNull();
  });
});
