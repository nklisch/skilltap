import { version } from "../package.json";
export const VERSION: string = version;
export * from "./debug";
export * from "./doctor";

export * from "./adapters";
export * from "./agents";
export * from "./config";
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
