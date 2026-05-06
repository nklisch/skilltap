import { version } from "../package.json";
export const VERSION: string = version;
export * from "./debug";
export * from "./doctor";

export * from "./adapters";
export * from "./agents";
export * from "./config";
export * from "./json-state";
export * from "./config-keys";
export * from "./fs";
export * from "./git";
export * from "./install";
export * from "./link";
export * from "./npm-registry";
export * from "./skills-registry";
export * from "./paths";
export * from "./remove";
export * from "./discover";
export * from "./disable";
export * from "./adopt";
export * from "./move";
export * from "./update";
export * from "./scanner";
export * from "./schemas";
export * from "./plugin";
export * from "./security";
export * from "./symlink";
// Export registry module — exclude names that conflict with ./schemas and ./skills-registry
export type {
  RegistryDetailResponse,
  RegistryListResponse,
} from "./registry/types";
export {
  RegistryDetailResponseSchema,
  RegistryListResponseSchema,
  RegistrySkillSchema,
  RegistryTrustSchema,
} from "./registry/types";
export type { RegistryAuth, FetchSkillListResult } from "./registry/client";
export { detectTapType, fetchSkillList, fetchSkillDetail } from "./registry/client";
export * from "./orphan";
export * from "./taps";
export * from "./templates";
export * from "./trust";
export * from "./policy";
export * from "./validate";
export * from "./self-update";
export * from "./skill-check";
export * from "./shell";
export * from "./types";

// v2.0 additions (Phase 26+) — additive, no v1.0 paths use these yet.
export * from "./manifest";
export * from "./plugin-v2";
export {
  type ConfigV2,
  ConfigV2Schema,
  type ConfigV2Defaults,
  ConfigV2DefaultsSchema,
  type SecurityConfigV2,
  SecurityConfigV2Schema,
  type AgentConfig,
  AgentConfigSchema,
  type ConfigV2TapEntry,
  ConfigV2TapEntrySchema,
  SECURITY_SCAN_V2,
  SECURITY_ON_WARN_V2,
  SCOPE_V2,
} from "./schemas/config-v2";
export * from "./state";
export * from "./migrate";
export * from "./sync";
export * from "./policy-v2";
