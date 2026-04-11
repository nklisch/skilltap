export { detectPlugin } from "./detect";
export { parseClaudePlugin } from "./parse-claude";
export { parseCodexPlugin } from "./parse-codex";
export { parseMcpJson, parseMcpObject } from "./mcp";
export { parseAgentDefinitions } from "./agents";
export {
  loadPlugins,
  savePlugins,
  addPlugin,
  removePlugin,
  toggleComponent,
  findPlugin,
  manifestToRecord,
  type PluginInstallMeta,
} from "./state";
export { installPlugin, type PluginInstallOptions, type PluginInstallResult } from "./install";
export {
  removeInstalledPlugin,
  toggleInstalledComponent,
  type RemovePluginOptions,
  type ToggleComponentOptions,
  type ToggleResult,
} from "./lifecycle";
export {
  MCP_AGENT_CONFIGS,
  namespaceMcpServer,
  isNamespacedKey,
  parseNamespacedKey,
  substituteMcpVars,
  mcpConfigPath,
  injectMcpServers,
  removeMcpServers,
  listMcpServers,
  type InjectOptions,
  type RemoveOptions,
  type McpVarContext,
} from "./mcp-inject";
