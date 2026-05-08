import { defineCommand } from "citty";
import { mcpCommand } from "./mcp";
import { pluginCommand } from "./plugin";
import { skillCommand } from "./skill";

export const installCommand = defineCommand({
  meta: {
    name: "install",
    description: "Install a skill, plugin, or MCP server. Type is required.",
  },
  subCommands: {
    skill: skillCommand,
    plugin: pluginCommand,
    mcp: mcpCommand,
  },
});
