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
