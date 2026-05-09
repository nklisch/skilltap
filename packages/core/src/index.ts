import { version } from "../package.json";
export const VERSION: string = version;

export * from "./adapters";
export * from "./agent-plugins";
export * from "./adopt";
export * from "./agents";
export * from "./config";
export * from "./config-keys";
export * from "./debug";
export * from "./dirs";
export * from "./disable";
export * from "./discover";
export * from "./doctor";
export * from "./fs";
export * from "./git";
export * from "./install";
export * from "./json-state";
// v2.0 additions (Phase 26+) — additive, no v1.0 paths use these yet.
export * from "./manifest";
export {
  installMcp,
  type McpInstallOptions,
  type McpInstallResult,
  type McpRemoveOptions,
  type McpRemoveResult,
  type ParsedMcpRef,
  parseMcpRef,
  removeMcp,
} from "./mcp-install";
export * from "./migrate";
export * from "./move";
export * from "./npm-registry";
export * from "./orphan";
export type { Output, OutputMode, OutputOptions, Progress } from "./output";
export { pickMode } from "./output";
export * from "./paths";
export * from "./plugin";
export * from "./plugin-v2";
export * from "./policy";
export * from "./remove";
export * from "./scanner";
export * from "./schemas";
export * from "./security";
export * from "./self-update";
export * from "./shell";
export * from "./skill-check";
export * from "./skills-registry";
export * from "./state";
export * from "./status";
export * from "./symlink";
export * from "./sync";
export * from "./taps";
export * from "./templates";
export * from "./trust";
export { type TryOptions, type TryReport, tryPreview } from "./try";
export * from "./types";
export * from "./update";
