import { defineCommand } from "citty";
import { mcpRemoveCommand } from "./mcp";
import { pluginRemoveCommand } from "./plugin";
import { skillRemoveCommand } from "./skill";

export const removeCommand = defineCommand({
  meta: {
    name: "remove",
    description: "Remove a skill, plugin, or MCP server",
  },
  subCommands: {
    skill: skillRemoveCommand,
    plugin: pluginRemoveCommand,
    mcp: mcpRemoveCommand,
  },
});
