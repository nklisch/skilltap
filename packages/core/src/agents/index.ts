export {
  claudeAdapter,
  codexAdapter,
  geminiAdapter,
  opencodeAdapter,
} from "./adapters";
export { createCustomAdapter } from "./custom";
export { detectAgents, resolveAgent } from "./detect";
export { extractAgentResponse } from "./extract";
export { createCliAdapter } from "./factory";
export { createOllamaAdapter } from "./ollama";
export type { AgentAdapter } from "./types";
