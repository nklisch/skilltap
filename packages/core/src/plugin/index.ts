export { parseAgentDefinitions } from "./agents";
export {
  type ApplyCaptureOptions,
  type ApplyCaptureResult,
  applyCapture,
  buildCrossSourceHint,
  type CaptureBucket,
  type CaptureCandidate,
  type CaptureMatches,
  detectCaptureMatches,
  mergeBuckets,
} from "./capture";
export {
  findComponentInPlugin,
  type ParsedComponentRef,
  parseComponentRef,
} from "./component-ref";
export { type DetectOptions, detectPlugin, listPluginOptions } from "./detect";
export {
  installPlugin,
  type PluginInstallOptions,
  type PluginInstallResult,
} from "./install";
export {
  type RemovePluginOptions,
  removeInstalledPlugin,
  type ToggleComponentOptions,
  type ToggleResult,
  toggleInstalledComponent,
} from "./lifecycle";
export { parseMcpJson, parseMcpObject } from "./mcp";
export {
  type InjectOptions,
  injectMcpServers,
  isNamespacedKey,
  listMcpServers,
  MCP_AGENT_CONFIGS,
  type McpVarContext,
  mcpConfigPath,
  namespaceMcpServer,
  parseNamespacedKey,
  type RemoveOptions,
  removeMcpServers,
  substituteMcpVars,
} from "./mcp-inject";
export { parseClaudePlugin } from "./parse-claude";
export { parseCodexPlugin } from "./parse-codex";
export {
  addPlugin,
  findPlugin,
  loadPlugins,
  manifestToRecord,
  mcpServerToStored,
  type PluginInstallMeta,
  removePlugin,
  savePlugins,
  toggleComponent,
} from "./state";
