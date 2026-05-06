export {
  type ApplyItemResult,
  type ApplyStatus,
  applySync,
  type SyncApplyOptions,
  type SyncApplyResult,
} from "./apply";
export { detectDrift } from "./drift";
export { planSync } from "./plan";
export type {
  DriftItem,
  DriftKind,
  DriftReport,
  DriftTarget,
  SyncPlan,
} from "./types";
