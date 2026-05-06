export type {
  DriftItem,
  DriftKind,
  DriftReport,
  DriftTarget,
  SyncPlan,
} from "./types";
export { detectDrift } from "./drift";
export { planSync } from "./plan";
export {
  applySync,
  type ApplyStatus,
  type ApplyItemResult,
  type SyncApplyOptions,
  type SyncApplyResult,
} from "./apply";
